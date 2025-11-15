//! Filesystem-based media storage implementation.
//!
//! This backend stores media files in a content-addressable filesystem structure,
//! organized by media type and content hash for automatic deduplication.

use crate::{
    BoticelliResult, MediaMetadata, MediaReference, MediaStorage, MediaType, StorageError,
    StorageErrorKind,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

/// Filesystem storage backend.
///
/// Stores media files in a content-addressable structure:
/// `{base_path}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
///
/// # Example Structure
///
/// ```text
/// /var/boticelli/media/
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
    pub fn new(base_path: impl Into<PathBuf>) -> BoticelliResult<Self> {
        let base_path = base_path.into();

        std::fs::create_dir_all(&base_path).map_err(|e| {
            StorageError::new(StorageErrorKind::Io(format!(
                "Failed to create base directory {}: {}",
                base_path.display(),
                e
            )))
        })?;

        Ok(Self { base_path })
    }

    /// Compute SHA-256 hash of data.
    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Get the filesystem path for a given hash and media type.
    ///
    /// Structure: `{base}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
    fn get_path(&self, hash: &str, media_type: MediaType) -> PathBuf {
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
    fn verify_hash(data: &[u8], expected_hash: &str) -> BoticelliResult<()> {
        let actual_hash = Self::compute_hash(data);
        if actual_hash != expected_hash {
            return Err(StorageError::new(StorageErrorKind::HashMismatch {
                expected: expected_hash.to_string(),
                actual: actual_hash,
            })
            .into());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl MediaStorage for FileSystemStorage {
    async fn store(
        &self,
        data: &[u8],
        metadata: &MediaMetadata,
    ) -> BoticelliResult<MediaReference> {
        let hash = Self::compute_hash(data);
        let path = self.get_path(&hash, metadata.media_type);

        // If file already exists, just return reference (deduplication)
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tracing::debug!(
                hash = %hash,
                path = %path.display(),
                "Media already exists, returning existing reference"
            );

            return Ok(MediaReference {
                id: Uuid::new_v4(),
                content_hash: hash,
                storage_backend: "filesystem".to_string(),
                storage_path: path.to_string_lossy().to_string(),
                size_bytes: data.len() as i64,
                media_type: metadata.media_type,
                mime_type: metadata.mime_type.clone(),
            });
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::new(StorageErrorKind::Io(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                )))
            })?;
        }

        // Write to temp file first, then rename for atomicity
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, data).await.map_err(|e| {
            StorageError::new(StorageErrorKind::Io(format!(
                "Failed to write temp file {}: {}",
                temp_path.display(),
                e
            )))
        })?;

        tokio::fs::rename(&temp_path, &path).await.map_err(|e| {
            StorageError::new(StorageErrorKind::Io(format!(
                "Failed to rename {} to {}: {}",
                temp_path.display(),
                path.display(),
                e
            )))
        })?;

        tracing::info!(
            hash = %hash,
            path = %path.display(),
            size = data.len(),
            media_type = %metadata.media_type,
            "Stored media file"
        );

        Ok(MediaReference {
            id: Uuid::new_v4(),
            content_hash: hash,
            storage_backend: "filesystem".to_string(),
            storage_path: path.to_string_lossy().to_string(),
            size_bytes: data.len() as i64,
            media_type: metadata.media_type,
            mime_type: metadata.mime_type.clone(),
        })
    }

    async fn retrieve(&self, reference: &MediaReference) -> BoticelliResult<Vec<u8>> {
        let path = Path::new(&reference.storage_path);

        let data = tokio::fs::read(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::new(StorageErrorKind::NotFound(
                    reference.storage_path.clone(),
                ))
            } else {
                StorageError::new(StorageErrorKind::Io(format!(
                    "Failed to read {}: {}",
                    path.display(),
                    e
                )))
            }
        })?;

        // Verify content hash
        Self::verify_hash(&data, &reference.content_hash)?;

        tracing::debug!(
            hash = %reference.content_hash,
            path = %path.display(),
            size = data.len(),
            "Retrieved media file"
        );

        Ok(data)
    }

    async fn get_url(
        &self,
        _reference: &MediaReference,
        _expires_in: Duration,
    ) -> BoticelliResult<Option<String>> {
        // Filesystem storage doesn't support direct URLs
        Ok(None)
    }

    async fn delete(&self, reference: &MediaReference) -> BoticelliResult<()> {
        let path = Path::new(&reference.storage_path);

        tokio::fs::remove_file(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::new(StorageErrorKind::NotFound(
                    reference.storage_path.clone(),
                ))
            } else {
                StorageError::new(StorageErrorKind::Io(format!(
                    "Failed to delete {}: {}",
                    path.display(),
                    e
                )))
            }
        })?;

        tracing::info!(
            hash = %reference.content_hash,
            path = %path.display(),
            "Deleted media file"
        );

        Ok(())
    }

    async fn exists(&self, reference: &MediaReference) -> BoticelliResult<bool> {
        let path = Path::new(&reference.storage_path);
        Ok(tokio::fs::try_exists(path).await.unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"Hello, world!";
        let metadata = MediaMetadata {
            media_type: MediaType::Image,
            mime_type: "image/png".to_string(),
            filename: Some("test.png".to_string()),
            width: Some(800),
            height: Some(600),
            duration_seconds: None,
        };

        // Store the data
        let reference = storage.store(data, &metadata).await.unwrap();

        assert_eq!(reference.storage_backend, "filesystem");
        assert_eq!(reference.media_type, MediaType::Image);
        assert_eq!(reference.mime_type, "image/png");
        assert_eq!(reference.size_bytes, data.len() as i64);
        assert!(!reference.content_hash.is_empty());

        // Retrieve the data
        let retrieved = storage.retrieve(&reference).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"Duplicate content";
        let metadata = MediaMetadata {
            media_type: MediaType::Audio,
            mime_type: "audio/mp3".to_string(),
            filename: None,
            width: None,
            height: None,
            duration_seconds: Some(120.5),
        };

        // Store same data twice
        let ref1 = storage.store(data, &metadata).await.unwrap();
        let ref2 = storage.store(data, &metadata).await.unwrap();

        // Should have same hash and path
        assert_eq!(ref1.content_hash, ref2.content_hash);
        assert_eq!(ref1.storage_path, ref2.storage_path);

        // Should only exist once on disk
        let path = Path::new(&ref1.storage_path);
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_hash_verification() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"Original data";
        let metadata = MediaMetadata {
            media_type: MediaType::Video,
            mime_type: "video/mp4".to_string(),
            filename: None,
            width: None,
            height: None,
            duration_seconds: None,
        };

        let reference = storage.store(data, &metadata).await.unwrap();

        // Corrupt the file
        let path = Path::new(&reference.storage_path);
        tokio::fs::write(path, b"Corrupted data").await.unwrap();

        // Should detect corruption on retrieve
        let result = storage.retrieve(&reference).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            crate::BoticelliErrorKind::Storage(_)
        ));
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"Delete me";
        let metadata = MediaMetadata {
            media_type: MediaType::Image,
            mime_type: "image/jpeg".to_string(),
            filename: None,
            width: None,
            height: None,
            duration_seconds: None,
        };

        let reference = storage.store(data, &metadata).await.unwrap();
        assert!(storage.exists(&reference).await.unwrap());

        storage.delete(&reference).await.unwrap();
        assert!(!storage.exists(&reference).await.unwrap());
    }

    #[tokio::test]
    async fn test_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let fake_reference = MediaReference {
            id: Uuid::new_v4(),
            content_hash: "nonexistent".to_string(),
            storage_backend: "filesystem".to_string(),
            storage_path: temp_dir.path().join("fake.dat").to_string_lossy().to_string(),
            size_bytes: 100,
            media_type: MediaType::Image,
            mime_type: "image/png".to_string(),
        };

        let result = storage.retrieve(&fake_reference).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_content_addressable_structure() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"Test structure";
        let metadata = MediaMetadata {
            media_type: MediaType::Image,
            mime_type: "image/png".to_string(),
            filename: None,
            width: None,
            height: None,
            duration_seconds: None,
        };

        let reference = storage.store(data, &metadata).await.unwrap();
        let path = Path::new(&reference.storage_path);

        // Verify path structure: base/images/XX/YY/hash
        let components: Vec<_> = path.components().collect();
        assert!(components.len() >= 4);

        // Last component should be the full hash
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, reference.content_hash);

        // Should be in images subdirectory
        assert!(reference.storage_path.contains("images"));
    }

    #[tokio::test]
    async fn test_no_direct_urls() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();

        let data = b"No URL support";
        let metadata = MediaMetadata {
            media_type: MediaType::Image,
            mime_type: "image/png".to_string(),
            filename: None,
            width: None,
            height: None,
            duration_seconds: None,
        };

        let reference = storage.store(data, &metadata).await.unwrap();
        let url = storage
            .get_url(&reference, Duration::from_secs(3600))
            .await
            .unwrap();

        assert!(url.is_none());
    }
}
