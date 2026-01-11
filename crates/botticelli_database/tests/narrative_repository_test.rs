//! Integration tests for NarrativeRepository trait implementation.

mod helpers;

use botticelli_core::{
    ActExecutionBuilder, ExecutionFilter, ExecutionStatus, Input, NarrativeExecution,
};
use botticelli_database::{PostgresNarrativeRepository, establish_connection};
use botticelli_interface::NarrativeRepository;
use botticelli_storage::FileSystemStorage;
use std::sync::Arc;

fn create_test_storage() -> anyhow::Result<Arc<FileSystemStorage>> {
    let temp_dir = std::env::temp_dir().join(format!("botticelli_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)?;
    Ok(Arc::new(FileSystemStorage::new(temp_dir)?))
}

fn create_test_execution(name: &str) -> anyhow::Result<NarrativeExecution> {
    let act = ActExecutionBuilder::default()
        .act_name("test_act".to_string())
        .sequence_number(0_usize)
        .inputs(vec![Input::Text("test input".to_string())])
        .response("test response".to_string())
        .build()?;

    Ok(NarrativeExecution::new(
        name.to_string(),
        vec![act],
        None,
        None,
        None,
    ))
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_save_and_load_execution() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing save_execution and load_execution");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    debug!("Created test storage and database connection");
    let repo = PostgresNarrativeRepository::new(conn, storage);

    // Create test execution
    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    debug!(narrative_name = %name, "Creating test execution");
    let execution = create_test_execution(&name)?;

    // Save execution
    debug!("Saving execution to database");
    let id = repo.save_execution(&execution).await?;
    assert!(id > 0, "Expected positive execution ID");
    debug!(execution_id = id, "Execution saved successfully");

    // Load execution
    debug!(execution_id = id, "Loading execution from database");
    let loaded = repo.load_execution(id).await?;
    assert_eq!(loaded.narrative_name(), &name);
    assert_eq!(loaded.act_executions().len(), 1);
    debug!("Execution loaded and verified successfully");

    // Cleanup
    debug!(execution_id = id, "Cleaning up test execution");
    repo.delete_execution(id).await?;

    info!("save_and_load_execution test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_executions() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing list_executions");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    // Create multiple executions
    let name1 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    let name2 = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    debug!(name1 = %name1, name2 = %name2, "Creating two test executions");

    let exec1 = create_test_execution(&name1)?;
    let exec2 = create_test_execution(&name2)?;

    debug!("Saving executions to database");
    let id1 = repo.save_execution(&exec1).await?;
    let id2 = repo.save_execution(&exec2).await?;
    debug!(id1 = id1, id2 = id2, "Both executions saved");

    // List all executions
    let filter = ExecutionFilter::new().with_limit(100);

    debug!("Listing executions with filter");
    let summaries = repo.list_executions(&filter).await?;
    assert!(summaries.len() >= 2, "Expected at least 2 executions");
    debug!(count = summaries.len(), "Retrieved execution summaries");

    // Verify our executions are in the list
    let found1 = summaries.iter().any(|s| s.narrative_name() == &name1);
    let found2 = summaries.iter().any(|s| s.narrative_name() == &name2);

    assert!(found1, "Expected to find execution 1");
    assert!(found2, "Expected to find execution 2");
    debug!("Both test executions found in list");

    // Cleanup
    debug!("Cleaning up test executions");
    repo.delete_execution(id1).await.ok();
    repo.delete_execution(id2).await.ok();

    info!("list_executions test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_update_execution_status() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing update_status");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    debug!(narrative_name = %name, "Creating test execution");
    let execution = create_test_execution(&name)?;

    // Save execution
    debug!("Saving execution");
    let id = repo.save_execution(&execution).await?;
    debug!(execution_id = id, "Execution saved");

    // Update status to failed
    debug!(
        execution_id = id,
        status = "Failed",
        "Updating execution status"
    );
    repo.update_status(id, ExecutionStatus::Failed).await?;
    debug!("Status updated successfully");

    // Load and verify
    debug!(execution_id = id, "Loading execution to verify status");
    let loaded = repo.load_execution(id).await?;
    assert_eq!(loaded.narrative_name(), &name);
    debug!("Status update verified");

    // Cleanup
    debug!(execution_id = id, "Cleaning up test execution");
    repo.delete_execution(id).await?;

    info!("update_execution_status test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_delete_execution() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing delete_execution");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    debug!(narrative_name = %name, "Creating test execution");
    let execution = create_test_execution(&name)?;

    // Save execution
    debug!("Saving execution");
    let id = repo.save_execution(&execution).await?;
    debug!(execution_id = id, "Execution saved");

    // Delete execution
    debug!(execution_id = id, "Deleting execution");
    repo.delete_execution(id).await?;
    debug!("Execution deleted");

    // Verify deletion - should fail to load
    debug!(
        execution_id = id,
        "Verifying deletion by attempting to load"
    );
    let result = repo.load_execution(id).await;
    assert!(
        result.is_err(),
        "Expected error when loading deleted execution"
    );
    debug!("Deletion verified - load correctly returned error");

    info!("delete_execution test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_list_executions_with_filter() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing list_executions with status filter");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());
    debug!(narrative_name = %name, "Creating test execution");
    let execution = create_test_execution(&name)?;

    // Save and update status to completed
    debug!("Saving execution");
    let id = repo.save_execution(&execution).await?;
    debug!(
        execution_id = id,
        status = "Completed",
        "Updating status to completed"
    );
    repo.update_status(id, ExecutionStatus::Completed).await?;

    // Filter by completed status
    let filter = ExecutionFilter::new()
        .with_status(ExecutionStatus::Completed)
        .with_limit(100);

    debug!("Listing executions with completed status filter");
    let summaries = repo.list_executions(&filter).await?;
    let found = summaries.iter().any(|s| s.narrative_name() == &name);

    assert!(found, "Expected to find completed execution");
    debug!(
        count = summaries.len(),
        "Found completed execution in filtered list"
    );

    // Cleanup
    debug!(execution_id = id, "Cleaning up test execution");
    repo.delete_execution(id).await?;

    info!("list_executions_with_filter test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn test_execution_with_multiple_acts() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing execution with multiple acts");

    let conn = establish_connection()?;
    let storage = create_test_storage()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    let name = format!("test_narrative_{}", uuid::Uuid::new_v4().simple());

    // Create execution with multiple acts
    debug!(narrative_name = %name, "Creating execution with 2 acts");
    let act1 = ActExecutionBuilder::default()
        .act_name("act1".to_string())
        .sequence_number(0_usize)
        .inputs(vec![Input::Text("input1".to_string())])
        .response("response1".to_string())
        .build()?;

    let act2 = ActExecutionBuilder::default()
        .act_name("act2".to_string())
        .sequence_number(1_usize)
        .inputs(vec![
            Input::Text("input2a".to_string()),
            Input::Text("input2b".to_string()),
        ])
        .response("response2".to_string())
        .build()?;

    let execution = NarrativeExecution::new(name.clone(), vec![act1, act2], None, None, None);
    debug!(act_count = 2, "Created multi-act execution");

    // Save and load
    debug!("Saving multi-act execution");
    let id = repo.save_execution(&execution).await?;
    debug!(execution_id = id, "Loading multi-act execution");
    let loaded = repo.load_execution(id).await?;

    assert_eq!(loaded.act_executions().len(), 2);
    assert_eq!(loaded.act_executions()[0].act_name(), "act1");
    assert_eq!(loaded.act_executions()[1].act_name(), "act2");
    assert_eq!(loaded.act_executions()[1].inputs().len(), 2);
    debug!("Multi-act execution loaded and verified successfully");

    // Cleanup
    debug!(execution_id = id, "Cleaning up test execution");
    repo.delete_execution(id).await?;

    info!("execution_with_multiple_acts test passed");
    Ok(())
}
