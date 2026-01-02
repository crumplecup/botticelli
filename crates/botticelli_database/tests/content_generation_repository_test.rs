//! Integration tests for ContentGenerationRepository trait implementation.

use botticelli_database::{PostgresContentGenerationRepository, DatabaseResult};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
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
async fn test_create_and_get_generation() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseContentGenerationRepository::new(pool);

    // Create content generation record
    let table_name = "test_content_table";
    let content_id = 1;
    let prompt = "Generate test content";
    let parameters = json!({
        "model": "test-model",
        "temperature": 0.7
    });

    let generation_id = repo
        .create_generation(table_name, content_id, prompt, &parameters)
        .await
        .expect("Failed to create generation");

    assert!(generation_id > 0, "Expected positive generation ID");

    // Get generation by ID
    let generation = repo
        .get_generation(generation_id)
        .await
        .expect("Failed to get generation")
        .expect("Generation not found");

    assert_eq!(generation.table_name(), table_name);
    assert_eq!(generation.content_id(), content_id);
    assert_eq!(generation.prompt(), prompt);
    assert_eq!(generation.parameters(), &parameters);

    // Cleanup
    repo.delete_generation(generation_id).await.ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_generations_by_table() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresContentGenerationRepository::new(pool);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let prompt = "Test prompt";
    let params = json!({});

    // Create multiple generations for same table
    let id1 = repo
        .create_generation(&table_name, 1, prompt, &params)
        .await
        .expect("Failed to create generation 1");
    
    let id2 = repo
        .create_generation(&table_name, 2, prompt, &params)
        .await
        .expect("Failed to create generation 2");

    // List generations for table
    let generations = repo
        .list_generations_by_table(&table_name)
        .await
        .expect("Failed to list generations");

    assert!(generations.len() >= 2, "Expected at least 2 generations");
    
    let found1 = generations.iter().any(|g| g.content_id() == 1);
    let found2 = generations.iter().any(|g| g.content_id() == 2);
    
    assert!(found1, "Expected to find generation for content_id 1");
    assert!(found2, "Expected to find generation for content_id 2");

    // Cleanup
    repo.delete_generation(id1).await.ok();
    repo.delete_generation(id2).await.ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_generation_with_complex_parameters() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresContentGenerationRepository::new(pool);

    let table_name = "test_table";
    let content_id = 1;
    let prompt = "Complex test";
    let parameters = json!({
        "model": "gpt-4",
        "temperature": 0.8,
        "max_tokens": 1000,
        "stop_sequences": ["END", "STOP"],
        "nested": {
            "value": 42,
            "array": [1, 2, 3]
        }
    });

    let generation_id = repo
        .create_generation(table_name, content_id, prompt, &parameters)
        .await
        .expect("Failed to create generation");

    // Retrieve and verify
    let generation = repo
        .get_generation(generation_id)
        .await
        .expect("Failed to get generation")
        .expect("Generation not found");

    assert_eq!(generation.parameters(), &parameters);
    assert_eq!(generation.parameters()["nested"]["array"][1], 2);

    // Cleanup
    repo.delete_generation(generation_id).await.ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_delete_generation() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresContentGenerationRepository::new(pool);

    let generation_id = repo
        .create_generation("test_table", 1, "Test prompt", &json!({}))
        .await
        .expect("Failed to create generation");

    // Delete generation
    repo.delete_generation(generation_id)
        .await
        .expect("Failed to delete generation");

    // Verify deletion
    let result = repo.get_generation(generation_id).await;
    
    assert!(
        result.is_ok() && result.unwrap().is_none(),
        "Expected generation to be deleted"
    );
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_empty_table_generations() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresContentGenerationRepository::new(pool);

    let nonexistent_table = format!("nonexistent_{}", uuid::Uuid::new_v4().simple());
    
    let generations = repo
        .list_generations_by_table(&nonexistent_table)
        .await
        .expect("Failed to list generations");

    assert!(generations.is_empty(), "Expected empty list for nonexistent table");
}
