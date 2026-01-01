//! Tests for filesystem storage backend.

use botticelli_error::BotticelliErrorKind;
use botticelli_interface::MediaStorage;
use botticelli_storage::{
    FileSystemStorage, MediaMetadataBuilder, MediaReferenceBuilder, MediaType,
};
use tempfile::TempDir;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_store_and_retrieve() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Hello, world!";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .filename(Some("test.png".to_string()))
        .width(Some(800))
        .height(Some(600))
        .build()?;

    // Store the data
    let reference = storage.store(data, &metadata).await?;

    assert_eq!(reference.storage_backend(), "filesystem");
    assert_eq!(*reference.media_type(), MediaType::Image);
    assert_eq!(reference.mime_type(), "image/png");
    assert_eq!(*reference.size_bytes(), data.len() as i64);
    assert!(!reference.content_hash().is_empty());

    // Retrieve the data
    let retrieved = storage.retrieve(&reference).await?;
    assert_eq!(retrieved, data);

    Ok(())
}

#[tokio::test]
async fn test_deduplication() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Duplicate content";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Audio)
        .mime_type("audio/mp3")
        .duration_seconds(Some(120.5))
        .build()?;

    // Store same data twice
    let ref1 = storage.store(data, &metadata).await?;
    let ref2 = storage.store(data, &metadata).await?;

    // Should have same hash and path
    assert_eq!(ref1.content_hash(), ref2.content_hash());
    assert_eq!(ref1.storage_path(), ref2.storage_path());

    // Should only exist once on disk
    let path = std::path::Path::new(ref1.storage_path());
    assert!(path.exists());

    Ok(())
}

#[tokio::test]
async fn test_hash_verification() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Original data";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Video)
        .mime_type("video/mp4")
        .build()?;

    let reference = storage.store(data, &metadata).await?;

    // Corrupt the file
    let path = std::path::Path::new(reference.storage_path());
    tokio::fs::write(path, b"Corrupted data").await?;

    // Should detect corruption on retrieve
    let result = storage.retrieve(&reference).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        BotticelliErrorKind::Storage(_)
    ));

    Ok(())
}

#[tokio::test]
async fn test_delete() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Delete me";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/jpeg")
        .build()?;

    let reference = storage.store(data, &metadata).await?;
    assert!(storage.exists(&reference).await?);

    storage.delete(&reference).await?;
    assert!(!storage.exists(&reference).await?);

    Ok(())
}

#[tokio::test]
async fn test_not_found() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let fake_reference = MediaReferenceBuilder::default()
        .id(Uuid::new_v4())
        .content_hash("nonexistent")
        .storage_backend("filesystem")
        .storage_path(temp_dir.path().join("fake.dat").to_string_lossy().to_string())
        .size_bytes(100)
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;

    let result = storage.retrieve(&fake_reference).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_content_addressable_structure() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"Test structure";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;

    let reference = storage.store(data, &metadata).await?;
    let path = std::path::Path::new(reference.storage_path());

    // Verify path structure: base/images/XX/YY/hash
    let components: Vec<_> = path.components().collect();
    assert!(components.len() >= 4);

    // Last component should be the full hash
    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Invalid filename")?;
    assert_eq!(filename, reference.content_hash());

    // Should be in images subdirectory
    assert!(reference.storage_path().contains("images"));

    Ok(())
}

#[tokio::test]
async fn test_no_direct_urls() -> TestResult {
    let temp_dir = TempDir::new()?;
    let storage = FileSystemStorage::new(temp_dir.path())?;

    let data = b"No URL support";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()?;

    let reference = storage.store(data, &metadata).await?;
    let url = storage
        .get_url(&reference, std::time::Duration::from_secs(3600))
        .await?;

    assert!(url.is_none());

    Ok(())
}
