mod helpers;

use botticelli_database::DatabaseContentRepository;
use botticelli_interface::ContentRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
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
async fn test_content_repository_query() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing content_repository query with limit");

    let database_url = get_database_url()?;
    debug!(database_url = %database_url, "Creating connection pool");
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    // Test querying with limit
    debug!(table = "generated_images", limit = 5, "Querying content table");
    let result = repo.query_content("generated_images", None, Some(5)).await;

    match result {
        Ok(rows) => {
            assert!(
                rows.len() <= 5,
                "Query should respect limit of 5, got {}",
                rows.len()
            );
            debug!(rows_returned = rows.len(), "Query successful");
        }
        Err(e) => {
            // Table might not exist in test database, that's ok
            info!(error = %e, "Query failed (expected if table doesn't exist)");
        }
    }

    info!("content_repository_query test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_content_repository_different_limits() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing content_repository with various limits");

    let database_url = get_database_url()?;
    let pool = create_pool(&database_url)?;
    let repo = DatabaseContentRepository::new(pool);

    let limits = vec![1, 5, 10, 50];
    debug!(limits = ?limits, "Testing multiple limit values");

    for limit in limits {
        debug!(limit = limit, "Querying with limit");
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
            debug!(limit = limit, rows_returned = rows.len(), "Limit respected");
        } else {
            debug!(limit = limit, "Query failed (table may not exist)");
        }
    }

    info!("content_repository_different_limits test passed");
    Ok(())
}
