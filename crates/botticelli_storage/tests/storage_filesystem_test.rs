//! Tests for filesystem storage backend.

use botticelli_storage::{
    FileSystemStorage, MediaMetadata, MediaReference, MediaStorage, MediaType,
};
use tempfile::TempDir;
use uuid::Uuid;

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
    let path = std::path::Path::new(&ref1.storage_path);
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
    let path = std::path::Path::new(&reference.storage_path);
    tokio::fs::write(path, b"Corrupted data").await.unwrap();

    // Should detect corruption on retrieve
    let result = storage.retrieve(&reference).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        botticelli_error::BotticelliErrorKind::Storage(_)
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
        storage_path: temp_dir
            .path()
            .join("fake.dat")
            .to_string_lossy()
            .to_string(),
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
    let path = std::path::Path::new(&reference.storage_path);

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
        .get_url(&reference, std::time::Duration::from_secs(3600))
        .await
        .unwrap();

    assert!(url.is_none());
}
