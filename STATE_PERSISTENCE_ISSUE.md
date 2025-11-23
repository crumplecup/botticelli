# State Persistence Issue

**STATUS: ROOT CAUSE FOUND** - The `just narrate` recipe doesn't pass `--save` or `--state-dir` flags to the CLI.

## Latest Finding (2025-11-23)

Test successfully created Discord channel (ID: 1441994324920373333), but no state file was created even though `--save` was supposed to be passed via `just narrate`. 

The problem: The `justfile`'s `narrate` recipe doesn't support `--save` or `--state-dir` flags.

## Problem (Historical)

State is not persisting between narrative executions, even when using `--state-dir` flag.

### Observed Behavior

1. Setup narrative (`channel_create_setup.toml`) creates a channel via Discord bot command
2. Bot command executor captures the `channel_id` and saves it to state
3. Test narrative (`channel_update_test.toml`) runs immediately after
4. Test narrative tries to reference `${state:channel_id}`
5. **State key 'channel_id' not found. Available keys: none**

### Test Output

```
Stderr: Error: BotticelliError(Narrative(NarrativeError { 
  kind: TemplateError("State key 'channel_id' not found. Available keys: none"), 
  line: 900, 
  file: "crates/botticelli_narrative/src/executor.rs" 
}))
```

## Investigation

### State Manager Implementation

State manager exists in `botticelli_narrative/src/state.rs`:
- `load(&self, scope: &StateScope)` - loads state from disk
- `save(&self, scope: &StateScope, state: &NarrativeState)` - saves state to disk
- Uses JSON files in `state_dir` for persistence

### Executor Integration

In `executor.rs`:
- Line 170: State is loaded at start of `capture_bot_command_ids()`
- Line 209-210: IDs are saved to state (both long and short keys)
- Line 225: State is saved back to disk if IDs were captured
- Line 575: `capture_bot_command_ids()` is called after bot command execution

### CLI Integration

In `botticelli/src/cli/run.rs`:
- Lines 99-105: State manager is configured when `--state-dir` is provided
- State manager is passed to executor via `with_state_manager()`

## ROOT CAUSE IDENTIFIED

**The `state_capture` feature is NOT IMPLEMENTED in the executor.**

The narratives declare state_capture configurations like:

```toml
[acts.create_channel.state_capture]
channel_id = "$.channel_id"
```

But grep for "state_capture" in crates/botticelli_narrative/src returns **NO matches**.

The executor has NO code to:
1. Parse `state_capture` from the TOML ActConfig
2. Extract values from bot command outputs using JSONPath expressions
3. Store those values in the StateManager

**Evidence:**
- Setup narrative runs but warning shows "State file does not exist after Setup"
- Test narrative error: "State key 'channel_id' not found. Available keys: none"
- No state_capture parsing or handling code exists in executor

## Implementation Plan

### 1. Add state_capture to ActConfig

In `crates/botticelli_narrative/src/models/act.rs`:
- Add `state_capture: Option<HashMap<String, String>>` field to ActConfig
- Update ActConfigBuilder to support state_capture

### 2. Parse state_capture from TOML

In `crates/botticelli_narrative/src/core.rs`:
- Extract state_capture from TOML act table
- Add to ActConfig during parsing

### 3. Implement state capture in executor

In `crates/botticelli_narrative/src/executor.rs`:
- After bot command execution, check if act has state_capture config
- Parse bot command JSON output
- Use JSONPath expressions to extract values (add `jsonpath_lib` dependency)
- Store extracted values in StateManager with specified keys
- Save state to disk

### 4. Update NARRATIVE_TOML_SPEC

Document the state_capture feature with examples.

## Investigation Cycle 1

Added debug tracing to state manager to see what's happening. Need to run tests and capture logs.

## Investigation Cycle 2

Running with RUST_LOG=debug to see state operations:
```bash
cargo test --features local test_discord_write_operations_with_teardown -- --nocapture
```

Waiting for compilation... (this is fine, making tea)

## Investigation Cycle 3 - FOUND THE ISSUE!

Test output shows:
```
2025-11-23T01:51:39.169426Z  INFO Configuring state manager state_dir=/tmp/botticelli_test_state
2025-11-23T01:51:39.169486Z  INFO State manager configured
2025-11-23T01:51:39.169499Z  INFO Executing narrative
Error: TemplateError("State key 'channel_id' not found. Available keys: none")
```

**Key observation**: State manager IS being configured with `/tmp/botticelli_test_state`, but when template substitution runs, it finds NO available keys!

This means:
1. ✅ State directory is configured correctly
2. ✅ State manager is initialized
3. ❌ State is EMPTY when template tries to substitute `${state:channel_id}`

**Root Cause**: The setup narrative CREATED the channel and presumably saved the ID, but the test narrative is loading empty state. This suggests:

**The state is not being LOADED from disk at the start of narrative execution!**

Looking at executor.rs line 900 (where error occurs) - this is in template substitution. The executor needs to load state BEFORE template substitution happens, not just when bot commands run.

## Investigation Cycle 4 - Checking State Save

Code shows (executor.rs:224-229):
- IDs ARE being captured to state
- State IS being saved if captured_count > 0
- Uses `StateScope::Global`

Need to verify:
1. Is `capture_bot_command_ids()` actually being called after channel creation?
2. Is it finding the `channel_id` in the output?
3. Is the file `/tmp/botticelli_test_state/global.json` created after setup?

Let me manually check if the state file exists after running setup narrative...

## Investigation Cycle 5 - Bot Registry Not Configured!

When running the setup narrative manually:
```
WARN Discord processing requires database feature
ERROR Bot command 'channels.create' requires bot_registry to be configured
```

**AH HA!** The test helper is not passing `--process-discord` flag OR the bot registry is not being initialized!

Looking at the test output from cycle 3, I see:
```
INFO Registering bot command executor platform=discord commands=62
INFO Discord bot registry configured
```

So in the TEST it IS configured. But when running manually, it's not. The difference must be in how the test helper configures the executor vs how the CLI does it.

## Investigation Cycle 6 - FOUND IT!!!

Setup narrative runs successfully:
```
INFO Successfully created channel channel_id=1441970520919904349 name="botticelli-write-test"
INFO Narrative execution completed acts_completed=1
```

But checking the state directory:
```
=== STATE FILE ===
No state file found
```

**THE BUG**: The `state_capture` configuration in the TOML is NOT being processed!

The narrative has:
```toml
[acts.create_channel.state_capture]
channel_id = "$.channel_id"
```

But this is never executed. The state file is never created even though the bot command returned a channel_id.

Looking at executor.rs:162-235 (`capture_bot_command_ids`), this function captures IDs from bot command outputs.  But it's only called AFTER the act is processed. The question is: does the act configuration have the state_capture info available to the executor?

## Solution

The `state_capture` configuration in TOML is being parsed but NOT used by the executor!

**What needs to happen:**
1. Parse `state_capture` from TOML (may already exist)
2. Pass state_capture configuration to executor  
3. After bot command executes, use state_capture JSONPath queries to extract values
4. Save extracted values to state with configured keys
5. Persist state to disk

**Files to modify:**
- `crates/botticelli_narrative/src/core.rs` - Parse state_capture from TOML
- `crates/botticelli_narrative/src/executor.rs` - Use state_capture to extract and save values

## Implementation Status

`state_capture` is mentioned in NARRATIVE_TOML_SPEC and used in test narratives, but **NOT IMPLEMENTED** in the code!

**Next steps:**
1. Add `state_capture` field to Act/ActConfig structs
2. Parse `state_capture` from TOML (HashMap<String, String> where key=state_key, value=JSONPath)
3. In executor, after bot command executes, apply JSONPath queries to output
4. Save extracted values to state manager
5. Update NARRATIVE_TOML_SPEC if syntax needs clarification

## Related Code

- `crates/botticelli_narrative/src/state.rs` - State manager implementation
- `crates/botticelli_narrative/src/executor.rs:162-235` - ID capture logic
- `crates/botticelli_narrative/src/executor.rs:575` - ID capture invocation
- `crates/botticelli/src/cli/run.rs:99-105` - State manager configuration
- `crates/botticelli_social/tests/narratives/discord/write_tests/` - Test narratives

## Test Results (2025-11-22)

### Basic State Persistence Test
✅ **PASSED** - Basic file I/O for state works correctly
- Can write state to JSON file
- Can read state back from file
- File operations are reliable

### Remaining Issues

1. **CLI Integration incomplete**
   - Test with `--save` flag exists but is `#[ignore]`d
   - Need to verify full CLI workflow with state persistence

2. **End-to-end setup/teardown flow not tested**
   - Setup narrative creates resource and saves ID
   - Teardown narrative should load state and clean up
   - This full cycle needs integration test

### Next Steps

Create end-to-end integration test:
1. Run setup narrative with `--save --state-dir`
2. Verify state file contains expected ID
3. Run separate narrative that loads state
4. Verify state was loaded correctly
5. Run teardown to clean up


## Investigation Cycle 7 - State File Isolation Issue (2025-11-23)

Test failed with:
```
Error: State key 'channel_id' not found. Available keys: none
```

**Root Cause Discovered**: Each narrative uses its own state file based on narrative name!

State file naming pattern: `.narrative_state/{narrative_name}.json`

Example of the problem:
- `state_test_create.toml` → `.narrative_state/state_test_create.json`
- `state_test_use.toml` → `.narrative_state/state_test_use.json` ← Can't see create's state!

The narratives are isolated from each other by design, but tests need shared state.

## Solution: Add --state-name Flag

Add `--state-name` CLI flag to specify custom state file name:
- Default: Use narrative name for state file (current behavior)
- With flag: Use specified name for state file
- Tests use: `--state-name test_state` to share state

Implementation:
1. Add `--state-name` to CLI args
2. Pass state_name to StateManager  
3. Update StateManager to use custom name if provided
4. Update justfile to support state-name parameter
5. Update test narratives to use shared state name


## Root Cause Discovery (2025-11-23)

The issue has been identified! Looking at the logs:
- Channel is created successfully: "Successfully created channel channel_id=1442012048920674335"  
- But NO "Captured bot command ID to state" or "Saved bot command IDs to state" messages appear
- This means `capture_bot_command_ids()` is not finding any matching ID fields in the JSON response

### The Problem

The `capture_bot_command_ids()` method in executor.rs looks for fields like "channel_id", "message_id", etc. in the JSON result. But the Discord API response from channels.create likely uses a different structure (maybe just "id" or nested structure).

### The Solution

We need to:
1. Log what the actual JSON response looks like from Discord commands
2. Update the ID field extraction logic to handle Discord's actual response format  
3. Possibly use the command type to determine which field contains the ID

## Next Steps

1. **Debug Discord Response Format** - Add logging to see actual JSON structure
2. **Fix ID Extraction Logic** - Update capture_bot_command_ids to handle Discord's format
3. **Verify State Persistence** - Confirm IDs are actually captured and saved
