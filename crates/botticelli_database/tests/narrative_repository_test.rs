//! Integration tests for NarrativeRepository trait implementation.

use botticelli_core::{ActExecutionBuilder, ExecutionFilter, ExecutionStatus, Input, NarrativeExecution};
use botticelli_database::{establish_connection, PostgresNarrativeRepository};
use botticelli_error::{BackendError, BotticelliError, BotticelliResult};
use botticelli_interface::NarrativeRepository;
use botticelli_storage::FileSystemStorage;
use std::sync::Arc;

fn create_test_storage() -> BotticelliResult<Arc<FileSystemStorage>> {
    let temp_dir = std::env::temp_dir().join(format!("botticelli_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        BotticelliError::from(BackendError::new(format!("Failed to create temp dir: {}", e)))
    })?;
    Ok(Arc::new(FileSystemStorage::new(temp_dir)?))
}

fn create_test_execution(name: &str) -> NarrativeExecution {
    let act = ActExecutionBuilder::default()
        .act_name("test_act".to_string())
        .sequence_number(0_usize)
        .inputs(vec![Input::Text("test input".to_string())])
        .response("test response".to_string())
        .build()
        .expect("Valid act");

    NarrativeExecution::new(
        name.to_string(),
        vec![act],
        None,
        None,
        None,
    )
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_save_and_load_execution() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    // Create test execution
    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let execution = create_test_execution(&name);

    // Save execution
    let id = repo.save_execution(&execution).await?;
    assert!(id > 0, "Expected positive execution ID");

    // Load execution
    let loaded = repo.load_execution(id).await?;
    assert_eq!(loaded.narrative_name(), &name);
    assert_eq!(loaded.act_executions().len(), 1);

    // Cleanup
    repo.delete_execution(id).await?;

    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_executions() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    // Create multiple executions
    let name1 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let name2 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());

    let exec1 = create_test_execution(&name1);
    let exec2 = create_test_execution(&name2);

    let id1 = repo.save_execution(&exec1).await?;
    let id2 = repo.save_execution(&exec2).await?;

    // List all executions
    let filter = ExecutionFilter::new().with_limit(100);

    let summaries = repo.list_executions(&filter).await?;
    assert!(summaries.len() >= 2, "Expected at least 2 executions");

    // Verify our executions are in the list
    let found1 = summaries.iter().any(|s| s.narrative_name() == &name1);
    let found2 = summaries.iter().any(|s| s.narrative_name() == &name2);

    assert!(found1, "Expected to find execution 1");
    assert!(found2, "Expected to find execution 2");

    // Cleanup
    repo.delete_execution(id1).await.ok();
    repo.delete_execution(id2).await.ok();

    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_update_execution_status() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let execution = create_test_execution(&name);

    // Save execution
    let id = repo.save_execution(&execution).await?;

    // Update status to failed
    repo.update_status(id, ExecutionStatus::Failed).await?;

    // Load and verify
    let loaded = repo.load_execution(id).await?;
    assert_eq!(loaded.narrative_name(), &name);

    // Cleanup
    repo.delete_execution(id).await?;

    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_delete_execution() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let execution = create_test_execution(&name);

    // Save execution
    let id = repo.save_execution(&execution).await?;

    // Delete execution
    repo.delete_execution(id).await?;

    // Verify deletion - should fail to load
    let result = repo.load_execution(id).await;
    assert!(result.is_err(), "Expected error when loading deleted execution");

    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_executions_with_filter() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let execution = create_test_execution(&name);

    // Save and update status to completed
    let id = repo.save_execution(&execution).await?;
    repo.update_status(id, ExecutionStatus::Completed).await?;

    // Filter by completed status
    let filter = ExecutionFilter::new()
        .with_status(ExecutionStatus::Completed)
        .with_limit(100);

    let summaries = repo.list_executions(&filter).await?;
    let found = summaries.iter().any(|s| s.narrative_name() == &name);

    assert!(found, "Expected to find completed execution");

    // Cleanup
    repo.delete_execution(id).await?;

    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_execution_with_multiple_acts() -> BotticelliResult<()> {
    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());

    // Create execution with multiple acts
    let act1 = ActExecutionBuilder::default()
        .act_name("act1".to_string())
        .sequence_number(0_usize)
        .inputs(vec![Input::Text("input1".to_string())])
        .response("response1".to_string())
        .build()
        .expect("Valid act");

    let act2 = ActExecutionBuilder::default()
        .act_name("act2".to_string())
        .sequence_number(1_usize)
        .inputs(vec![
            Input::Text("input2a".to_string()),
            Input::Text("input2b".to_string()),
        ])
        .response("response2".to_string())
        .build()
        .expect("Valid act");

    let execution = NarrativeExecution::new(
        name.clone(),
        vec![act1, act2],
        None,
        None,
        None,
    );

    // Save and load
    let id = repo.save_execution(&execution).await?;
    let loaded = repo.load_execution(id).await?;

    assert_eq!(loaded.act_executions().len(), 2);
    assert_eq!(loaded.act_executions()[0].act_name(), "act1");
    assert_eq!(loaded.act_executions()[1].act_name(), "act2");
    assert_eq!(loaded.act_executions()[1].inputs().len(), 2);

    // Cleanup
    repo.delete_execution(id).await?;

    Ok(())
}
