//! Tests for circuit breaker functionality in BotStorageStatePersistence.

use botticelli_actor::{BotStorageStatePersistence, DatabaseExecutionResult};
use botticelli_database::RedbStorage;
use botticelli_interface::{ActorServerStateRecord, BotStorage};
use chrono::Utc;
use std::sync::Arc;

fn make_storage() -> Arc<dyn BotStorage> {
    Arc::new(RedbStorage::in_memory().expect("in-memory redb"))
}

fn setup_test_task(task_id: &str, actor_name: &str) -> ActorServerStateRecord {
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
async fn test_record_failure_increments_counter() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_failure_counter";
    let state = setup_test_task(task_id, "test_actor");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save initial state");

    // Record first failure
    let threshold_exceeded = persistence
        .record_failure(task_id, 3)
        .await
        .expect("Record failure");
    assert!(!threshold_exceeded, "Should not exceed threshold yet");

    // Verify consecutive_failures incremented
    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load state")
        .expect("State exists");
    assert_eq!(loaded.consecutive_failures, 1);
    assert!(!loaded.is_paused, "Should not be paused yet");
}

#[tokio::test]
async fn test_record_failure_triggers_circuit_breaker() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_circuit_breaker";
    let state = setup_test_task(task_id, "test_actor");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save initial state");

    // Record 3 failures (max = 3)
    for i in 1..=3 {
        let result = persistence
            .record_failure(task_id, 3)
            .await
            .expect("Record failure");
        if i < 3 {
            assert!(!result, "Should not trigger until threshold");
        } else {
            assert!(result, "Should trigger at threshold");
        }
    }

    // Verify task is paused
    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load state")
        .expect("State exists");
    assert!(loaded.is_paused, "Task should be paused after circuit breaker");
    assert_eq!(loaded.consecutive_failures, 3);
}

#[tokio::test]
async fn test_record_success_resets_counter() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_success_reset";
    let state = ActorServerStateRecord {
        task_id: task_id.to_string(),
        actor_name: "test_actor".to_string(),
        last_run: None,
        next_run: Utc::now(),
        consecutive_failures: 5,
        is_paused: false,
        metadata: serde_json::json!({}),
        updated_at: Utc::now(),
    };

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save initial state");

    persistence
        .record_success(task_id)
        .await
        .expect("Record success");

    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load state")
        .expect("State exists");
    assert_eq!(loaded.consecutive_failures, 0, "Failures should be reset");
}

#[tokio::test]
async fn test_should_execute_respects_pause() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_should_execute";

    // No state → should execute
    let result = persistence.should_execute(task_id).await.expect("Check");
    assert!(result, "Should execute when no state exists");

    // Active state → should execute
    let active = setup_test_task(task_id, "actor");
    persistence
        .save_task_state(task_id, &active)
        .await
        .expect("Save");
    let result = persistence.should_execute(task_id).await.expect("Check");
    assert!(result, "Should execute when active");

    // Paused → should not execute
    persistence.pause_task(task_id).await.expect("Pause");
    let result = persistence.should_execute(task_id).await.expect("Check");
    assert!(!result, "Should not execute when paused");

    // Resumed → should execute again
    persistence.resume_task(task_id).await.expect("Resume");
    let result = persistence.should_execute(task_id).await.expect("Check");
    assert!(result, "Should execute after resume");
}

#[tokio::test]
async fn test_start_and_complete_execution() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_exec_lifecycle";
    let state = setup_test_task(task_id, "exec_actor");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save state");

    let exec_id = persistence
        .start_execution(task_id, "exec_actor")
        .await
        .expect("Start execution");
    assert!(!exec_id.is_empty(), "Execution ID should be non-empty");

    let result = DatabaseExecutionResult {
        skills_succeeded: 2,
        skills_failed: 1,
        skills_skipped: 0,
        metadata: serde_json::json!({"notes": "test"}),
    };

    persistence
        .complete_execution(&exec_id, task_id, result)
        .await
        .expect("Complete execution");

    let history = persistence
        .get_execution_history(task_id, 10)
        .await
        .expect("Get history");
    assert_eq!(history.len(), 1, "Should have one execution record");
    assert!(history[0].success, "Execution should be marked successful");
    assert_eq!(history[0].skills_succeeded, 2);
}
