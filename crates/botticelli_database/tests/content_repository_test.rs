//! Integration tests for ContentRepository trait implementation.

use botticelli_database::{DatabaseContentRepository, DatabaseResult};
use botticelli_interface::ContentRepository;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{PgConnection, RunQueryDsl};
use serde_json::json;
use std::env;

fn get_database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://botticelli:renaissance@localhost:5432/botticelli_test".to_string()
    })
}

fn create_pool(database_url: &str) -> DatabaseResult<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .map_err(|e| botticelli_error::DatabaseErrorKind::Connection(e.to_string()).into())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_create_content_table() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "title": "text",
        "description": "text",
        "url": "text"
    });

    let result = repo.create_content_table(&table_name, &schema).await;
    
    assert!(result.is_ok(), "Failed to create content table: {:?}", result.err());
    
    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name))
        .execute(&mut conn)
        .ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_insert_and_query_content() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "title": "text",
        "description": "text",
        "rating": "integer"
    });

    // Create table
    repo.create_content_table(&table_name, &schema)
        .await
        .expect("Failed to create table");

    // Insert content
    let content = json!({
        "title": "Test Title",
        "description": "Test Description",
        "rating": 5
    });

    let id = repo
        .insert_content(&table_name, &content)
        .await
        .expect("Failed to insert content");

    assert!(id > 0, "Expected positive ID, got {}", id);

    // Query content
    let results = repo
        .query_content(&table_name, None, None)
        .await
        .expect("Failed to query content");

    assert!(!results.is_empty(), "Expected at least one result");
    assert_eq!(results[0]["title"], "Test Title");
    assert_eq!(results[0]["description"], "Test Description");
    assert_eq!(results[0]["rating"], 5);

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name))
        .execute(&mut conn)
        .ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_with_limit() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "value": "integer"
    });

    // Create table and insert multiple rows
    repo.create_content_table(&table_name, &schema)
        .await
        .expect("Failed to create table");

    for i in 0..10 {
        let content = json!({"value": i});
        repo.insert_content(&table_name, &content)
            .await
            .expect("Failed to insert content");
    }

    // Query with limit
    let results = repo
        .query_content(&table_name, None, Some(5))
        .await
        .expect("Failed to query content");

    assert_eq!(results.len(), 5, "Expected exactly 5 results");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name))
        .execute(&mut conn)
        .ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_empty_table() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "data": "text"
    });

    // Create empty table
    repo.create_content_table(&table_name, &schema)
        .await
        .expect("Failed to create table");

    // Query empty table
    let results = repo
        .query_content(&table_name, None, None)
        .await
        .expect("Failed to query content");

    assert!(results.is_empty(), "Expected empty results");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name))
        .execute(&mut conn)
        .ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_insert_special_characters() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "text": "text"
    });

    repo.create_content_table(&table_name, &schema)
        .await
        .expect("Failed to create table");

    // Insert content with special characters
    let special_text = "Test with 'quotes', \"double quotes\", and\nnewlines";
    let content = json!({
        "text": special_text
    });

    repo.insert_content(&table_name, &content)
        .await
        .expect("Failed to insert content with special characters");

    // Query and verify
    let results = repo
        .query_content(&table_name, None, None)
        .await
        .expect("Failed to query content");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["text"], special_text);

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name))
        .execute(&mut conn)
        .ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_nonexistent_table() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentRepository::new(pool);

    let table_name = "nonexistent_table_12345";
    
    let result = repo
        .query_content(table_name, None, None)
        .await;

    assert!(result.is_err(), "Expected error for nonexistent table");
}
