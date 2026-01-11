//! Tests for filesystem storage backend.

mod helpers;

use botticelli_error::{BotticelliErrorKind, IoError};
use botticelli_interface::MediaStorage;
use botticelli_storage::{
    FileSystemStorage, MediaMetadataBuilder, MediaReferenceBuilder, MediaType,
};
use tempfile::TempDir;
use uuid::Uuid;

fn setup_temp_storage() -> anyhow::Result<(TempDir, FileSystemStorage)> {
    let temp_dir = TempDir::new().map_err(IoError::from)?;
    let storage = FileSystemStorage::new(temp_dir.path())?;
    Ok((temp_dir, storage))
}

#[tokio::test]
async fn test_store_and_retrieve() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing store and retrieve operations");
    let (_temp_dir, storage) = setup_temp_storage()?;

    let data = b"Hello, world!";
    debug!(size = data.len(), "Creating test data");

    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .filename(Some("test.png".to_string()))
        .width(Some(800))
        .height(Some(600))
        .build()?;

    debug!("Storing media");
    let reference = storage.store(data, &metadata).await?;

    debug!(
        backend = reference.storage_backend(),
        hash = reference.content_hash(),
        "Validating reference"
    );
    assert_eq!(reference.storage_backend(), "filesystem");
    assert_eq!(*reference.media_type(), MediaType::Image);
    assert_eq!(reference.mime_type(), "image/png");
    assert_eq!(*reference.size_bytes(), data.len() as i64);
    assert!(!reference.content_hash().is_empty());

    debug!("Retrieving media");
    let retrieved = storage.retrieve(&reference).await?;
    assert_eq!(retrieved, data);

    info!("Store and retrieve test passed");
    Ok(())
}

#[tokio::test]
async fn test_deduplication() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing deduplication of stored content");
    let (_temp_dir, storage) = setup_temp_storage()?;

    let data = b"Duplicate content";
    debug!(size = data.len(), "Creating test data");
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Audio)
        .mime_type("audio/mp3")
        .duration_seconds(Some(120.5))
        .build()?;

    // Store same data twice
    debug!("Storing content first time");
    let ref1 = storage.store(data, &metadata).await?;
    debug!(
        hash = ref1.content_hash(),
        path = ref1.storage_path(),
        "First store complete"
    );

    debug!("Storing same content second time");
    let ref2 = storage.store(data, &metadata).await?;
    debug!(
        hash = ref2.content_hash(),
        path = ref2.storage_path(),
        "Second store complete"
    );

    // Should have same hash and path
    debug!("Verifying deduplication");
    assert_eq!(ref1.content_hash(), ref2.content_hash());
    assert_eq!(ref1.storage_path(), ref2.storage_path());

    // Should only exist once on disk
    let path = std::path::Path::new(ref1.storage_path());
    assert!(path.exists());
    debug!("Verified content exists only once on disk");

    info!("Deduplication test passed");
    Ok(())
}

#[tokio::test]
async fn test_hash_verification() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing hash verification on corrupted content");
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Original data";
    debug!(size = data.len(), "Creating test data");
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Video)
        .mime_type("video/mp4")
        .build()?;

    debug!("Storing original content");
    let reference = storage.store(data, &metadata).await?;
    debug!(
        hash = reference.content_hash(),
        path = reference.storage_path(),
        "Content stored"
    );

    // Corrupt the file
    let path = std::path::Path::new(reference.storage_path());
    debug!(path = ?path, "Corrupting stored file");
    tokio::fs::write(path, b"Corrupted data").await?;
    debug!("File corrupted");

    // Should detect corruption on retrieve
    debug!("Attempting to retrieve corrupted content");
    let result = storage.retrieve(&reference).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        BotticelliErrorKind::Storage(_)
    ));
    debug!("Hash verification correctly detected corruption");

    info!("Hash verification test passed");
    Ok(())
}

#[tokio::test]
async fn test_delete() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing deletion of stored content");
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Delete me";
    debug!(size = data.len(), "Creating test data");
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/jpeg")
        .build()?;

    debug!("Storing content");
    let reference = storage.store(data, &metadata).await?;
    debug!(
        hash = reference.content_hash(),
        path = reference.storage_path(),
        "Content stored"
    );

    debug!("Verifying content exists");
    assert!(storage.exists(&reference).await?);
    debug!("Content exists confirmed");

    debug!("Deleting content");
    storage.delete(&reference).await?;
    debug!("Delete operation complete");

    debug!("Verifying content no longer exists");
    assert!(!storage.exists(&reference).await?);
    debug!("Content deletion confirmed");

    info!("Delete test passed");
    Ok(())
}

#[tokio::test]
async fn test_not_found() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing retrieval of non-existent content");
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    debug!("Creating fake reference to non-existent content");
    let fake_reference = MediaReferenceBuilder::default()
        .id(Uuid::new_v4())
        .content_hash("nonexistent")
        .storage_backend("filesystem")
        .storage_path(
            temp_dir
                .path()
                .join("fake.dat")
                .to_string_lossy()
                .to_string(),
        )
        .size_bytes(100)
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;
    debug!(
        path = fake_reference.storage_path(),
        "Fake reference created"
    );

    debug!("Attempting to retrieve non-existent content");
    let result = storage.retrieve(&fake_reference).await;
    assert!(result.is_err());
    debug!("Retrieval correctly failed for non-existent content");

    info!("Not found test passed");
    Ok(())
}

#[tokio::test]
async fn test_content_addressable_structure() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing content-addressable directory structure");
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Test structure";
    debug!(size = data.len(), "Creating test data");
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;

    debug!("Storing content");
    let reference = storage.store(data, &metadata).await?;
    let path = std::path::Path::new(reference.storage_path());
    debug!(path = ?path, "Content stored");

    // Verify path structure: base/images/XX/YY/hash
    debug!("Verifying path structure");
    let components: Vec<_> = path.components().collect();
    assert!(components.len() >= 4);
    debug!(
        component_count = components.len(),
        "Path components verified"
    );

    // Last component should be the full hash
    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    assert_eq!(filename, reference.content_hash());
    debug!(
        filename = filename,
        hash = reference.content_hash(),
        "Filename matches hash"
    );

    // Should be in images subdirectory
    assert!(reference.storage_path().contains("images"));
    debug!("Verified content is in images subdirectory");

    info!("Content-addressable structure test passed");
    Ok(())
}

#[tokio::test]
async fn test_no_direct_urls() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing that filesystem storage does not provide direct URLs");
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"No URL support";
    debug!(size = data.len(), "Creating test data");
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;

    debug!("Storing content");
    let reference = storage.store(data, &metadata).await?;
    debug!(hash = reference.content_hash(), "Content stored");

    debug!("Requesting URL with 1 hour TTL");
    let url = storage
        .get_url(&reference, std::time::Duration::from_secs(3600))
        .await?;
    debug!("URL request completed");

    assert!(url.is_none());
    debug!("Verified filesystem storage returns None for URL");

    info!("No direct URLs test passed");
    Ok(())
}
