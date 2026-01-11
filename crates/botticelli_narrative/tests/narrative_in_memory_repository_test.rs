//! Tests for in-memory narrative repository.

mod helpers;

use botticelli_core::{ExecutionFilter, NarrativeExecution};
use botticelli_interface::NarrativeRepository;
use botticelli_narrative::InMemoryNarrativeRepository;

#[tokio::test]
async fn test_save_and_list() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing save and list executions in memory");

    let repo = InMemoryNarrativeRepository::new();
    tracing::debug!("Created in-memory repository");

    let execution = NarrativeExecution::new("test".to_string(), vec![], None, None, None);

    let id = repo.save_execution(&execution).await?;
    tracing::debug!(execution_id = ?id, "Saved execution");

    let filter = ExecutionFilter::new()
        .with_narrative_name("test".to_string())
        .with_limit(10);

    let results = repo.list_executions(&filter).await?;
    tracing::debug!(result_count = results.len(), "Listed executions");

    assert_eq!(results.len(), 1);

    tracing::info!("Save and list executions test passed");
    Ok(())
}
