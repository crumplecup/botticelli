# Bot Command State Integration

## Problem

Bot commands (like `channels.create`) return IDs that need to be persisted and reused across:
1. Multiple acts within a narrative
2. Multiple narrative executions (for cleanup/teardown)
3. Test scenarios that create, use, and delete resources

Currently:
- Bot commands execute but their outputs are not captured
- No automatic persistence of command results to state
- Tests cannot reliably reference created resource IDs

## Solution

### 1. Bot Command Output Capture

When a bot command executes successfully:
1. Parse the command output for key identifiers (channel_id, message_id, role_id, etc.)
2. Automatically save these to the state manager under predictable keys
3. Make them available for template substitution in subsequent acts

### 2. State Key Naming Convention

```
<platform>.<command>.<resource_type>_id
```

Examples:
- `discord.channels.create.channel_id` - ID of last created channel
- `discord.messages.send.message_id` - ID of last sent message  
- `discord.roles.create.role_id` - ID of last created role

###3. Template Substitution from State

In narrative TOMLs, reference state values:
```toml
[act.delete.input.args]
channel_id = "${state:discord.channels.create.channel_id}"
```

Or use short form if context is clear:
```toml
[act.delete.input.args]
channel_id = "${state:channel_id}"
```

### 4. Implementation Steps

**Step 1**: Enhance `execute_bot_command` in executor.rs
- After successful command execution, parse output JSON
- Extract ID fields (channel_id, message_id, etc.)
- Save to state_manager if present

**Step 2**: Add template substitution for `${state:*}` patterns
- In `substitute_variables`, check for `state:` prefix
- Load value from state_manager
- Replace in args before execution

**Step 3**: Update narrative guides
- Document state: prefix for referencing state values
- Show examples of create -> use -> delete patterns
- Explain automatic ID capture

**Step 4**: Create test helpers
- `setup_test_channel()` - creates channel, returns state key
- `cleanup_test_resources()` - deletes all resources from state
- `with_test_resource()` - RAII pattern for resource lifecycle

### 5. Example Flow

```toml
name = "channel_lifecycle_test"
skip_content_generation = true

# Create channel - ID automatically saved to state
[act.create]
[[act.create.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"
[act.create.input.args]
name = "test-channel"

# Use channel - ID loaded from state automatically
[act.send]
[[act.send.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"
[act.send.input.args]
channel_id = "${state:channel_id}"  # Auto-resolved
content = "Test message"

# Cleanup - ID loaded from state
[act.cleanup]
[[act.cleanup.input]]
type = "bot_command"
platform = "discord"
command = "channels.delete"
[act.cleanup.input.args]
channel_id = "${state:channel_id}"
```

### 6. Benefits

- **No manual ID tracking**: Commands auto-save their outputs
- **Persistent across runs**: State saved to disk between executions
- **Clean test patterns**: Create, use, cleanup with minimal boilerplate
- **Debugging**: State file shows what resources were created
- **Safety**: Can inspect state to clean up orphaned resources

## Next Steps

1. Implement output capture in executor
2. Add ${state:*} template substitution
3. Update AI_NARRATIVE_TOML_GUIDE with state examples
4. Refactor existing test narratives to use state
5. Add test helper functions for common patterns
