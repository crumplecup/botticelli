//! Multi-task state persistence tests.

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
async fn test_save_and_load_task_state() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_save_and_load_task_state";
    let state = create_test_state(task_id, "actor1");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save failed");

    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load failed")
        .expect("State not found");

    assert_eq!(loaded.task_id, task_id);
    assert_eq!(loaded.actor_name, "actor1");
    assert_eq!(loaded.consecutive_failures, 0);
    assert!(!loaded.is_paused);
}

#[tokio::test]
async fn test_load_nonexistent_task() {
    let persistence = BotStorageStatePersistence::new(make_storage());

    let loaded = persistence
        .load_task_state("nonexistent_task_xyz")
        .await
        .expect("Load should not error");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_delete_task_state() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_delete_task_state";
    let state = create_test_state(task_id, "actor1");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save failed");

    persistence
        .delete_task_state(task_id)
        .await
        .expect("Delete failed");

    let loaded = persistence
        .load_task_state(task_id)
        .await
        .expect("Load failed");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_list_all_tasks() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_ids = [
        "test_list_all_tasks_1",
        "test_list_all_tasks_2",
        "test_list_all_tasks_3",
    ];

    let state1 = create_test_state(task_ids[0], "actor1");
    let state2 = create_test_state(task_ids[1], "actor2");
    let state3 = create_test_state(task_ids[2], "actor1");

    persistence.save_task_state(task_ids[0], &state1).await.expect("Save failed");
    persistence.save_task_state(task_ids[1], &state2).await.expect("Save failed");
    persistence.save_task_state(task_ids[2], &state3).await.expect("Save failed");

    let tasks = persistence.list_all_tasks().await.expect("List failed");

    assert!(tasks.len() >= 3);
    assert!(tasks.iter().any(|t| t.task_id == task_ids[0]));
    assert!(tasks.iter().any(|t| t.task_id == task_ids[1]));
    assert!(tasks.iter().any(|t| t.task_id == task_ids[2]));
}

#[tokio::test]
async fn test_list_tasks_by_actor() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_ids = [
        "test_list_tasks_by_actor_1",
        "test_list_tasks_by_actor_2",
        "test_list_tasks_by_actor_3",
    ];

    let state1 = create_test_state(task_ids[0], "test_actor_unique_1");
    let state2 = create_test_state(task_ids[1], "test_actor_unique_2");
    let state3 = create_test_state(task_ids[2], "test_actor_unique_1");

    persistence.save_task_state(task_ids[0], &state1).await.expect("Save failed");
    persistence.save_task_state(task_ids[1], &state2).await.expect("Save failed");
    persistence.save_task_state(task_ids[2], &state3).await.expect("Save failed");

    let tasks = persistence
        .list_tasks_by_actor("test_actor_unique_1")
        .await
        .expect("List failed");

    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().all(|t| t.actor_name == "test_actor_unique_1"));
}

#[tokio::test]
async fn test_list_active_and_paused_tasks() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_ids = [
        "test_list_active_paused_1",
        "test_list_active_paused_2",
        "test_list_active_paused_3",
    ];

    let state1 = create_test_state(task_ids[0], "actor1");
    let mut state2 = create_test_state(task_ids[1], "actor2");
    state2.is_paused = true;
    let state3 = create_test_state(task_ids[2], "actor3");

    persistence.save_task_state(task_ids[0], &state1).await.expect("Save failed");
    persistence.save_task_state(task_ids[1], &state2).await.expect("Save failed");
    persistence.save_task_state(task_ids[2], &state3).await.expect("Save failed");

    let active = persistence.list_active_tasks().await.expect("List failed");
    assert!(active.iter().any(|t| t.task_id == task_ids[0]));
    assert!(active.iter().any(|t| t.task_id == task_ids[2]));
    assert!(!active.iter().any(|t| t.task_id == task_ids[1]));

    let paused = persistence.list_paused_tasks().await.expect("List failed");
    assert!(paused.iter().any(|t| t.task_id == task_ids[1]));
}

#[tokio::test]
async fn test_pause_and_resume_task() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_pause_and_resume_task";
    let state = create_test_state(task_id, "actor1");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save failed");

    persistence.pause_task(task_id).await.expect("Pause failed");

    let paused_state = persistence
        .load_task_state(task_id)
        .await
        .expect("Load failed")
        .expect("State not found");
    assert!(paused_state.is_paused);

    persistence.resume_task(task_id).await.expect("Resume failed");

    let resumed_state = persistence
        .load_task_state(task_id)
        .await
        .expect("Load failed")
        .expect("State not found");
    assert!(!resumed_state.is_paused);
}

#[tokio::test]
async fn test_update_next_run() {
    let persistence = BotStorageStatePersistence::new(make_storage());
    let task_id = "test_update_next_run";
    let state = create_test_state(task_id, "actor1");

    persistence
        .save_task_state(task_id, &state)
        .await
        .expect("Save failed");

    let new_next_run = chrono::DateTime::parse_from_rfc3339("2025-12-31T23:59:59Z")
        .unwrap()
        .with_timezone(&Utc);

    persistence
        .update_next_run(task_id, new_next_run)
        .await
        .expect("Update next_run failed");

    let updated = persistence
        .load_task_state(task_id)
        .await
        .expect("Load failed")
        .expect("State not found");

    assert_eq!(updated.next_run.timestamp(), new_next_run.timestamp());
}
