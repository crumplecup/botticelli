//! Integration tests for NarrativeRepository trait implementation.

use botticelli_database::{PostgresNarrativeRepository, DatabaseResult};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
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
async fn test_create_and_get_narrative() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseNarrativeRepository::new(pool);

    // Create narrative
    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let description = "Test narrative description";
    
    let narrative_id = repo
        .create_narrative(&name, description)
        .await
        .expect("Failed to create narrative");

    assert!(narrative_id > 0, "Expected positive narrative ID");

    // Get narrative by ID
    let narrative = repo
        .get_narrative(narrative_id)
        .await
        .expect("Failed to get narrative")
        .expect("Narrative not found");

    assert_eq!(narrative.name(), &name);
    assert_eq!(narrative.description(), Some(description));

    // Cleanup
    repo.delete_narrative(narrative_id)
        .await
        .expect("Failed to delete narrative");
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_narratives() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresNarrativeRepository::new(pool);

    // Create multiple narratives
    let name1 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let name2 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    
    let id1 = repo
        .create_narrative(&name1, "Description 1")
        .await
        .expect("Failed to create narrative 1");
    
    let id2 = repo
        .create_narrative(&name2, "Description 2")
        .await
        .expect("Failed to create narrative 2");

    // List narratives
    let narratives = repo
        .list_narratives()
        .await
        .expect("Failed to list narratives");

    assert!(narratives.len() >= 2, "Expected at least 2 narratives");
    
    let found1 = narratives.iter().any(|n| n.name() == &name1);
    let found2 = narratives.iter().any(|n| n.name() == &name2);
    
    assert!(found1, "Expected to find narrative 1");
    assert!(found2, "Expected to find narrative 2");

    // Cleanup
    repo.delete_narrative(id1).await.ok();
    repo.delete_narrative(id2).await.ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_update_narrative() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = DatabaseNarrativeRepository::new(pool);

    // Create narrative
    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let narrative_id = repo
        .create_narrative(&name, "Original description")
        .await
        .expect("Failed to create narrative");

    // Update narrative
    let new_description = "Updated description";
    repo.update_narrative(narrative_id, None, Some(new_description))
        .await
        .expect("Failed to update narrative");

    // Verify update
    let narrative = repo
        .get_narrative(narrative_id)
        .await
        .expect("Failed to get narrative")
        .expect("Narrative not found");

    assert_eq!(narrative.description(), Some(new_description));

    // Cleanup
    repo.delete_narrative(narrative_id).await.ok();
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_delete_narrative() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresNarrativeRepository::new(pool);

    // Create narrative
    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let narrative_id = repo
        .create_narrative(&name, "Test description")
        .await
        .expect("Failed to create narrative");

    // Delete narrative
    repo.delete_narrative(narrative_id)
        .await
        .expect("Failed to delete narrative");

    // Verify deletion
    let result = repo.get_narrative(narrative_id).await;
    
    assert!(
        result.is_ok() && result.unwrap().is_none(),
        "Expected narrative to be deleted"
    );
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_get_nonexistent_narrative() {
    let database_url = get_database_url();
    let pool = create_pool(&database_url).expect("Failed to create database pool");
    let repo = PostgresNarrativeRepository::new(pool);

    let result = repo.get_narrative(999999).await;
    
    assert!(
        result.is_ok() && result.unwrap().is_none(),
        "Expected None for nonexistent narrative"
    );
}
