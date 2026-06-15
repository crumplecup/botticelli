use botticelli_actor::BotStorageStatePersistence;
use botticelli_database::RedbStorage;
use botticelli_interface::{ActorServerStateRecord, BotStorage};
use chrono::Utc;
use std::sync::Arc;

fn make_storage() -> Arc<dyn BotStorage> {
    Arc::new(RedbStorage::in_memory().expect("in-memory redb"))
}

fn create_test_state(task_id: &str, actor_name: &str) -> ActorServerStateRecord {
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
async fn test_single_state_save() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "minimal_test_single_save";
    let state = create_test_state(task_id, "test_actor");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("State saved successfully");
}

#[tokio::test]
async fn test_single_state_load() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "minimal_test_single_load";
    let state = create_test_state(task_id, "test_load_actor");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save state");

    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load state")
        .expect("State should exist");

    assert_eq!(loaded.task_id, task_id);
    assert_eq!(loaded.actor_name, "test_load_actor");
}

#[tokio::test]
async fn test_two_task_inserts() {
    let persistence = BotStorageStatePersistence::new(make_storage());

    for i in 0..2 {
        let task_id = format!("minimal_test_task_{}", i);
        let actor_name = format!("actor_{}", i);
        let state = create_test_state(&task_id, &actor_name);
        persistence
            .save_task_state(&task_id, &state)
            .await
            .unwrap_or_else(|e| panic!("Failed to insert task {i}: {e}"));
    }

    for i in 0..2 {
        let task_id = format!("minimal_test_task_{}", i);
        let expected_actor = format!("actor_{}", i);
        let loaded = persistence
            .load_task_state(&task_id)
            .await
            .unwrap_or_else(|e| panic!("Failed to load task {i}: {e}"))
            .unwrap_or_else(|| panic!("Task {i} missing after insert!"));
        assert_eq!(loaded.actor_name, expected_actor);
    }
}

#[tokio::test]
async fn test_two_task_inserts_competing() {
    let persistence = BotStorageStatePersistence::new(make_storage());

    for i in 0..5 {
        let task_id = format!("competing_test_task_{}", i);
        let actor_name = format!("competing_actor_{}", i);
        let state = create_test_state(&task_id, &actor_name);
        persistence
            .save_task_state(&task_id, &state)
            .await
            .unwrap_or_else(|e| panic!("Failed to insert competing task {i}: {e}"));
    }

    for i in 0..5 {
        let task_id = format!("competing_test_task_{}", i);
        let expected_actor = format!("competing_actor_{}", i);
        let loaded = persistence
            .load_task_state(&task_id)
            .await
            .unwrap_or_else(|e| panic!("Failed to load competing task {i}: {e}"))
            .unwrap_or_else(|| panic!("Competing task {i} missing after insert!"));
        assert_eq!(loaded.actor_name, expected_actor);
    }
}
