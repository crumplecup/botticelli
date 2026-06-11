//! Tests for in-memory narrative repository.

use botticelli_interface::{ExecutionFilter, NarrativeExecution, NarrativeRepository};
use botticelli_narrative::InMemoryNarrativeRepository;

#[tokio::test]
async fn test_save_and_list() {
    let repo = InMemoryNarrativeRepository::new();

    let execution = NarrativeExecution::new("test".to_string(), vec![], None, None, None);

    let _id = repo.save_execution(&execution).await.unwrap();

    let filter = ExecutionFilter::new()
        .with_narrative_name("test")
        .with_limit(10);

    let results = repo.list_executions(&filter).await.unwrap();
    assert_eq!(results.len(), 1);
}
