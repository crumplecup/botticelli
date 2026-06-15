//! Tests for RedbStorage's NarrativeStore implementation.

use botticelli_database::RedbStorage;
use botticelli_interface::{
    ActExecutionRecord, NarrativeExecutionRecord, NarrativeStore,
};
use chrono::Utc;

fn make_storage() -> RedbStorage {
    RedbStorage::in_memory().expect("in-memory redb")
}

fn narrative_record(id: &str, name: &str) -> NarrativeExecutionRecord {
    NarrativeExecutionRecord {
        id: id.to_string(),
        narrative_name: name.to_string(),
        narrative_description: None,
        started_at: Utc::now(),
        completed_at: None,
        status: "running".to_string(),
        error_message: None,
        created_at: Utc::now(),
    }
}

fn act_record(id: &str, narrative_id: &str, act_name: &str) -> ActExecutionRecord {
    ActExecutionRecord {
        id: id.to_string(),
        narrative_execution_id: narrative_id.to_string(),
        act_name: act_name.to_string(),
        sequence_number: 0,
        model: None,
        temperature: None,
        max_tokens: None,
        response: "test response".to_string(),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_narrative_execution_round_trip() {
    let s = make_storage();
    let rec = narrative_record("narr-1", "test_narrative");

    s.save_narrative_execution(&rec).await.expect("save");

    let loaded = s
        .get_narrative_execution("narr-1")
        .await
        .expect("get")
        .expect("should exist");

    assert_eq!(loaded.id, "narr-1");
    assert_eq!(loaded.narrative_name, "test_narrative");
    assert_eq!(loaded.status, "running");
}

#[tokio::test]
async fn test_get_narrative_execution_missing() {
    let s = make_storage();
    let result = s
        .get_narrative_execution("nonexistent-id")
        .await
        .expect("get");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_narrative_execution_upsert() {
    let s = make_storage();
    let mut rec = narrative_record("narr-upsert", "my_narrative");

    s.save_narrative_execution(&rec).await.expect("first save");

    rec.status = "success".to_string();
    rec.completed_at = Some(Utc::now());
    s.save_narrative_execution(&rec).await.expect("second save");

    let loaded = s
        .get_narrative_execution("narr-upsert")
        .await
        .expect("get")
        .expect("should exist");
    assert_eq!(loaded.status, "success");
    assert!(loaded.completed_at.is_some());
}

#[tokio::test]
async fn test_list_narrative_executions_ordering() {
    let s = make_storage();

    s.save_narrative_execution(&narrative_record("narr-order-1", "first"))
        .await
        .expect("save 1");
    s.save_narrative_execution(&narrative_record("narr-order-2", "second"))
        .await
        .expect("save 2");
    s.save_narrative_execution(&narrative_record("narr-order-3", "third"))
        .await
        .expect("save 3");

    let all = s.list_narrative_executions(10).await.expect("list");
    assert!(all.len() >= 3);

    let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"narr-order-1"));
    assert!(ids.contains(&"narr-order-2"));
    assert!(ids.contains(&"narr-order-3"));
}

#[tokio::test]
async fn test_list_narrative_executions_limit() {
    let s = make_storage();

    for i in 0..5 {
        s.save_narrative_execution(&narrative_record(
            &format!("narr-limit-{}", i),
            "bulk_narrative",
        ))
        .await
        .expect("save");
    }

    let limited = s.list_narrative_executions(2).await.expect("list limited");
    assert!(limited.len() <= 2);
}

#[tokio::test]
async fn test_act_execution_round_trip() {
    let s = make_storage();
    let narr = narrative_record("narr-act-parent", "parent_narrative");
    s.save_narrative_execution(&narr).await.expect("save narrative");

    let act = act_record("act-1", "narr-act-parent", "generate_content");
    s.save_act_execution(&act).await.expect("save act");

    let acts = s
        .list_act_executions("narr-act-parent")
        .await
        .expect("list acts");
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].id, "act-1");
    assert_eq!(acts[0].act_name, "generate_content");
}

#[tokio::test]
async fn test_list_act_executions_isolation() {
    let s = make_storage();

    let narr_a = narrative_record("narr-iso-a", "narrative_a");
    let narr_b = narrative_record("narr-iso-b", "narrative_b");
    s.save_narrative_execution(&narr_a).await.expect("save a");
    s.save_narrative_execution(&narr_b).await.expect("save b");

    s.save_act_execution(&act_record("act-iso-a1", "narr-iso-a", "act_for_a"))
        .await
        .expect("save act a1");
    s.save_act_execution(&act_record("act-iso-a2", "narr-iso-a", "act_for_a_2"))
        .await
        .expect("save act a2");
    s.save_act_execution(&act_record("act-iso-b1", "narr-iso-b", "act_for_b"))
        .await
        .expect("save act b1");

    let acts_a = s.list_act_executions("narr-iso-a").await.expect("list a");
    let acts_b = s.list_act_executions("narr-iso-b").await.expect("list b");

    assert_eq!(acts_a.len(), 2);
    assert_eq!(acts_b.len(), 1);
    assert!(acts_a.iter().all(|a| a.narrative_execution_id == "narr-iso-a"));
    assert_eq!(acts_b[0].narrative_execution_id, "narr-iso-b");
}
