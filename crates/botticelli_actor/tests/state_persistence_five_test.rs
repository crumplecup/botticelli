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
async fn test_five_task_inserts() {
    let persistence = BotStorageStatePersistence::new(make_storage());

    // Insert 5 tasks
    for i in 0..5 {
        let task_id = format!("five_test_task_{}", i);
        let actor_name = format!("actor_{}", i);
        let state = create_test_state(&task_id, &actor_name);
        persistence
            .save_task_state(&task_id, &state)
            .await
            .unwrap_or_else(|e| panic!("Failed to insert task {i}: {e}"));
    }

    // Verify all 5 can be loaded
    for i in 0..5 {
        let task_id = format!("five_test_task_{}", i);
        let expected_actor = format!("actor_{}", i);
        let loaded = persistence
            .load_task_state(&task_id)
            .await
            .unwrap_or_else(|e| panic!("Failed to load task {i}: {e}"))
            .unwrap_or_else(|| panic!("Task {i} missing after insert!"));
        assert_eq!(loaded.actor_name, expected_actor);
    }

    // list_all_tasks returns at least 5
    let all = persistence.list_all_tasks().await.expect("List tasks");
    assert!(all.len() >= 5, "Should have at least 5 tasks");
}
