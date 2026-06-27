//! Integration tests for task scheduler with database persistence.

use botticelli_actor::{BotStorageStatePersistence, SimpleTaskScheduler};
use botticelli_database::RedbStorage;
use botticelli_interface::BotStorage;
use botticelli_server::{ActorServerResult, TaskScheduler};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_scheduler_with_persistence() -> ActorServerResult<()> {
    let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::in_memory().expect("in-memory redb"));
    let persistence = BotStorageStatePersistence::new(storage);
    let mut scheduler = SimpleTaskScheduler::with_persistence(persistence);

    assert!(scheduler.has_persistence());

    // Schedule a simple task
    scheduler
        .schedule("test_task".to_string(), Duration::from_secs(10), || async {
            Ok(())
        })
        .await?;

    assert!(scheduler.is_scheduled("test_task"));

    // Cancel the task
    scheduler.cancel("test_task").await?;

    assert!(!scheduler.is_scheduled("test_task"));

    Ok(())
}

#[tokio::test]
async fn test_scheduler_without_persistence() -> ActorServerResult<()> {
    let mut scheduler = SimpleTaskScheduler::new();

    assert!(!scheduler.has_persistence());

    // Schedule a simple task
    scheduler
        .schedule("test_task".to_string(), Duration::from_secs(10), || async {
            Ok(())
        })
        .await?;

    assert!(scheduler.is_scheduled("test_task"));

    // Cancel the task
    scheduler.cancel("test_task").await?;

    Ok(())
}

#[tokio::test]
async fn test_scheduler_task_recovery() -> ActorServerResult<()> {
    let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::in_memory().expect("in-memory redb"));
    let persistence = BotStorageStatePersistence::new(storage);
    let scheduler = SimpleTaskScheduler::with_persistence(persistence);

    // Attempt recovery (should handle empty state gracefully)
    let recovered = scheduler.recover_tasks().await?;

    assert_eq!(recovered.len(), 0);

    Ok(())
}
