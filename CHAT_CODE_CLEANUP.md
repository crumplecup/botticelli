# Chat Code Cleanup Plan

## Issues to Fix

### 1. Dead Code Warnings
- [ ] `CloseModal`, `ExecuteAction`, `OpenFile`, `SaveFile`, `UpdateSearch` variants
- [ ] `Csv`, `Json`, `Sql` variants  
- [ ] `selected` field
- [ ] `limit` field
- [ ] `is_empty` method
- [ ] `handle_key`, `selected` methods
- [ ] `setup_terminal`, `restore_terminal` functions

### 2. CLAUDE.md Violations

#### Remove all `#[allow]` directives
- [ ] Never use `#[allow(dead_code)]` or any `#[allow(...)]`
- [ ] Fix root causes with feature gates, getters, or deletion

#### Use derive_getters/derive_setters
- [ ] `ConversationState` - already has manual getters, convert to derives
- [ ] `NavigationPanel` - has `set_items`, needs derive_setters
- [ ] `ScheduleTab` - has `set_tasks`, needs derive_setters  
- [ ] `DatabaseTab` - has multiple setters, needs derive_setters
- [ ] Other structs with public fields

#### Make fields private
- [ ] Audit all `pub` fields in structs
- [ ] Make private and add `#[derive(Getters)]`
- [ ] Use `#[derive(Setters)]` with `#[setters(prefix = "with_")]` for mutable state

#### Add #[instrument] to all public functions
- [ ] Audit all public functions in all modules
- [ ] Add `#[instrument]` with appropriate `skip` and `fields`
- [ ] Add tracing events at key points

#### Add missing documentation
- [ ] All public items need `///` documentation
- [ ] Enforced by `#![warn(missing_docs)]`

### 3. Feature Gate Issues
- [ ] Binary requires `cli` and `tui` features - document or fix
- [ ] Ensure unused code is properly feature-gated

## Success Criteria
- [ ] `cargo check -p botticelli_chat` - zero warnings
- [ ] `cargo check -p botticelli_chat --all-features` - zero warnings
- [ ] `just check-features` - all combinations pass
- [ ] No `#[allow]` directives anywhere
- [ ] All public functions have `#[instrument]`
- [ ] All public items have documentation
- [ ] All fields private with derived accessors
