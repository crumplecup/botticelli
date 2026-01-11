# Discord Commands Helper Type Refactor

## Goal
Refactor discord command free functions to use helper type pattern (like `Extract` in botticelli_narrative).

## Current Pattern (Free Functions)
```rust
// crates/botticelli_social/src/discord/commands/channels.rs
pub(super) async fn list(http: &Arc<Http>, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> { }
pub(super) async fn get(http: &Arc<Http>, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> { }
```

## New Pattern (Helper Type)
```rust
/// Channel command namespace
#[derive(Debug, Clone, Copy)]
pub(super) struct Channels;

impl Channels {
    pub async fn list(http: &Arc<Http>, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> { }
    pub async fn get(http: &Arc<Http>, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> { }
}
```

## Benefits
1. **Namespace organization** - Groups related commands under a type
2. **Consistency** - Matches `Extract` pattern from botticelli_narrative
3. **Discoverability** - IDE autocomplete shows all channel commands under `Channels::`
4. **Future extensibility** - Easy to add associated constants or constructor parameters
5. **Clear ownership** - Commands belong to a type, not floating in module

## Modules to Refactor
1. `channels.rs` - 8 functions → `Channels` helper type
2. `events.rs` - 5 functions → `Events` helper type
3. `forum.rs` - 3 functions → `Forum` helper type
4. `members.rs` - 5 functions → `Members` helper type
5. `messages.rs` - 9 functions → `Messages` helper type
6. `misc.rs` - 6 functions → `Misc` helper type
7. `moderation.rs` - 4 functions → `Moderation` helper type
8. `reactions.rs` - 5 functions → `Reactions` helper type
9. `roles.rs` - 7 functions → `Roles` helper type
10. `server.rs` - 1 function → `Server` helper type
11. `threads.rs` - 9 functions → `Threads` helper type

**Total: 62 functions across 11 modules**

## Implementation Steps

### Phase 1: Single Module Prototype (channels.rs)
1. Add helper type struct at top of module
2. Move all command functions into impl block
3. Update executor.rs call sites: `channels::list()` → `Channels::list()`
4. Verify compilation and tests
5. Commit

### Phase 2: Remaining Modules (batch by size)
**Batch 1 - Large modules:**
- `messages.rs` (9 functions)
- `threads.rs` (9 functions)

**Batch 2 - Medium modules:**
- `channels.rs` (already done)
- `roles.rs` (7 functions)
- `misc.rs` (6 functions)

**Batch 3 - Small modules:**
- `members.rs` (5 functions)
- `reactions.rs` (5 functions)
- `events.rs` (5 functions)
- `moderation.rs` (4 functions)
- `forum.rs` (3 functions)
- `server.rs` (1 function)

### Phase 3: Update Exports
- Update `mod.rs` if needed
- Verify all executor call sites updated
- Run full test suite

## Executor Call Site Updates

### Before
```rust
// executor.rs execute() method
"channels.list" => channels::list(http, args).await,
"channels.get" => channels::get(http, args).await,
```

### After
```rust
// executor.rs execute() method
"channels.list" => Channels::list(http, args).await,
"channels.get" => Channels::get(http, args).await,
```

## Documentation Updates
- Module-level docs should describe helper type
- Each helper type gets doc comment explaining its purpose
- Example usage in module docs

## Testing Strategy
- Run `just check botticelli_social` after each module
- Run `just test-package botticelli_social` after each batch
- Verify executor still routes commands correctly

## Non-Goals
- Not changing function signatures
- Not changing behavior or logic
- Not adding new functionality
- Not modifying helper functions (those stay as free functions)

## Risks & Mitigations
- **Risk**: Breaking executor routing
  - **Mitigation**: Systematic updates, compile after each module
- **Risk**: Import statement changes
  - **Mitigation**: Helper types are `pub(super)`, imported once in executor
- **Risk**: Missing call sites
  - **Mitigation**: Compiler will catch all missing updates

## Success Criteria
- ✅ All 62 command functions moved to helper types
- ✅ Executor properly routes all commands
- ✅ All tests pass
- ✅ Code compiles without warnings
- ✅ Pattern consistent across all modules

## Completion Status

**COMPLETED** - All 11 modules refactored successfully (commits 10d021d through 92d0a63)

### Refactored Modules
1. ✅ `channels.rs` - 8 functions → `Channels` (commit 10d021d)
2. ✅ `messages.rs` - 9 functions → `Messages` (commit 5ec0ec0)
3. ✅ `threads.rs` - 9 functions → `Threads` (commit fc49a69)
4. ✅ `roles.rs` - 7 functions → `Roles` (commit 271d3fa)
5. ✅ `members.rs` - 5 functions → `Members` (commit 2e082e0)
6. ✅ `reactions.rs` - 5 functions → `Reactions` (commit eb26384)
7. ✅ `events.rs` - 5 functions → `Events` (commit bf4b430)
8. ✅ `moderation.rs` - 4 functions → `Moderation` (commit b323e15)
9. ✅ `forum.rs` - 3 functions → `Forum` (commit 45c657f)
10. ✅ `misc.rs` - 6 functions → `Misc` (commit 6481051)
11. ✅ `server.rs` - 1 function → `Server` (commit 92d0a63)

### Key Achievements
- Zero-sized helper types with `#[derive(Debug, Clone, Copy)]` pattern
- All `#[instrument]` attributes preserved correctly
- Proper 4-space indentation inside impl blocks
- Helper functions remain as free functions outside impl blocks
- Executor updated with helper type imports and call sites
- All compilation checks pass (`cargo check -p botticelli_social`)
- All tests pass (`just test-package botticelli_social`)
- Manual refactoring approach ensured correctness after failed automation attempt

### Lessons Learned
- Automated sed-based refactoring fails for context-dependent transformations
- Manual "surgeon" approach required for instrumentation attribute placement
- Indentation and impl block structure critical for maintainability
- Pattern consistency across all modules improves discoverability
