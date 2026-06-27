//! Tests for RedbStorage's ActorStateStore implementation.

use botticelli_database::RedbStorage;
use botticelli_interface::{ActorServerExecutionRecord, ActorServerStateRecord, ActorStateStore};
use chrono::Utc;

fn make_storage() -> RedbStorage {
    RedbStorage::in_memory().expect("in-memory redb")
}

fn state_record(task_id: &str, actor_name: &str) -> ActorServerStateRecord {
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

fn execution_record(id: &str, task_id: &str, actor_name: &str) -> ActorServerExecutionRecord {
    ActorServerExecutionRecord {
        id: id.to_string(),
        task_id: task_id.to_string(),
        actor_name: actor_name.to_string(),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        success: true,
        error_message: None,
        skills_succeeded: 3,
        skills_failed: 0,
        skills_skipped: 1,
        metadata: serde_json::json!({}),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_actor_state_round_trip() {
    let s = make_storage();
    let rec = state_record("task-1", "my_actor");

    s.save_actor_state(&rec).await.expect("save");

    let loaded = s
        .get_actor_state("task-1")
        .await
        .expect("get")
        .expect("should exist");

    assert_eq!(loaded.task_id, "task-1");
    assert_eq!(loaded.actor_name, "my_actor");
    assert_eq!(loaded.consecutive_failures, 0);
    assert!(!loaded.is_paused);
}

#[tokio::test]
async fn test_get_actor_state_missing() {
    let s = make_storage();
    let result = s.get_actor_state("nonexistent-task").await.expect("get");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_actor_state_upsert() {
    let s = make_storage();
    let mut rec = state_record("task-upsert", "my_actor");

    s.save_actor_state(&rec).await.expect("first save");

    rec.consecutive_failures = 2;
    rec.is_paused = true;
    s.save_actor_state(&rec).await.expect("second save");

    let loaded = s
        .get_actor_state("task-upsert")
        .await
        .expect("get")
        .expect("should exist");
    assert_eq!(loaded.consecutive_failures, 2);
    assert!(loaded.is_paused);
}

#[tokio::test]
async fn test_list_actor_states() {
    let s = make_storage();

    s.save_actor_state(&state_record("task-list-1", "actor_a"))
        .await
        .expect("save 1");
    s.save_actor_state(&state_record("task-list-2", "actor_b"))
        .await
        .expect("save 2");
    s.save_actor_state(&state_record("task-list-3", "actor_a"))
        .await
        .expect("save 3");

    let all = s.list_actor_states().await.expect("list");
    assert!(all.len() >= 3);

    let task_ids: Vec<&str> = all.iter().map(|r| r.task_id.as_str()).collect();
    assert!(task_ids.contains(&"task-list-1"));
    assert!(task_ids.contains(&"task-list-2"));
    assert!(task_ids.contains(&"task-list-3"));
}

#[tokio::test]
async fn test_delete_actor_state() {
    let s = make_storage();
    let rec = state_record("task-del", "my_actor");

    s.save_actor_state(&rec).await.expect("save");
    assert!(s.get_actor_state("task-del").await.expect("get").is_some());

    s.delete_actor_state("task-del").await.expect("delete");
    assert!(s.get_actor_state("task-del").await.expect("get").is_none());
}

#[tokio::test]
async fn test_actor_execution_round_trip() {
    let s = make_storage();
    let state = state_record("task-exec-parent", "my_actor");
    s.save_actor_state(&state).await.expect("save state");

    let exec = execution_record("exec-1", "task-exec-parent", "my_actor");
    s.save_actor_execution(&exec).await.expect("save exec");

    let execs = s
        .list_actor_executions("task-exec-parent", 10)
        .await
        .expect("list");
    assert_eq!(execs.len(), 1);
    assert_eq!(execs[0].id, "exec-1");
    assert!(execs[0].success);
    assert_eq!(execs[0].skills_succeeded, 3);
}

#[tokio::test]
async fn test_list_actor_executions_isolation() {
    let s = make_storage();

    s.save_actor_state(&state_record("task-iso-a", "actor_a"))
        .await
        .expect("save");
    s.save_actor_state(&state_record("task-iso-b", "actor_b"))
        .await
        .expect("save");

    s.save_actor_execution(&execution_record("exec-iso-a1", "task-iso-a", "actor_a"))
        .await
        .expect("save exec a1");
    s.save_actor_execution(&execution_record("exec-iso-a2", "task-iso-a", "actor_a"))
        .await
        .expect("save exec a2");
    s.save_actor_execution(&execution_record("exec-iso-b1", "task-iso-b", "actor_b"))
        .await
        .expect("save exec b1");

    let execs_a = s
        .list_actor_executions("task-iso-a", 10)
        .await
        .expect("list a");
    let execs_b = s
        .list_actor_executions("task-iso-b", 10)
        .await
        .expect("list b");

    assert_eq!(execs_a.len(), 2);
    assert_eq!(execs_b.len(), 1);
    assert!(execs_a.iter().all(|e| e.task_id == "task-iso-a"));
    assert_eq!(execs_b[0].task_id, "task-iso-b");
}

#[tokio::test]
async fn test_list_all_actor_executions() {
    let s = make_storage();

    s.save_actor_state(&state_record("task-all-1", "actor_a"))
        .await
        .expect("save state 1");
    s.save_actor_state(&state_record("task-all-2", "actor_b"))
        .await
        .expect("save state 2");

    s.save_actor_execution(&execution_record("exec-all-1", "task-all-1", "actor_a"))
        .await
        .expect("save exec 1");
    s.save_actor_execution(&execution_record("exec-all-2", "task-all-2", "actor_b"))
        .await
        .expect("save exec 2");

    let all = s.list_all_actor_executions(10).await.expect("list all");
    assert!(all.len() >= 2);

    let ids: Vec<&str> = all.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&"exec-all-1"));
    assert!(ids.contains(&"exec-all-2"));
}

#[tokio::test]
async fn test_delete_actor_execution() {
    let s = make_storage();

    s.save_actor_state(&state_record("task-exec-del", "my_actor"))
        .await
        .expect("save state");
    s.save_actor_execution(&execution_record("exec-del-1", "task-exec-del", "my_actor"))
        .await
        .expect("save exec");

    let before = s
        .list_actor_executions("task-exec-del", 10)
        .await
        .expect("list");
    assert_eq!(before.len(), 1);

    s.delete_actor_execution("exec-del-1")
        .await
        .expect("delete");

    let after = s
        .list_actor_executions("task-exec-del", 10)
        .await
        .expect("list");
    assert!(after.is_empty());
}
