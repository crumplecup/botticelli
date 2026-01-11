mod helpers;

use botticelli_error::{BotticelliResult, ConfigError};
use botticelli_narrative::{NarrativeState, StateManager, StateScope};
use tempfile::TempDir;

#[test]
fn test_state_manager_save_and_load() -> BotticelliResult<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing state manager save and load");
    
    let temp_dir = TempDir::new().map_err(|e| ConfigError::new(e.to_string()))?;
    let manager = StateManager::new(temp_dir.path())?;
    let scope = StateScope::Global;
    tracing::debug!(state_dir = ?temp_dir.path(), "Created state manager");

    // Create and save state
    {
        let mut state = NarrativeState::new();
        state.set("test_key", "test_value");
        manager.save(&scope, &state)?;
        tracing::debug!("Saved state with test_key");
    }

    // Load state and verify
    {
        let state = manager.load(&scope)?;
        assert_eq!(state.get("test_key"), Some("test_value"));
        tracing::debug!("Loaded and verified state");
    }

    tracing::info!("State manager save and load test passed");
    Ok(())
}

#[test]
fn test_state_manager_persistence_across_runs() -> BotticelliResult<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing state persistence across multiple runs");
    
    let temp_dir = TempDir::new().map_err(|e| ConfigError::new(e.to_string()))?;
    let manager = StateManager::new(temp_dir.path())?;
    let scope = StateScope::Narrative("test".to_string());
    tracing::debug!(state_dir = ?temp_dir.path(), scope = "test", "Created state manager");

    // First run: create and save
    {
        let mut state = NarrativeState::new();
        state.set("channel_id", "123456789");
        state.set("message_id", "987654321");
        manager.save(&scope, &state)?;
        tracing::debug!("Run 1: Saved initial state");
    }

    // Second run: load and verify
    {
        let state = manager.load(&scope)?;
        assert_eq!(state.get("channel_id"), Some("123456789"));
        assert_eq!(state.get("message_id"), Some("987654321"));
        tracing::debug!("Run 2: Loaded and verified state");
    }

    // Third run: load, modify, save
    {
        let mut state = manager.load(&scope)?;
        state.set("new_key", "new_value");
        manager.save(&scope, &state)?;
        tracing::debug!("Run 3: Modified and saved state");
    }

    // Fourth run: verify all values
    {
        let state = manager.load(&scope)?;
        assert_eq!(state.get("channel_id"), Some("123456789"));
        assert_eq!(state.get("message_id"), Some("987654321"));
        assert_eq!(state.get("new_key"), Some("new_value"));
        tracing::debug!("Run 4: Verified all keys present");
    }

    tracing::info!("State persistence across runs test passed");
    Ok(())
}

#[test]
fn test_cli_workflow_simulation() -> BotticelliResult<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing CLI workflow simulation");
    
    let temp_dir = TempDir::new().map_err(|e| ConfigError::new(e.to_string()))?;
    let state_dir = temp_dir.path();
    tracing::debug!(state_dir = ?state_dir, "Created temp state directory");

    // Simulate first CLI run with --save flag
    {
        let manager = StateManager::new(state_dir)?;
        let scope = StateScope::Narrative("test_narrative".to_string());

        let mut state = NarrativeState::new();
        state.set("TEST_CHANNEL_ID", "1234567890");
        state.set("TEST_MESSAGE_ID", "0987654321");

        manager.save(&scope, &state)?;
        tracing::debug!("CLI run 1: Saved state with --save flag");

        // Verify it was saved
        let loaded = manager.load(&scope)?;
        assert_eq!(loaded.get("TEST_CHANNEL_ID"), Some("1234567890"));
        tracing::debug!("CLI run 1: Verified saved state");
    }

    // Simulate second CLI run loading state
    {
        let manager = StateManager::new(state_dir)?;
        let scope = StateScope::Narrative("test_narrative".to_string());

        let state = manager.load(&scope)?;
        assert_eq!(state.get("TEST_CHANNEL_ID"), Some("1234567890"));
        assert_eq!(state.get("TEST_MESSAGE_ID"), Some("0987654321"));
        tracing::debug!("CLI run 2: Loaded and verified state from previous run");
    }

    tracing::info!("CLI workflow simulation test passed");
    Ok(())
}
