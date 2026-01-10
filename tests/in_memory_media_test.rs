//! Tests for in-memory media storage implementation.

use botticelli_interface::NarrativeRepository;
use botticelli_narrative::InMemoryNarrativeRepository;
use botticelli_storage::{MediaMetadataBuilder, MediaType};

#[tokio::test]
async fn test_store_and_retrieve_media() {
    let repo = InMemoryNarrativeRepository::new();

    // Create test media data
    let data = b"test image data";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .build()
        .expect("Valid metadata");

    // Store media
    let reference = repo
        .store_media(data, &metadata)
        .await
        .expect("Should store media");

    assert_eq!(reference.mime_type(), "image/png");
    assert_eq!(*reference.size_bytes(), data.len() as i64);
    assert_eq!(reference.storage_backend(), "in-memory");

    // Retrieve media
    let retrieved = repo
        .load_media(&reference)
        .await
        .expect("Should load media");

    assert_eq!(retrieved, data);
}

#[tokio::test]
async fn test_deduplication_by_hash() {
    let repo = InMemoryNarrativeRepository::new();

    let data = b"duplicate content";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Image)
        .mime_type("image/jpeg")
        .build()
        .expect("Valid metadata");

    // Store same content twice
    let ref1 = repo
        .store_media(data, &metadata)
        .await
        .expect("Should store first");

    let ref2 = repo
        .store_media(data, &metadata)
        .await
        .expect("Should store second");

    // Should return same reference (deduplication)
    assert_eq!(ref1.id(), ref2.id());
    assert_eq!(ref1.content_hash(), ref2.content_hash());
}

#[tokio::test]
async fn test_get_media_by_hash() {
    let repo = InMemoryNarrativeRepository::new();

    let data = b"test data for hash lookup";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Audio)
        .mime_type("audio/mp3")
        .build()
        .expect("Valid metadata");

    // Store media
    let reference = repo
        .store_media(data, &metadata)
        .await
        .expect("Should store media");

    let hash = reference.content_hash();

    // Lookup by hash
    let found = repo
        .get_media_by_hash(hash)
        .await
        .expect("Should query by hash")
        .expect("Should find media");

    assert_eq!(found.id(), reference.id());
    assert_eq!(found.content_hash(), hash);
}

#[tokio::test]
async fn test_get_media_by_hash_not_found() {
    let repo = InMemoryNarrativeRepository::new();

    let result = repo
        .get_media_by_hash("nonexistent_hash")
        .await
        .expect("Should query successfully");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_load_nonexistent_media() {
    let repo = InMemoryNarrativeRepository::new();

    // Create a reference that doesn't exist in storage
    let fake_ref = botticelli_storage::MediaReferenceBuilder::default()
        .id(uuid::Uuid::new_v4())
        .media_type(MediaType::Image)
        .mime_type("image/png")
        .size_bytes(100)
        .content_hash("fake_hash")
        .storage_backend("in-memory")
        .storage_path("mem://fake")
        .build()
        .expect("Valid reference");

    let result = repo.load_media(&fake_ref).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Media not found"));
}

#[tokio::test]
async fn test_clear_removes_media() {
    let repo = InMemoryNarrativeRepository::new();

    let data = b"test data";
    let metadata = MediaMetadataBuilder::default()
        .media_type(MediaType::Video)
        .mime_type("video/mp4")
        .build()
        .expect("Valid metadata");

    let reference = repo
        .store_media(data, &metadata)
        .await
        .expect("Should store media");

    // Clear repository
    repo.clear().await;

    // Media should no longer be retrievable
    let result = repo.load_media(&reference).await;
    assert!(result.is_err());

    // Hash lookup should also fail
    let hash_result = repo
        .get_media_by_hash(reference.content_hash())
        .await
        .expect("Should query");
    assert!(hash_result.is_none());
}
