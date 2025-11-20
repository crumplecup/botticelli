//! Integration tests for table reference system (Phase 3).
//!
//! Tests table querying, formatting, and integration with narrative execution.

use botticelli::{
    DatabaseError, DatabaseErrorKind, DatabaseResult, TableQueryExecutor,
    TableQueryViewBuilder,
};
use diesel::{Connection, PgConnection, RunQueryDsl};
use std::{env, sync::{Arc, Mutex}};
use dotenvy;

/// Get database connection from environment.
fn get_test_connection() -> DatabaseResult<PgConnection> {
    // Load .env file for tests
    let _ = dotenvy::dotenv();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    PgConnection::establish(&database_url)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(format!("Failed to connect: {}", e))))
}

/// Create a test table with sample data.
fn create_test_table(conn: &mut PgConnection) -> DatabaseResult<()> {
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS test_social_posts (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            status TEXT NOT NULL,
            platform TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(conn)
    .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(format!("Failed to create table: {}", e))))?;

    // Insert sample data
    diesel::sql_query(
        "INSERT INTO test_social_posts (title, body, status, platform) VALUES
        ('First Post', 'This is my first post', 'published', 'twitter'),
        ('Second Post', 'Another great post', 'published', 'twitter'),
        ('Draft Post', 'Work in progress', 'draft', 'twitter'),
        ('Facebook Post', 'Sharing on Facebook', 'published', 'facebook'),
        ('Scheduled Post', 'Will publish later', 'scheduled', 'twitter')
        ON CONFLICT DO NOTHING",
    )
    .execute(conn)
    .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(format!("Failed to insert data: {}", e))))?;

    Ok(())
}

/// Clean up test table.
fn cleanup_test_table(conn: &mut PgConnection) -> DatabaseResult<()> {
    diesel::sql_query("DROP TABLE IF EXISTS test_social_posts")
        .execute(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(format!("Failed to drop table: {}", e))))?;
    Ok(())
}

#[test]
fn test_basic_table_query() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    let query = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .limit(10)
        .build()
        .expect("Failed to build query");

    let results = executor.query_table(&query)?;

    assert!(!results.is_empty(), "Should return results");
    assert!(
        results.len() <= 10,
        "Should respect limit of 10, got {}",
        results.len()
    );

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}

#[test]
fn test_column_selection() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    let query = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .columns(vec!["title".to_string(), "status".to_string()])
        .limit(5)
        .build()
        .expect("Failed to build query");

    let results = executor.query_table(&query)?;

    assert!(!results.is_empty(), "Should return results");

    // Check that only requested columns are present
    for row in &results {
        let obj = row.as_object().expect("Result should be JSON object");
        assert!(obj.contains_key("title"), "Should have title column");
        assert!(obj.contains_key("status"), "Should have status column");
        // Should not have other columns like body or platform
        assert_eq!(obj.len(), 2, "Should only have 2 columns");
    }

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}

#[test]
fn test_where_clause_filtering() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    let query = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .where_clause("status = 'published'")
        .build()
        .expect("Failed to build query");

    let results = executor.query_table(&query)?;

    assert!(!results.is_empty(), "Should return published posts");

    // Verify all results have status = 'published'
    for row in &results {
        let status = row["status"].as_str().expect("Status should be string");
        assert_eq!(status, "published", "All results should be published");
    }

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}

#[test]
fn test_order_by() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    let query = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .columns(vec!["title".to_string()])
        .order_by("title ASC")
        .build()
        .expect("Failed to build query");

    let results = executor.query_table(&query)?;

    assert!(results.len() >= 2, "Need at least 2 results to test ordering");

    // Verify results are sorted by title
    let titles: Vec<String> = results
        .iter()
        .map(|r| r["title"].as_str().unwrap().to_string())
        .collect();

    let mut sorted_titles = titles.clone();
    sorted_titles.sort();

    assert_eq!(
        titles, sorted_titles,
        "Results should be sorted by title ascending"
    );

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}

#[test]
fn test_pagination() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    // Get first page
    let query1 = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .order_by("id ASC")
        .limit(2)
        .offset(0)
        .build()
        .expect("Failed to build query");

    let page1 = executor.query_table(&query1)?;

    // Get second page
    let query2 = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .order_by("id ASC")
        .limit(2)
        .offset(2)
        .build()
        .expect("Failed to build query");

    let page2 = executor.query_table(&query2)?;

    assert_eq!(page1.len(), 2, "First page should have 2 results");
    assert_eq!(page2.len(), 2, "Second page should have 2 results");

    // Pages should not overlap
    let id1 = page1[0]["id"].as_i64().unwrap();
    let id2 = page2[0]["id"].as_i64().unwrap();
    assert_ne!(id1, id2, "Pages should have different records");

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}

#[test]
fn test_json_formatting() {
    use botticelli::format_as_json;
    use serde_json::json;

    let data = vec![
        json!({"name": "Alice", "age": 30}),
        json!({"name": "Bob", "age": 25}),
    ];

    let formatted = format_as_json(&data);

    assert!(formatted.contains("Alice"), "Should contain Alice");
    assert!(formatted.contains("Bob"), "Should contain Bob");
    assert!(formatted.contains("30"), "Should contain age 30");
    assert!(formatted.starts_with('['), "Should start with [");
    assert!(formatted.ends_with(']'), "Should end with ]");
}

#[test]
fn test_markdown_formatting() {
    use botticelli::format_as_markdown;
    use serde_json::json;

    let data = vec![
        json!({"name": "Alice", "age": 30}),
        json!({"name": "Bob", "age": 25}),
    ];

    let formatted = format_as_markdown(&data);

    assert!(formatted.contains("| name"), "Should have name column");
    assert!(formatted.contains("| age"), "Should have age column");
    assert!(formatted.contains("| Alice"), "Should contain Alice");
    assert!(formatted.contains("| Bob"), "Should contain Bob");
    assert!(formatted.contains("---"), "Should have header separator");
}

#[test]
fn test_csv_formatting() {
    use botticelli::format_as_csv;
    use serde_json::json;

    let data = vec![
        json!({"name": "Alice", "age": 30}),
        json!({"name": "Bob", "age": 25}),
    ];

    let formatted = format_as_csv(&data);

    // Check that header exists (key order may vary)
    assert!(
        formatted.starts_with("name,age") || formatted.starts_with("age,name"),
        "Should have CSV header"
    );
    // Check data is present (order may vary based on header)
    assert!(
        formatted.contains("Alice") && formatted.contains("30"),
        "Should contain Alice and age 30"
    );
    assert!(
        formatted.contains("Bob") && formatted.contains("25"),
        "Should contain Bob and age 25"
    );
}

#[test]
fn test_table_not_found() -> DatabaseResult<()> {
    let conn = get_test_connection()?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc);

    let query = TableQueryViewBuilder::default()
        .table_name("nonexistent_table")
        .build()
        .expect("Failed to build query");

    let result = executor.query_table(&query);

    assert!(
        result.is_err(),
        "Should return error for nonexistent table"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found")
            || err.to_string().contains("does not exist")
            || err.to_string().contains("nonexistent"),
        "Error should indicate table not found: {}",
        err
    );

    Ok(())
}

#[test]
fn test_sql_injection_protection() -> DatabaseResult<()> {
    let mut conn = get_test_connection()?;
    create_test_table(&mut conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));
    let executor = TableQueryExecutor::new(conn_arc.clone());

    // Try table name with SQL injection
    let query = TableQueryViewBuilder::default()
        .table_name("test_social_posts; DROP TABLE test_social_posts")
        .build()
        .expect("Failed to build query");

    let result = executor.query_table(&query);
    assert!(result.is_err(), "Should reject invalid table name");

    // Try WHERE clause with dangerous patterns
    let query2 = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .where_clause("1=1; DROP TABLE test_social_posts")
        .build()
        .expect("Failed to build query");

    let result2 = executor.query_table(&query2);
    assert!(
        result2.is_err(),
        "Should reject WHERE clause with dangerous patterns"
    );

    // Verify table still exists
    let safe_query = TableQueryViewBuilder::default()
        .table_name("test_social_posts")
        .limit(1)
        .build()
        .expect("Failed to build query");

    let safe_result = executor.query_table(&safe_query);
    assert!(
        safe_result.is_ok(),
        "Table should still exist after injection attempts"
    );

    let mut conn = conn_arc.lock().unwrap();
    cleanup_test_table(&mut conn)?;
    Ok(())
}
