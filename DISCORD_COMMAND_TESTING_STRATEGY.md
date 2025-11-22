# Discord Command Testing Strategy

## Status: Phase 1 Complete ✅

We've successfully implemented direct command testing infrastructure:
- Created `discord_direct_commands_test.rs` with 8 tests covering basic read operations
- Tests use `BotCommandExecutor` trait directly (no narrative overhead)
- All tests pass when run with `--ignored` flag
- Established pattern for future command tests

**Next Steps:** Expand test coverage to write operations and remaining command categories.

## Current State Analysis

### Problems Identified

1. **Narrative-based tests are complex** - Each test requires:
   - Creating TOML narrative files
   - Setting up database connections
   - Running full executor pipeline
   - Hard to isolate individual command failures

2. **Missing test infrastructure**:
   - No direct Discord API testing helpers
   - No way to test commands without full narrative execution
   - Difficult to verify command outputs

3. **Test coverage gaps**:
   - Many commands have no tests at all
   - Existing tests only cover happy paths
   - No error condition testing
   - No validation of command arguments

### Root Cause

The current architecture couples command testing to the narrative executor. We need a way to test commands in isolation.

## Recommended Testing Strategy

### Phase 1: Direct Command Testing (PRIORITY)

Create unit tests that directly invoke Discord bot commands without narratives:

```rust
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_channels_list() {
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").unwrap();
    let guild_id = env::var("TEST_GUILD_ID").unwrap();
    
    let bot = DiscordBot::new(&token).unwrap();
    let args = BotCommandArgs::new()
        .with_arg("guild_id", &guild_id);
    
    let result = bot.execute("channels.list", args).unwrap();
    
    // Verify result structure
    assert!(result.contains("channels"));
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["channels"].is_array());
}
```

**Benefits:**
- Fast execution (no narrative parsing)
- Easy to debug individual commands
- Clear pass/fail criteria
- Can test error conditions

### Phase 2: Command Argument Validation

Test that commands properly validate their arguments:

```rust
#[test]
fn test_channels_get_missing_channel_id() {
    let bot = DiscordBot::new(&token).unwrap();
    let args = BotCommandArgs::new()
        .with_arg("guild_id", "123");
    // Missing channel_id
    
    let result = bot.execute("channels.get", args);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("channel_id"));
}
```

### Phase 3: Integration Tests with Narratives

Keep narrative-based tests for end-to-end workflows:
- Multi-step operations (create → publish → pin)
- Cross-command dependencies
- State management validation

**But:** Simplify them using helper functions:

```rust
fn run_test_narrative(name: &str) -> Result<String, BotticelliError> {
    let narrative_path = format!("tests/narratives/discord/{}.toml", name);
    let executor = create_test_executor();
    executor.execute_from_file(&narrative_path)
}

#[test]
fn test_welcome_workflow() {
    let result = run_test_narrative("publish_welcome").unwrap();
    assert!(result.contains("message_id"));
}
```

### Phase 4: Test Fixtures and Cleanup

Create test utilities for:
- Creating temporary test channels
- Cleaning up after tests
- Shared test state (e.g., test guild setup)

```rust
struct TestGuild {
    guild_id: String,
    bot: DiscordBot,
    created_channels: Vec<String>,
}

impl TestGuild {
    fn new() -> Self { /* ... */ }
    
    fn create_test_channel(&mut self, name: &str) -> String {
        // Create and track channel
    }
}

impl Drop for TestGuild {
    fn drop(&mut self) {
        // Clean up all created channels
    }
}
```

## Implementation Plan

### Step 1: Create Direct Command Test Suite ✅ DONE

File: `crates/botticelli_social/tests/discord_direct_commands_test.rs`

Test each command category:
- ✅ Basic reads (guilds.get, channels.list, channels.get)
- ✅ Member operations (members.list)
- ✅ Role operations (roles.list)
- ⚠️ Channel write operations (channels.create, channels.delete) - TODO
- ⚠️ Message operations (messages.send, messages.get, messages.pin) - TODO

### Step 2: Add Argument Validation Tests

For each command, test:
- Missing required arguments
- Invalid argument formats
- Boundary conditions

### Step 3: Simplify Narrative Tests

Consolidate narrative tests into workflow tests:
- `test_channel_creation_workflow` - Create → verify → cleanup
- `test_message_publishing_workflow` - Send → pin → verify
- `test_content_generation_workflow` - Generate → select → publish

### Step 4: Add Test Utilities

Create `crates/botticelli_social/tests/test_utils/discord.rs`:
- `create_test_bot()` - Initialize bot with test token
- `create_temporary_channel()` - Create and auto-cleanup
- `send_test_message()` - Send and auto-cleanup
- `assert_discord_id()` - Validate ID format

## Success Criteria

1. **Coverage**: Every implemented command has at least one passing test
2. **Speed**: Direct command tests run in <5 seconds total
3. **Reliability**: Tests pass consistently (not flaky)
4. **Clarity**: Test failures clearly indicate what command/argument failed
5. **Cleanup**: Tests leave no artifacts in Discord server

## Migration Path

1. ✅ Create `discord_direct_commands_test.rs`
2. Port one command test from narrative → direct (prove pattern)
3. Implement remaining direct command tests
4. Refactor existing narrative tests to use helpers
5. Add cleanup utilities
6. Update TEST_FIXES_NEEDED with progress

## Open Questions

1. **Rate Limiting**: How do we handle Discord API rate limits in tests?
   - Solution: Use test guilds with generous rate limits, or add delays between tests

2. **Permissions**: What permissions does the test bot need?
   - Solution: Document required permissions in test README

3. **Cleanup Failures**: What if cleanup fails (e.g., network error)?
   - Solution: Log cleanup failures but don't fail tests; provide manual cleanup script

4. **Parallel Testing**: Can we run tests in parallel?
   - Solution: Start with serial execution, add parallelism later with proper isolation
