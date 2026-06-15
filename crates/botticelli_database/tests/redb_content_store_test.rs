//! Tests for RedbStorage's ContentStore implementation.

use botticelli_database::RedbStorage;
use botticelli_interface::{ContentRecord, ContentStore, ModelResponseRecord};
use chrono::Utc;

fn make_storage() -> RedbStorage {
    RedbStorage::in_memory().expect("in-memory redb")
}

fn content_record(id: &str, table: &str) -> ContentRecord {
    ContentRecord {
        id: id.to_string(),
        table_name: table.to_string(),
        content_json: serde_json::json!({"text": "hello world", "id": id}),
        created_at: Utc::now(),
    }
}

fn model_response_record(id: &str) -> ModelResponseRecord {
    ModelResponseRecord {
        id: id.to_string(),
        created_at: Utc::now(),
        provider: "gemini".to_string(),
        model_name: "gemini-2.0-flash".to_string(),
        request_messages: serde_json::json!([{"role": "user", "content": "hello"}]),
        request_temperature: Some(0.7),
        request_max_tokens: Some(100),
        request_model: None,
        response_outputs: serde_json::json!([{"text": "world"}]),
        duration_ms: Some(42),
        error_message: None,
    }
}

#[tokio::test]
async fn test_content_round_trip() {
    let s = make_storage();
    let rec = content_record("content-1", "approved_posts");

    s.save_content(&rec).await.expect("save");

    let loaded = s
        .get_content("content-1")
        .await
        .expect("get")
        .expect("should exist");

    assert_eq!(loaded.id, "content-1");
    assert_eq!(loaded.table_name, "approved_posts");
    assert_eq!(loaded.content_json["text"], "hello world");
}

#[tokio::test]
async fn test_get_content_missing() {
    let s = make_storage();
    let result = s.get_content("nonexistent").await.expect("get");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_content_upsert() {
    let s = make_storage();
    let mut rec = content_record("content-upsert", "posts");

    s.save_content(&rec).await.expect("first save");

    rec.content_json = serde_json::json!({"text": "updated content"});
    s.save_content(&rec).await.expect("second save");

    let loaded = s
        .get_content("content-upsert")
        .await
        .expect("get")
        .expect("should exist");
    assert_eq!(loaded.content_json["text"], "updated content");
}

#[tokio::test]
async fn test_list_content_table_filter() {
    let s = make_storage();

    s.save_content(&content_record("c-posts-1", "posts"))
        .await
        .expect("save");
    s.save_content(&content_record("c-posts-2", "posts"))
        .await
        .expect("save");
    s.save_content(&content_record("c-drafts-1", "drafts"))
        .await
        .expect("save");

    let posts = s.list_content("posts", 10).await.expect("list posts");
    let drafts = s.list_content("drafts", 10).await.expect("list drafts");

    assert_eq!(posts.len(), 2);
    assert_eq!(drafts.len(), 1);
    assert!(posts.iter().all(|r| r.table_name == "posts"));
    assert_eq!(drafts[0].table_name, "drafts");
}

#[tokio::test]
async fn test_list_content_limit() {
    let s = make_storage();

    for i in 0..5 {
        s.save_content(&content_record(&format!("c-limit-{}", i), "bulk_table"))
            .await
            .expect("save");
    }

    let limited = s.list_content("bulk_table", 3).await.expect("list");
    assert!(limited.len() <= 3);
}

#[tokio::test]
async fn test_delete_content() {
    let s = make_storage();
    let rec = content_record("content-del", "posts");

    s.save_content(&rec).await.expect("save");
    assert!(s.get_content("content-del").await.expect("get").is_some());

    s.delete_content("content-del").await.expect("delete");
    assert!(s.get_content("content-del").await.expect("get").is_none());
}

#[tokio::test]
async fn test_model_response_round_trip() {
    let s = make_storage();
    let rec = model_response_record("resp-1");

    s.save_model_response(&rec).await.expect("save");

    let responses = s.list_model_responses(10).await.expect("list");
    assert!(!responses.is_empty());

    let found = responses.iter().find(|r| r.id == "resp-1");
    assert!(found.is_some(), "resp-1 should be in the list");
    let found = found.unwrap();
    assert_eq!(found.provider, "gemini");
    assert_eq!(found.model_name, "gemini-2.0-flash");
}

#[tokio::test]
async fn test_model_responses_limit() {
    let s = make_storage();

    for i in 0..5 {
        s.save_model_response(&model_response_record(&format!("resp-limit-{}", i)))
            .await
            .expect("save");
    }

    let limited = s.list_model_responses(2).await.expect("list");
    assert!(limited.len() <= 2);
}
