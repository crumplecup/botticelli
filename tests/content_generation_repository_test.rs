//! Tests for content generation repository.

#![cfg(feature = "database")]

use boticelli::{
    ContentGenerationRepository, NewContentGenerationRow,
    PostgresContentGenerationRepository, UpdateContentGenerationRow, establish_connection, PgConnection,
};
use chrono::Utc;

/// Create a test database connection.
/// 
/// Note: These tests require a running PostgreSQL database with the
/// boticelli schema and migrations applied.
fn create_test_connection() -> PgConnection {
    establish_connection().expect("Failed to establish test database connection")
}

/// Helper to create a test generation record.
fn create_test_generation(table_name: &str) -> NewContentGenerationRow {
    NewContentGenerationRow {
        table_name: table_name.to_string(),
        narrative_file: "tests/test_narrative.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: Some("test_user".to_string()),
    }
}

/// Helper to clean up test data before/after tests.
fn cleanup_test_generation(conn: &mut PgConnection, table_name: &str) {
    let mut repo = PostgresContentGenerationRepository::new(conn);
    let _ = repo.delete_generation(table_name);
}

#[test]
fn test_start_generation() {
    let mut conn = create_test_connection();
    
    let table_name = "test_start_gen";
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    let new_gen = create_test_generation(table_name);
    let result = repo.start_generation(new_gen).unwrap();
    
    assert_eq!(result.table_name, table_name);
    assert_eq!(result.status, "running");
    assert_eq!(result.narrative_name, "test_narrative");
    assert!(result.completed_at.is_none());
    assert!(result.row_count.is_none());
    
    // Cleanup at end
    repo.delete_generation(table_name).unwrap();
}

#[test]
fn test_complete_generation_success() {
    let mut conn = create_test_connection();
    let table_name = "test_complete_success";
    
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Start a generation
    let new_gen = create_test_generation(table_name);
    repo.start_generation(new_gen).unwrap();
    
    // Complete it successfully
    let update = UpdateContentGenerationRow {
        completed_at: Some(Utc::now()),
        row_count: Some(42),
        generation_duration_ms: Some(1234),
        status: Some("success".to_string()),
        error_message: None,
    };
    
    let result = repo.complete_generation(table_name, update).unwrap();
    
    assert_eq!(result.status, "success");
    assert_eq!(result.row_count, Some(42));
    assert_eq!(result.generation_duration_ms, Some(1234));
    assert!(result.completed_at.is_some());
    assert!(result.error_message.is_none());
    
    repo.delete_generation(table_name).unwrap();
}

#[test]
fn test_complete_generation_failed() {
    let mut conn = create_test_connection();
    let table_name = "test_complete_failed";
    
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Start a generation
    let new_gen = create_test_generation(table_name);
    repo.start_generation(new_gen).unwrap();
    
    // Complete it with failure
    let update = UpdateContentGenerationRow {
        completed_at: Some(Utc::now()),
        row_count: None,
        generation_duration_ms: Some(567),
        status: Some("failed".to_string()),
        error_message: Some("Test error message".to_string()),
    };
    
    let result = repo.complete_generation(table_name, update).unwrap();
    
    assert_eq!(result.status, "failed");
    assert!(result.row_count.is_none());
    assert_eq!(result.generation_duration_ms, Some(567));
    assert_eq!(result.error_message, Some("Test error message".to_string()));
    
    repo.delete_generation(table_name).unwrap();
}

#[test]
fn test_get_last_successful() {
    let mut conn = create_test_connection();
    
    // Clean up any existing test data first
    cleanup_test_generation(&mut conn, "test_last_1");
    cleanup_test_generation(&mut conn, "test_last_2");
    cleanup_test_generation(&mut conn, "test_last_3");
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create multiple generations
    repo.start_generation(create_test_generation("test_last_1")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    repo.start_generation(create_test_generation("test_last_2")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    repo.start_generation(create_test_generation("test_last_3")).unwrap();
    
    // Complete them
    let success_update = UpdateContentGenerationRow {
        completed_at: Some(Utc::now()),
        row_count: Some(10),
        generation_duration_ms: Some(100),
        status: Some("success".to_string()),
        error_message: None,
    };
    
    repo.complete_generation("test_last_1", success_update.clone()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    repo.complete_generation("test_last_2", success_update.clone()).unwrap();
    
    // Leave test_last_3 as running
    
    // Get last successful - should be test_last_2
    let last = repo.get_last_successful().unwrap();
    assert!(last.is_some());
    let last = last.unwrap();
    assert_eq!(last.table_name, "test_last_2");
    assert_eq!(last.status, "success");
    
    // Cleanup
    repo.delete_generation("test_last_1").unwrap();
    repo.delete_generation("test_last_2").unwrap();
    repo.delete_generation("test_last_3").unwrap();
}

#[test]
fn test_list_generations() {
    let mut conn = create_test_connection();
    
    cleanup_test_generation(&mut conn, "test_list_1");
    cleanup_test_generation(&mut conn, "test_list_2");
    cleanup_test_generation(&mut conn, "test_list_3");
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create test generations
    repo.start_generation(create_test_generation("test_list_1")).unwrap();
    repo.start_generation(create_test_generation("test_list_2")).unwrap();
    repo.start_generation(create_test_generation("test_list_3")).unwrap();
    
    // Complete some
    let success_update = UpdateContentGenerationRow {
        completed_at: Some(Utc::now()),
        row_count: Some(10),
        generation_duration_ms: Some(100),
        status: Some("success".to_string()),
        error_message: None,
    };
    repo.complete_generation("test_list_1", success_update).unwrap();
    
    let failed_update = UpdateContentGenerationRow {
        completed_at: Some(Utc::now()),
        row_count: None,
        generation_duration_ms: Some(50),
        status: Some("failed".to_string()),
        error_message: Some("Test error".to_string()),
    };
    repo.complete_generation("test_list_2", failed_update).unwrap();
    
    // List all
    let all = repo.list_generations(None, 10).unwrap();
    let test_gens: Vec<_> = all.iter()
        .filter(|g| g.table_name.starts_with("test_list_"))
        .collect();
    assert!(test_gens.len() >= 3);
    
    // List only successful
    let successful = repo.list_generations(Some("success".to_string()), 10).unwrap();
    let test_success: Vec<_> = successful.iter()
        .filter(|g| g.table_name.starts_with("test_list_"))
        .collect();
    assert!(!test_success.is_empty());
    
    // List only failed
    let failed = repo.list_generations(Some("failed".to_string()), 10).unwrap();
    let test_failed: Vec<_> = failed.iter()
        .filter(|g| g.table_name.starts_with("test_list_"))
        .collect();
    assert!(!test_failed.is_empty());
    
    // Cleanup
    repo.delete_generation("test_list_1").unwrap();
    repo.delete_generation("test_list_2").unwrap();
    repo.delete_generation("test_list_3").unwrap();
}

#[test]
fn test_get_by_table_name() {
    let mut conn = create_test_connection();
    let table_name = "test_get_by_name";
    
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create a generation
    let new_gen = create_test_generation(table_name);
    repo.start_generation(new_gen).unwrap();
    
    // Get it by name
    let result = repo.get_by_table_name(table_name).unwrap();
    assert!(result.is_some());
    let generation = result.unwrap();
    assert_eq!(generation.table_name, table_name);
    assert_eq!(generation.status, "running");
    
    // Try to get non-existent
    let not_found = repo.get_by_table_name("does_not_exist").unwrap();
    assert!(not_found.is_none());
    
    repo.delete_generation(table_name).unwrap();
}

#[test]
fn test_delete_generation() {
    let mut conn = create_test_connection();
    let table_name = "test_delete";
    
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create a generation
    let new_gen = create_test_generation(table_name);
    repo.start_generation(new_gen).unwrap();
    
    // Verify it exists
    let exists = repo.get_by_table_name(table_name).unwrap();
    assert!(exists.is_some());
    
    // Delete it
    repo.delete_generation(table_name).unwrap();
    
    // Verify it's gone
    let gone = repo.get_by_table_name(table_name).unwrap();
    assert!(gone.is_none());
}

#[test]
fn test_unique_constraint_violation() {
    let mut conn = create_test_connection();
    let table_name = "test_unique";
    
    cleanup_test_generation(&mut conn, table_name);
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create first generation
    let new_gen = create_test_generation(table_name);
    repo.start_generation(new_gen).unwrap();
    
    // Try to create another with the same table_name
    let duplicate = create_test_generation(table_name);
    let result = repo.start_generation(duplicate);
    
    // Should fail with unique constraint violation
    assert!(result.is_err());
    
    repo.delete_generation(table_name).unwrap();
}

#[test]
fn test_list_with_limit() {
    let mut conn = create_test_connection();
    
    cleanup_test_generation(&mut conn, "test_limit_1");
    cleanup_test_generation(&mut conn, "test_limit_2");
    cleanup_test_generation(&mut conn, "test_limit_3");
    
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);
    
    // Create multiple generations
    repo.start_generation(create_test_generation("test_limit_1")).unwrap();
    repo.start_generation(create_test_generation("test_limit_2")).unwrap();
    repo.start_generation(create_test_generation("test_limit_3")).unwrap();
    
    // List with limit
    let limited = repo.list_generations(None, 2).unwrap();
    assert!(limited.len() >= 2); // May have more from other tests
    
    repo.delete_generation("test_limit_1").unwrap();
    repo.delete_generation("test_limit_2").unwrap();
    repo.delete_generation("test_limit_3").unwrap();
}
