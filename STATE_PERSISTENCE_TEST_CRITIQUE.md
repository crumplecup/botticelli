# State Persistence Test Critique

## Current Test Pattern Analysis

### The Failing Test
```rust
#[test]
fn test_channel_update() {
    WriteOperationTest::new(
        narrative_path("write_tests/channel_create_setup"),
        narrative_path("write_tests/channel_update_test"),
    )
    .with_teardown(narrative_path("write_tests/channel_create_teardown"))
    .run()
    .expect("Channel update test failed");
}
```

### Test Flow
1. **Setup**: Creates a test channel and stores ID in state
2. **Test**: Updates the channel using stored ID
3. **Teardown**: Deletes the channel using stored ID

## Issues Identified

### 1. **State Directory Management**
**Problem**: Each narrative execution uses the same temp directory but may not be reading/writing state correctly.

**Evidence**:
```rust
let state_dir = std::env::temp_dir().join("botticelli_test_state");
```

**Why this could fail**:
- State may not be flushing to disk between narrative runs
- The `--save` flag behavior may not be working as expected
- State might be loading from wrong location

### 2. **Lack of State Verification**
**Problem**: No verification that state was actually saved after setup.

**Missing**:
- Check that state file exists after setup
- Check that state contains expected keys
- Log state contents for debugging

### 3. **Error Messages Are Vague**
**Problem**: When teardown fails with "channel not found", we don't know if:
- Setup never created the channel
- Setup created but didn't save state
- Test consumed/deleted the channel
- Teardown can't read state

### 4. **No Isolation Between Test Runs**
**Problem**: Shared temp directory means:
- Previous test runs might leave stale state
- Tests might interfere with each other
- Hard to reproduce failures

## Root Cause Hypothesis

The most likely issue: **State is not persisting between narrative executions**.

### Why?
1. StateManager.save() might not be called automatically
2. The `--save` flag might not trigger state persistence
3. State might be in-memory only during narrative execution
4. CLI might not be passing state_dir correctly to executor

## Recommended Fixes

### Fix 1: Add State Verification
```rust
fn run_narrative(&self, path: &PathBuf, stage: &str) -> TestResult {
    let state_dir = std::env::temp_dir().join("botticelli_test_state");
    std::fs::create_dir_all(&state_dir)?;
    
    // Run narrative
    let output = Command::new("cargo")...;
    
    // Verify state file exists after setup
    if stage == "Setup" {
        let state_file = state_dir.join("state.json");
        if !state_file.exists() {
            return Err("State file not created after setup".into());
        }
        
        // Read and log state
        let state_content = std::fs::read_to_string(&state_file)?;
        println!("State after {}: {}", stage, state_content);
    }
    
    Ok(())
}
```

### Fix 2: Use Unique State Directories Per Test
```rust
pub fn new(setup_narrative: impl Into<PathBuf>, test_narrative: impl Into<PathBuf>) -> Self {
    let test_id = uuid::Uuid::new_v4();
    Self {
        setup_narrative: setup_narrative.into(),
        test_narrative: test_narrative.into(),
        teardown_narrative: None,
        state_dir: std::env::temp_dir().join(format!("botticelli_test_{}", test_id)),
    }
}
```

### Fix 3: Add Explicit State Save/Load Tests
Create unit tests that ONLY test state management:
```rust
#[test]
fn test_state_save_and_load() {
    let state_dir = temp_dir();
    let state_manager = StateManager::new(&state_dir).unwrap();
    
    // Save state
    state_manager.set("TEST_KEY", "test_value")?;
    state_manager.save()?;
    
    // Verify file exists
    assert!(state_dir.join("state.json").exists());
    
    // Load in new instance
    let state_manager2 = StateManager::new(&state_dir).unwrap();
    assert_eq!(state_manager2.get("TEST_KEY")?, Some("test_value"));
}
```

### Fix 4: Check CLI State Handling
Verify the CLI actually:
1. Accepts `--state-dir` argument
2. Passes it to NarrativeExecutor
3. Calls save() when `--save` flag is present

## Next Steps

1. Add state verification logging to test helper
2. Run test with verbose output to see state contents
3. Add unit test for StateManager save/load
4. Verify CLI state-dir handling
5. Use unique state dirs per test run

## Success Criteria

Test passes when:
- Setup creates channel and saves ID to state
- Test reads ID from state and updates channel
- Teardown reads ID from state and deletes channel
- All state operations are logged and verifiable
