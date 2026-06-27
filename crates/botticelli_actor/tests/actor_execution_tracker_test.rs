//! Tests for actor execution tracker integration

use botticelli_actor::{
    ActorExecutionTracker, BotStorageStatePersistence, DatabaseExecutionResult,
};
use botticelli_database::RedbStorage;
use botticelli_interface::{ActorServerStateRecord, BotStorage};
use chrono::Utc;
use std::sync::Arc;

fn make_storage() -> Arc<dyn BotStorage> {
    Arc::new(RedbStorage::in_memory().expect("in-memory redb"))
}

fn create_state(task_id: &str, actor_name: &str) -> ActorServerStateRecord {
    ActorServerStateRecord {
        task_id: task_id.to_string(),
        actor_name: actor_name.to_string(),
        last_run: None,
        next_run: Utc::now(),
        consecutive_failures: 0,
        is_paused: false,
        metadata: serde_json::json!({}),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_execution_tracker_lifecycle() {
    let persistence = Arc::new(BotStorageStatePersistence::new(make_storage()));
    let task_id = format!(
        "test-tracker-{}-{}",
        Utc::now().timestamp_millis(),
        std::process::id()
    );
    let actor_name = "test-actor";

    // Setup: Create initial state
    persistence
        .save_task_state(&task_id, &create_state(&task_id, actor_name))
        .await
        .expect("Insert state");

    let tracker =
        ActorExecutionTracker::new(persistence.clone(), task_id.clone(), actor_name.to_string());

    // Should execute initially
    assert!(
        tracker.should_execute().await.expect("Check execution"),
        "Task should be executable initially"
    );

    // Start execution
    let exec_id = tracker.start_execution().await.expect("Start execution");
    assert!(!exec_id.is_empty(), "Execution ID should be non-empty");

    // Record success
    let result = DatabaseExecutionResult {
        skills_succeeded: 3,
        skills_failed: 0,
        skills_skipped: 1,
        metadata: serde_json::json!({"test": "success"}),
    };

    tracker
        .record_success(exec_id, result)
        .await
        .expect("Record success");

    // Update next run
    let next_run = Utc::now() + chrono::Duration::seconds(60);
    tracker
        .update_next_run(next_run)
        .await
        .expect("Update next run");

    // Cleanup
    persistence
        .delete_task_state(&task_id)
        .await
        .expect("Cleanup");
}

#[tokio::test]
async fn test_execution_tracker_circuit_breaker() {
    let persistence = Arc::new(BotStorageStatePersistence::new(make_storage()));
    let task_id = format!(
        "test-circuit-{}-{}",
        Utc::now().timestamp_millis(),
        std::process::id()
    );
    let actor_name = "failing-actor";

    // Setup with max_failures = 3 in metadata
    let state = ActorServerStateRecord {
        task_id: task_id.clone(),
        actor_name: actor_name.to_string(),
        last_run: None,
        next_run: Utc::now(),
        consecutive_failures: 0,
        is_paused: false,
        metadata: serde_json::json!({"max_failures": 3}),
        updated_at: Utc::now(),
    };
    persistence
        .save_task_state(&task_id, &state)
        .await
        .expect("Insert state");

    let tracker =
        ActorExecutionTracker::new(persistence.clone(), task_id.clone(), actor_name.to_string());

    // Record 3 failures
    for i in 1..=3 {
        let exec_id = tracker.start_execution().await.expect("Start execution");
        let should_pause = tracker
            .record_failure(exec_id, &format!("Error {}", i))
            .await
            .expect("Record failure");

        if i < 3 {
            assert!(!should_pause, "Should not pause until threshold exceeded");
        } else {
            assert!(should_pause, "Should pause after 3 failures");
        }
    }

    // Task should now be paused
    assert!(
        !tracker.should_execute().await.expect("Check execution"),
        "Task should be paused after circuit breaker"
    );

    // Cleanup
    persistence
        .delete_task_state(&task_id)
        .await
        .expect("Cleanup");
}

#[tokio::test]
async fn test_execution_tracker_accessors() {
    let storage = make_storage();
    let persistence = Arc::new(BotStorageStatePersistence::new(storage));
    let task_id = "test-task".to_string();
    let actor_name = "test-actor".to_string();

    let tracker =
        ActorExecutionTracker::new(persistence.clone(), task_id.clone(), actor_name.clone());

    assert_eq!(tracker.task_id(), "test-task");
    assert_eq!(tracker.actor_name(), "test-actor");
    assert!(Arc::ptr_eq(tracker.persistence(), &persistence));
}
