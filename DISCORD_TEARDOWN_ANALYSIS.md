# Discord Write Operations Teardown Analysis

## Current Issue

The `test_write_operations_with_teardown` test is failing because teardown narratives cannot find dynamically created resource IDs.

## Environment Variable Support

**Botticelli DOES support environment variable expansion:**
- Uses `shellexpand` crate for `${VAR_NAME}` and `$VAR_NAME` syntax
- Variables loaded from `.env` file or shell environment
- Expansion happens at narrative parse time
- Works for bot command arguments

**However, for test resources we need state management:**
- Test resources (channels, threads, roles) are created dynamically
- Their IDs don't exist until runtime
- IDs change between test runs
- Environment variables are static and not suitable for dynamic values

## Test Flow

1. **Setup** - Create test channel and store ID in **state management**
2. **Operations** - Perform write operations using IDs from state
3. **Teardown** - Delete created resources using IDs retrieved from state

## Potential Problems

### 1. State Management Issues
- IDs might not be persisted correctly between narratives
- State file path might be wrong
- State might not be loading properly in teardown narrative

### 2. Resource Deletion Order
- Some resources depend on others (threads depend on channels)
- Deletion order matters - need to delete in reverse dependency order

### 3. Error Handling
- If setup fails, teardown still runs but has nothing to clean up
- If operation fails, some resources might exist, others might not
- Teardown should handle missing resources gracefully

### 4. Discord API Constraints
- Rate limiting might affect rapid create/delete operations
- Permissions might prevent deletion
- Resources might not be immediately available for deletion after creation

## Root Cause Found (Test Run Evidence)

The state is NOT persisting between narratives! The test output shows:
- Setup runs with `--save` flag and `--state-dir /tmp/botticelli_test_state`
- Test runs with same flags and tries to read state  
- Error: "State key 'channel_id' not found. Available keys: none"

This means the state file isn't being written or isn't being read correctly between narrative executions.

## Testing Strategy

### Step 1: Verify State Persistence
Run setup and check state file manually:
```bash
cargo test test_write_operations_setup --features discord -- --nocapture
cat ~/.botticelli/state/narrative_state.json
```

### Step 2: Test Operations Separately
Test each operation individually to see which ones work:
- Message sending
- Thread creation
- Role creation (if implemented)

### Step 3: Test Teardown in Isolation
Run teardown with known good IDs to verify deletion works:
```bash
# Manually create a test channel via Discord
# Add its ID to state
# Run teardown
cargo test test_write_operations_teardown --features discord -- --nocapture
```

### Step 4: Check Error Messages
Look at the actual error from teardown to understand what's failing:
- Is it finding the state?
- Is it parsing the IDs correctly?
- Is the Discord API call failing?

## Findings

### Issue Identified
The setup narrative (`channel_create_setup.toml`) is NOT saving the channel_id to state.

Error message:
```
State key 'channel_id' not found. Available keys: none
```

This means:
1. Setup runs but doesn't persist the created channel ID
2. Update/teardown fail because they can't find the channel_id
3. State is empty ("Available keys: none")

### Root Cause
The setup narrative needs to use state management actions to save the channel ID after creation.

## Next Steps

1. ✅ Identified the problem - setup not saving state
2. ✅ Confirmed `--save` flag is passed and state dir is created
3. ✅ Verified state shows "Available keys: none" - setup is NOT saving
4. ✅ **PROGRESS**: Basic file I/O state persistence works (test_state_persistence_basic passes)
5. **CURRENT**: Test state persistence through CLI narrative execution
6. Fix setup narrative channel_create_setup.toml to save channel_id to state
7. Verify state persists between narratives
8. Re-run full test to confirm fix

## Investigation Results

Test output shows:
- Setup runs but doesn't save state
- Test narrative fails: "State key 'channel_id' not found. Available keys: none"
- This confirms setup is not properly configured to save to state

Need to check: Does channel_create_setup.toml have proper output/state configuration?

## Resolution Plan

Once we identify the issue:
1. Fix the root cause (state persistence, error handling, etc.)
2. Add logging/tracing to make future debugging easier
3. Consider adding a helper function for safe resource cleanup
4. Document the teardown pattern in TESTING_PATTERNS.md
