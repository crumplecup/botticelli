//! Tests for in-memory narrative repository.

use botticelli_interface::{ExecutionFilter, NarrativeExecution, NarrativeRepository};
use botticelli_narrative::InMemoryNarrativeRepository;

#[tokio::test]
async fn test_save_and_list() {
    let repo = InMemoryNarrativeRepository::new();

    let execution = NarrativeExecution {
        narrative_name: "test".to_string(),
        act_executions: vec![],
        total_token_usage: None,
        total_cost_usd: None,
        total_duration_ms: None,
    };

    let _id = repo.save_execution(&execution).await.unwrap();

    let filter = ExecutionFilter {
        narrative_name: Some("test".to_string()),
        status: None,
        limit: Some(10),
        offset: None,
    };

    let results = repo.list_executions(&filter).await.unwrap();
    assert_eq!(results.len(), 1);
}
