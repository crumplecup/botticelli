//! Filesystem-based media storage implementation.
//!
//! This backend stores media files in a content-addressable filesystem structure,
//! organized by media type and content hash for automatic deduplication.

use crate::{MediaMetadata, MediaReference, MediaReferenceBuilder, MediaType};
use botticelli_error::{BotticelliError, StorageError, StorageErrorKind};
use botticelli_interface::MediaStorage;
use rmcp::tool;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Filesystem storage backend.
///
/// Stores media files in a content-addressable structure:
/// `{base_path}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
///
/// # Example Structure
///
/// ```text
/// /var/botticelli/media/
/// ├── images/
/// │   ├── ab/
/// │   │   └── cd/
/// │   │       └── abcdef123456...  (PNG file)
/// ├── audio/
/// │   └── 12/
/// │       └── 34/
/// │           └── 123456abcdef...  (MP3 file)
/// └── video/
///     └── ef/
///         └── gh/
///             └── efgh789012...    (MP4 file)
/// ```
///
/// # Features
///
/// - **Content-addressable**: Files stored by SHA-256 hash
/// - **Automatic deduplication**: Same content = same hash = same file
/// - **Atomic writes**: Uses temp file + rename for atomicity
/// - **Organized structure**: Two-level subdirectories prevent directory bloat
#[derive(elicitation::Elicit)]
pub struct FileSystemStorage {
    base_path: PathBuf,
}

impl FileSystemStorage {
    /// Create a new filesystem storage backend.
    ///
    /// Creates the base directory if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for media storage
    ///
    /// # Errors
    ///
    /// Returns error if the directory cannot be created or accessed.
    #[tool]
    #[tracing::instrument(skip(base_path))]
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, BotticelliError> {
        let base_path = base_path.into();

        std::fs::create_dir_all(&base_path).map_err(|e| {
            StorageError::new(StorageErrorKind::DirectoryCreation(format!(
                "{}: {}",
                base_path.display(),
                e
            )))
        })?;

        tracing::info!(path = %base_path.display(), "Created filesystem storage");
        Ok(Self { base_path })
    }

    /// Compute SHA-256 hash of data.
    #[tool]
    #[tracing::instrument(skip(data), fields(size = data.len()))]
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Get the filesystem path for a given hash and media type.
    ///
    /// Structure: `{base}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn get_path(&self, hash: &str, media_type: MediaType) -> PathBuf {
        let type_dir = match media_type {
            MediaType::Image => "images",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
        };

        self.base_path
            .join(type_dir)
            .join(&hash[0..2])
            .join(&hash[2..4])
            .join(hash)
    }

    /// Verify content hash matches expected hash.
    #[tool]
    #[tracing::instrument(skip(data))]
    pub fn verify_hash(data: &[u8], expected_hash: &str) -> Result<(), BotticelliError> {
        let actual_hash = Self::compute_hash(data);
        if actual_hash != expected_hash {
            return Err(StorageError::new(StorageErrorKind::InvalidPath(format!(
                "Hash mismatch: expected {}, got {}",
                expected_hash, actual_hash
            )))
            .into());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl MediaStorage for FileSystemStorage {
    type Error = BotticelliError;
    type Metadata = MediaMetadata;
    type Reference = MediaReference;

    #[tracing::instrument(skip(self, params), fields(size = params.0.data.len()))]
    async fn store(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<
            botticelli_interface::StoreParams<Self::Metadata, Vec<u8>>,
        >,
    ) -> Result<rmcp::Json<botticelli_interface::StoreResult<Self::Reference>>, rmcp::ErrorData> {
        let data = &params.0.data;
        let metadata = &params.0.metadata;

        let hash = Self::compute_hash(data);
        let path = self.get_path(&hash, *metadata.media_type());

        // If file already exists, just return reference (deduplication)
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tracing::debug!(
                hash = %hash,
                path = %path.display(),
                "Media already exists, returning existing reference"
            );

            let reference = MediaReferenceBuilder::default()
                .id(Uuid::new_v4())
                .content_hash(hash)
                .storage_backend("filesystem")
                .storage_path(path.to_string_lossy().to_string())
                .size_bytes(data.len() as i64)
                .media_type(*metadata.media_type())
                .mime_type(metadata.mime_type().clone())
                .build()
                .expect("Valid MediaReference");

            return Ok(rmcp::Json(botticelli_interface::StoreResult { reference }));
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                let err = StorageError::new(StorageErrorKind::DirectoryCreation(format!(
                    "{}: {}",
                    parent.display(),
                    e
                )));
                rmcp::ErrorData::internal_error(err.to_string(), None)
            })?;
        }

        // Write to temp file first, then rename for atomicity
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, data).await.map_err(|e| {
            let err = StorageError::new(StorageErrorKind::FileWrite(format!(
                "{}: {}",
                temp_path.display(),
                e
            )));
            rmcp::ErrorData::internal_error(err.to_string(), None)
        })?;

        tokio::fs::rename(&temp_path, &path).await.map_err(|e| {
            let err = StorageError::new(StorageErrorKind::FileWrite(format!(
                "rename {} to {}: {}",
                temp_path.display(),
                path.display(),
                e
            )));
            rmcp::ErrorData::internal_error(err.to_string(), None)
        })?;

        tracing::info!(
            hash = %hash,
            path = %path.display(),
            size = data.len(),
            media_type = %metadata.media_type(),
            "Stored media file"
        );

        let reference = MediaReferenceBuilder::default()
            .id(Uuid::new_v4())
            .content_hash(hash)
            .storage_backend("filesystem")
            .storage_path(path.to_string_lossy().to_string())
            .size_bytes(data.len() as i64)
            .media_type(*metadata.media_type())
            .mime_type(metadata.mime_type().clone())
            .build()
            .expect("Valid MediaReference");

        Ok(rmcp::Json(botticelli_interface::StoreResult { reference }))
    }

    #[tracing::instrument(skip(self, params), fields(hash = %params.0.reference.content_hash(), path = %params.0.reference.storage_path()))]
    async fn retrieve(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<
            botticelli_interface::RetrieveParams<Self::Reference>,
        >,
    ) -> Result<rmcp::Json<botticelli_interface::RetrieveResult>, rmcp::ErrorData> {
        let reference = &params.0.reference;
        let path = Path::new(reference.storage_path());

        let data = tokio::fs::read(path).await.map_err(|e| {
            let err = if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::new(StorageErrorKind::NotFound(reference.storage_path().clone()))
            } else {
                StorageError::new(StorageErrorKind::FileRead(format!(
                    "{}: {}",
                    path.display(),
                    e
                )))
            };
            rmcp::ErrorData::internal_error(err.to_string(), None)
        })?;

        // Verify content hash
        Self::verify_hash(&data, reference.content_hash()).map_err(|e| {
            rmcp::ErrorData::internal_error(e.to_string(), None)
        })?;

        tracing::debug!(
            hash = %reference.content_hash(),
            path = %path.display(),
            size = data.len(),
            "Retrieved media file"
        );

        Ok(rmcp::Json(botticelli_interface::RetrieveResult { data }))
    }

    #[tracing::instrument(skip(self, _params))]
    async fn get_url(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<
            botticelli_interface::GetUrlParams<Self::Reference>,
        >,
    ) -> Result<rmcp::Json<botticelli_interface::GetUrlResult>, rmcp::ErrorData> {
        // Filesystem storage doesn't support direct URLs
        Ok(rmcp::Json(botticelli_interface::GetUrlResult { url: None }))
    }

    #[tracing::instrument(skip(self, params), fields(hash = %params.0.reference.content_hash(), path = %params.0.reference.storage_path()))]
    async fn delete(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<
            botticelli_interface::DeleteParams<Self::Reference>,
        >,
    ) -> Result<rmcp::Json<botticelli_interface::DeleteResult>, rmcp::ErrorData> {
        let reference = &params.0.reference;
        let path = Path::new(reference.storage_path());

        tokio::fs::remove_file(path).await.map_err(|e| {
            let err = if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::new(StorageErrorKind::NotFound(reference.storage_path().clone()))
            } else {
                StorageError::new(StorageErrorKind::FileWrite(format!(
                    "delete {}: {}",
                    path.display(),
                    e
                )))
            };
            rmcp::ErrorData::internal_error(err.to_string(), None)
        })?;

        tracing::info!(
            hash = %reference.content_hash(),
            path = %path.display(),
            "Deleted media file"
        );

        Ok(rmcp::Json(botticelli_interface::DeleteResult { success: true }))
    }

    #[tracing::instrument(skip(self, params), fields(hash = %params.0.reference.content_hash(), path = %params.0.reference.storage_path()))]
    async fn exists(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<
            botticelli_interface::ExistsParams<Self::Reference>,
        >,
    ) -> Result<rmcp::Json<botticelli_interface::ExistsResult>, rmcp::ErrorData> {
        let reference = &params.0.reference;
        let path = Path::new(reference.storage_path());
        let exists = tokio::fs::try_exists(path).await.unwrap_or(false);
        Ok(rmcp::Json(botticelli_interface::ExistsResult { exists }))
    }
}
