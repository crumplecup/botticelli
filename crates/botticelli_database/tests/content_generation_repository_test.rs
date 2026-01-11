//! Integration tests for ContentGenerationRepository trait implementation.

mod helpers;

use botticelli_database::{
    NewContentGenerationRow, PostgresContentGenerationRepository, UpdateContentGenerationRow,
};
use botticelli_interface::ContentGenerationRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::env;

fn get_database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://botticelli:renaissance@localhost:5432/botticelli_test".to_string()
    })
}

fn create_pool(database_url: &str) -> anyhow::Result<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Ok(Pool::builder().build(manager)?)
}

#[test]
#[cfg(feature = "postgres")]
fn test_start_and_get_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing content generation creation and retrieval");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    // Create new generation record
    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    debug!(table_name = %table_name, "Creating test generation");
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: Some("test_user".to_string()),
    };

    // Start generation
    debug!("Starting generation");
    let row = repo.start_generation(new_gen)?;
    assert_eq!(row.table_name(), &table_name);
    assert_eq!(row.status(), "running");
    debug!("Generation started successfully");

    // Get generation by table name
    debug!("Retrieving generation by table name");
    let retrieved = repo.get_by_table_name(&table_name)?;
    assert!(retrieved.is_some(), "Expected to find generation");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.table_name(), &table_name);
    assert_eq!(retrieved.narrative_name(), "test_narrative");
    debug!("Generation retrieved successfully");

    // Cleanup
    debug!("Cleaning up test generation");
    repo.delete_generation(&table_name)?;

    info!("Content generation test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_complete_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing generation completion");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    debug!(table_name = %table_name, "Creating test generation");
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    // Start generation
    repo.start_generation(new_gen)?;

    // Start generation
    debug!("Starting generation");
    repo.start_generation(new_gen)?;

    // Complete generation
    debug!("Completing generation with success status");
    let update = UpdateContentGenerationRow {
        completed_at: Some(chrono::Utc::now()),
        row_count: Some(100),
        generation_duration_ms: Some(5000),
        status: Some("success".to_string()),
        error_message: None,
    };

    let updated = repo.complete_generation(&table_name, update)?;
    assert_eq!(updated.status(), "success");
    assert_eq!(updated.row_count(), &Some(100));
    assert_eq!(updated.generation_duration_ms(), &Some(5000));
    debug!("Generation completed successfully");

    // Cleanup
    debug!("Cleaning up test generation");
    repo.delete_generation(&table_name)?;

    info!("Generation completion test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_list_generations() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing generation listing with filters");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    // Create multiple generations
    let table1 = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let table2 = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    debug!(table1 = %table1, table2 = %table2, "Creating two test generations");

    let new_gen1 = NewContentGenerationRow {
        table_name: table1.clone(),
        narrative_file: "test1.toml".to_string(),
        narrative_name: "test1".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    let new_gen2 = NewContentGenerationRow {
        table_name: table2.clone(),
        narrative_file: "test2.toml".to_string(),
        narrative_name: "test2".to_string(),
        status: "success".to_string(),
        created_by: None,
    };

    repo.start_generation(new_gen1)?;
    repo.start_generation(new_gen2)?;
    debug!("Created two test generations");

    // List all generations
    debug!("Listing all generations");
    let all = repo.list_generations(None, 100)?;
    assert!(
        all.len() >= 2,
        "Expected at least 2 generations, got {}",
        all.len()
    );
    debug!(count = all.len(), "Listed all generations");

    // List only successful
    debug!("Filtering for successful generations");
    let successful = repo.list_generations(Some("success".to_string()), 100)?;
    assert!(
        successful.iter().any(|g| g.table_name() == &table2),
        "Expected to find successful generation"
    );
    debug!(
        success_count = successful.len(),
        "Listed successful generations"
    );

    // Cleanup
    debug!("Cleaning up test generations");
    repo.delete_generation(&table1).ok();
    repo.delete_generation(&table2).ok();

    info!("Generation listing test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_get_last_successful() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing retrieval of last successful generation");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    debug!(table_name = %table_name, "Creating successful test generation");
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    // Start and complete a successful generation
    debug!("Starting generation");
    repo.start_generation(new_gen)?;

    debug!("Completing generation with success");
    let update = UpdateContentGenerationRow {
        completed_at: Some(chrono::Utc::now()),
        row_count: Some(50),
        generation_duration_ms: Some(3000),
        status: Some("success".to_string()),
        error_message: None,
    };

    repo.complete_generation(&table_name, update)?;

    // Get last successful
    debug!("Retrieving last successful generation");
    let last = repo.get_last_successful()?;
    assert!(
        last.is_some(),
        "Expected to find last successful generation"
    );

    let last = last.unwrap();
    assert_eq!(last.status(), "success");
    debug!(table = %last.table_name(), "Retrieved last successful generation");

    // Cleanup
    debug!("Cleaning up test generation");
    repo.delete_generation(&table_name)?;

    info!("Last successful generation test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_delete_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing generation deletion");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    debug!(table_name = %table_name, "Creating test generation for deletion");
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    // Start generation
    debug!("Starting generation");
    repo.start_generation(new_gen)?;

    // Delete generation
    debug!("Deleting generation");
    repo.delete_generation(&table_name)?;

    // Verify deletion
    debug!("Verifying generation was deleted");
    let result = repo.get_by_table_name(&table_name)?;
    assert!(result.is_none(), "Expected generation to be deleted");
    debug!("Confirmed generation is deleted");

    info!("Generation deletion test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_get_nonexistent_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing retrieval of nonexistent generation");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get()?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let nonexistent_table = format!("nonexistent_{}", uuid::Uuid::new_v4().simple());
    debug!(table = %nonexistent_table, "Attempting to retrieve nonexistent generation");
    let result = repo.get_by_table_name(&nonexistent_table)?;

    assert!(result.is_none(), "Expected None for nonexistent generation");
    debug!("Confirmed nonexistent generation returns None");

    info!("Nonexistent generation test passed");
    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_start_generation_idempotent() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_start_generation_idempotent");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get().map_err(DatabaseError::from)?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    // Start generation twice - should return existing record
    let first = repo.start_generation(new_gen.clone())?;
    let second = repo.start_generation(new_gen)?;

    assert_eq!(first.table_name(), second.table_name());
    assert_eq!(first.id(), second.id());

    // Cleanup
    repo.delete_generation(&table_name)?;

    Ok(())
}
