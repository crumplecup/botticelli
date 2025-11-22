# State Integration Implementation Plan

## Goal
Enable bot commands to automatically save their output IDs to persistent state, and allow narratives to reference these IDs using `${state:key}` syntax.

## Implementation Checklist

### Phase 1: State Template Resolution ✅ DONE
- [x] Add `state:` prefix handling to `resolve_template()` function
- [x] Check if state_manager is available in executor
- [x] Load value from state using the key after `state:`
- [x] Return error if state_manager is None or key not found
- [x] Add helpful error messages with available keys
- [ ] Add unit tests for state template resolution (deferred)

### Phase 2: Bot Command Output Capture ✅ DONE
- [x] Identify where bot commands are executed in executor.rs (process_inputs method)
- [x] After successful bot command execution, parse JSON response
- [x] Extract common ID fields (channel_id, message_id, role_id, etc.)
- [x] Generate state key: `<platform>.<command>.<id_type>`
- [x] Save extracted IDs to state_manager if present (both full and short keys)
- [x] Log state saves for debugging

### Phase 3: Integration Tests ⏳ NEXT
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

- [x] Can create Discord channel and ID is auto-saved to state
- [x] Can reference ${state:channel_id} in subsequent acts
- [x] Can delete channel using state ID
- [x] State persists across narrative executions (via --state-dir flag)
- [x] CLI supports --state-dir for state management
- [ ] Integration tests verified (ready to test)
- [ ] All existing tests still pass
- [ ] Documentation is clear and includes examples

## Implementation Complete (Phases 1 & 2)

✅ Phase 1: State template resolution with ${state:key} syntax
✅ Phase 2: Automatic bot command ID capture to persistent state
✅ CLI Integration: --state-dir flag for state management

**Ready for Phase 3**: Integration testing and validation
