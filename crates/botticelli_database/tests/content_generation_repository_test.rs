//! Integration tests for ContentGenerationRepository trait implementation.

use botticelli_database::{
    NewContentGenerationRow, PostgresContentGenerationRepository, UpdateContentGenerationRow,
};
use botticelli_error::{DatabaseError, DatabaseErrorKind};
use botticelli_interface::ContentGenerationRepository;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::env;
use tracing::info;

type DatabaseResult<T> = Result<T, DatabaseError>;



/// Initialize tracing for tests.
///
/// Uses try_init() to avoid panicking if already initialized.
/// Logs are captured by test framework when tests fail.
fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .with_test_writer()
        .try_init();
}

fn get_database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://botticelli:renaissance@localhost:5432/botticelli_test".to_string()
    })
}

fn create_pool(database_url: &str) -> DatabaseResult<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager).map_err(DatabaseError::from)
}

#[test]
#[cfg(feature = "postgres")]
fn test_start_and_get_generation() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_start_and_get_generation");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get().map_err(DatabaseError::from)?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    // Create new generation record
    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: Some("test_user".to_string()),
    };

    // Start generation
    let row = repo.start_generation(new_gen)?;
    assert_eq!(row.table_name(), &table_name);
    assert_eq!(row.status(), "running");

    // Get generation by table name
    let retrieved = repo.get_by_table_name(&table_name)?;
    assert!(retrieved.is_some(), "Expected to find generation");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.table_name(), &table_name);
    assert_eq!(retrieved.narrative_name(), "test_narrative");

    // Cleanup
    repo.delete_generation(&table_name)?;

    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_complete_generation() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_complete_generation");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get().map_err(|e| DatabaseError::from(e))?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let table_name = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let new_gen = NewContentGenerationRow {
        table_name: table_name.clone(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };

    // Start generation
    repo.start_generation(new_gen)?;

    // Complete generation
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

    // Cleanup
    repo.delete_generation(&table_name)?;

    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_list_generations() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_list_generations");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get().map_err(DatabaseError::from)?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    // Create multiple generations
    let table1 = format!("test_table_{}", uuid::Uuid::new_v4().simple());
    let table2 = format!("test_table_{}", uuid::Uuid::new_v4().simple());

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

    // List all generations
    let all = repo.list_generations(None, 100)?;
    assert!(
        all.len() >= 2,
        "Expected at least 2 generations, got {}",
        all.len()
    );

    // List only successful
    let successful = repo.list_generations(Some("success".to_string()), 100)?;
    assert!(
        successful.iter().any(|g| g.table_name() == &table2),
        "Expected to find successful generation"
    );

    // Cleanup
    repo.delete_generation(&table1).ok();
    repo.delete_generation(&table2).ok();

    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_get_last_successful() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_get_last_successful");

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

    // Start and complete a successful generation
    repo.start_generation(new_gen)?;

    let update = UpdateContentGenerationRow {
        completed_at: Some(chrono::Utc::now()),
        row_count: Some(50),
        generation_duration_ms: Some(3000),
        status: Some("success".to_string()),
        error_message: None,
    };

    repo.complete_generation(&table_name, update)?;

    // Get last successful
    let last = repo.get_last_successful()?;
    assert!(last.is_some(), "Expected to find last successful generation");

    let last = last.unwrap();
    assert_eq!(last.status(), "success");

    // Cleanup
    repo.delete_generation(&table_name)?;

    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_delete_generation() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_delete_generation");

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

    // Start generation
    repo.start_generation(new_gen)?;

    // Delete generation
    repo.delete_generation(&table_name)?;

    // Verify deletion
    let result = repo.get_by_table_name(&table_name)?;
    assert!(result.is_none(), "Expected generation to be deleted");

    Ok(())
}

#[test]
#[cfg(feature = "postgres")]
fn test_get_nonexistent_generation() -> DatabaseResult<()> {
    init_test_tracing();
    info!("Starting test: test_get_nonexistent_generation");

    let database_url = get_database_url();
    let pool = create_pool(&database_url)?;
    let mut conn = pool.get().map_err(DatabaseError::from)?;

    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let nonexistent_table = format!("nonexistent_{}", uuid::Uuid::new_v4().simple());
    let result = repo.get_by_table_name(&nonexistent_table)?;

    assert!(
        result.is_none(),
        "Expected None for nonexistent generation"
    );

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
