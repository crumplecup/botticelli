//! Integration tests for ContentRepository trait implementation.

mod helpers;

use botticelli_database::DatabaseContentRepository;
use botticelli_interface::ContentRepository;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{PgConnection, RunQueryDsl};
use serde_json::json;
use std::env;

fn get_database_url() -> anyhow::Result<String> {
    match env::var("DATABASE_URL") {
        Ok(url) => Ok(url),
        Err(_) => {
            Ok("postgres://botticelli:renaissance@localhost:5432/botticelli_test".to_string())
        }
    }
}

fn create_pool(database_url: &str) -> anyhow::Result<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Ok(Pool::builder().build(manager)?)
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_create_content_table() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing create_content_table");

    let database_url = get_database_url()?;
    debug!(database_url = %database_url, "Creating connection pool");
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "title": "text",
        "description": "text",
        "url": "text"
    });
    debug!(table_name = %table_name, schema = ?schema, "Creating content table");

    let result = repo.create_content_table(&table_name, &schema).await;

    assert!(
        result.is_ok(),
        "Failed to create content table: {:?}",
        result.err()
    );
    debug!(table_name = %table_name, "Table created successfully");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get()?;
    debug!(table_name = %table_name, "Cleaning up test table");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name)).execute(&mut conn)?;

    info!("create_content_table test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_insert_and_query_content() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing insert_content and query_content");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "title": "text",
        "description": "text",
        "rating": "integer"
    });

    // Create table
    debug!(table_name = %table_name, "Creating content table");
    repo.create_content_table(&table_name, &schema).await?;

    // Insert content
    let content = json!({
        "title": "Test Title",
        "description": "Test Description",
        "rating": 5
    });
    debug!(content = ?content, "Inserting content");

    let id = repo.insert_content(&table_name, &content).await?;

    assert!(id > 0, "Expected positive ID, got {}", id);
    debug!(id = id, "Content inserted successfully");

    // Query content
    debug!(table_name = %table_name, "Querying content");
    let results = repo.query_content(&table_name, None, None).await?;

    assert!(!results.is_empty(), "Expected at least one result");
    assert_eq!(results[0]["title"], "Test Title");
    assert_eq!(results[0]["description"], "Test Description");
    assert_eq!(results[0]["rating"], 5);
    debug!(count = results.len(), "Retrieved content successfully");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get()?;
    debug!(table_name = %table_name, "Cleaning up test table");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name)).execute(&mut conn)?;

    info!("insert_and_query_content test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_with_limit() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing query_content with limit");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "value": "integer"
    });

    // Create table and insert multiple rows
    debug!(table_name = %table_name, "Creating content table");
    repo.create_content_table(&table_name, &schema).await?;

    debug!("Inserting 10 test rows");
    for i in 0..10 {
        let content = json!({"value": i});
        repo.insert_content(&table_name, &content).await?;
    }
    debug!("All rows inserted");

    // Query with limit
    debug!(limit = 5, "Querying with limit");
    let results = repo.query_content(&table_name, None, Some(5)).await?;

    assert_eq!(results.len(), 5, "Expected exactly 5 results");
    debug!(count = results.len(), "Query with limit successful");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get()?;
    debug!(table_name = %table_name, "Cleaning up test table");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name)).execute(&mut conn)?;

    info!("query_with_limit test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_empty_table() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing query_content on empty table");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "data": "text"
    });

    // Create empty table
    debug!(table_name = %table_name, "Creating empty content table");
    repo.create_content_table(&table_name, &schema).await?;

    // Query empty table
    debug!(table_name = %table_name, "Querying empty table");
    let results = repo.query_content(&table_name, None, None).await?;

    assert!(results.is_empty(), "Expected empty results");
    debug!("Empty query returned no results as expected");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get()?;
    debug!(table_name = %table_name, "Cleaning up test table");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name)).execute(&mut conn)?;

    info!("query_empty_table test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_insert_special_characters() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing insert_content with special characters");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = format!("test_content_{}", uuid::Uuid::new_v4().simple());
    let schema = json!({
        "text": "text"
    });

    debug!(table_name = %table_name, "Creating content table");
    repo.create_content_table(&table_name, &schema).await?;

    // Insert content with special characters
    let special_text = "Test with 'quotes', \"double quotes\", and\nnewlines";
    let content = json!({
        "text": special_text
    });
    debug!(text_length = special_text.len(), "Inserting content with special characters");

    repo.insert_content(&table_name, &content).await?;
    debug!("Content with special characters inserted");

    // Query and verify
    debug!("Querying inserted content");
    let results = repo.query_content(&table_name, None, None).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["text"], special_text);
    debug!("Special characters preserved correctly");

    // Cleanup
    let pool = repo.pool();
    let mut conn = pool.get()?;
    debug!(table_name = %table_name, "Cleaning up test table");
    diesel::sql_query(format!("DROP TABLE IF EXISTS {}", table_name)).execute(&mut conn)?;

    info!("insert_special_characters test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_query_nonexistent_table() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing query_content on nonexistent table");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let table_name = "nonexistent_table_12345";

    debug!(table_name = %table_name, "Attempting to query nonexistent table");
    let result = repo.query_content(table_name, None, None).await;

    assert!(result.is_err(), "Expected error for nonexistent table");
    debug!("Query correctly returned error for nonexistent table");

    info!("query_nonexistent_table test passed");
    Ok(())
}
