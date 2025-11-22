# State Integration Implementation Plan

## Goal
Enable bot commands to automatically save their output IDs to persistent state, and allow narratives to reference these IDs using `${state:key}` syntax.

## Implementation Checklist

### Phase 1: State Template Resolution ✓ NEXT
- [ ] Add `state:` prefix handling to `resolve_template()` function
- [ ] Check if state_manager is available in executor
- [ ] Load value from state using the key after `state:`
- [ ] Return error if state_manager is None or key not found
- [ ] Add tests for state template resolution

### Phase 2: Bot Command Output Capture
- [ ] Identify where bot commands are executed in executor.rs
- [ ] After successful bot command execution, parse JSON response
- [ ] Extract common ID fields (channel_id, message_id, role_id, etc.)
- [ ] Generate state key: `<platform>.<command>.<id_type>`
- [ ] Save extracted IDs to state_manager if present
- [ ] Log state saves for debugging

### Phase 3: Integration Tests
- [ ] Create test narrative that creates a channel
- [ ] Verify channel_id is saved to state automatically
- [ ] Create second narrative that references ${state:channel_id}
- [ ] Verify ID is resolved correctly
- [ ] Create teardown narrative that deletes using state ID
- [ ] Run full lifecycle test

### Phase 4: Documentation Updates
- [ ] Update AI_NARRATIVE_TOML_GUIDE with ${state:*} examples
- [ ] Document automatic ID capture behavior
- [ ] Show create -> use -> delete patterns
- [ ] Add troubleshooting section for state issues

### Phase 5: Refactor Existing Tests
- [ ] Remove hardcoded TEST_CHANNEL_ID references
- [ ] Use state management instead
- [ ] Create setup narratives that populate state
- [ ] Update teardown narratives to use state
- [ ] Verify all tests pass

## Implementation Notes

### State Key Naming
```
discord.channels.create.channel_id
discord.messages.send.message_id
discord.roles.create.role_id
```

### Template Syntax
```toml
# Long form (explicit)
channel_id = "${state:discord.channels.create.channel_id}"

# Short form (when unambiguous)
channel_id = "${state:channel_id}"
```

### Error Handling
- If state_manager is None, template resolution should fail with clear error
- If state key doesn't exist, show available keys in error message
- If bot command fails, don't save to state

### Backward Compatibility
- Existing ${act_name.field} syntax continues to work
- ${previous} syntax continues to work
- New ${state:key} syntax added alongside

## Testing Strategy

1. **Unit tests**: Test resolve_template with state: prefix
2. **Integration tests**: Test full create -> use -> delete flow
3. **Error cases**: Missing state_manager, missing keys
4. **Multiple IDs**: Create multiple channels, verify each ID is distinct

## Success Criteria

- [ ] Can create Discord channel and ID is auto-saved to state
- [ ] Can reference ${state:channel_id} in subsequent acts
- [ ] Can delete channel using state ID
- [ ] State persists across narrative executions
- [ ] All existing tests still pass
- [ ] Documentation is clear and includes examples
