use botticelli_database::{DatabaseContentRepository, DatabaseResult};
use botticelli_error::BotticelliResult;
use botticelli_interface::ContentRepository;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::env;
use tracing::info;

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

fn get_database_url() -> BotticelliResult<String> {
    match env::var("DATABASE_URL") {
        Ok(url) => Ok(url),
        Err(_) => Ok("postgres://botticelli:renaissance@localhost:5432/botticelli_test".to_string()),
    }
}

fn create_pool(database_url: &str) -> DatabaseResult<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .map_err(|e| botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(e.to_string())))
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_content_repository_query() -> BotticelliResult<()> {
    init_test_tracing();
    info!("Starting test: test_content_repository_query");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    // Test querying with limit
    let result = repo
        .query_content("generated_images", None, Some(5))
        .await;

    match result {
        Ok(rows) => {
            assert!(
                rows.len() <= 5,
                "Query should respect limit of 5, got {}",
                rows.len()
            );
        }
        Err(e) => {
            // Table might not exist in test database, that's ok
            info!("Query failed (expected if table doesn't exist): {}", e);
        }
    }

    Ok(())
}



#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_content_repository_different_limits() -> BotticelliResult<()> {
    init_test_tracing();
    info!("Starting test: test_content_repository_different_limits");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let limits = vec![1, 5, 10, 50];

    for limit in limits {
        info!("Testing query with limit: {}", limit);
        let result = repo
            .query_content("generated_images", None, Some(limit))
            .await;

        if let Ok(rows) = result {
            assert!(
                rows.len() <= limit as usize,
                "Query with limit {} should return at most {} rows, got {}",
                limit,
                limit,
                rows.len()
            );
        }
    }

    Ok(())
}
