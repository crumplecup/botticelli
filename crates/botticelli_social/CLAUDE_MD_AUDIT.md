# CLAUDE.md Compliance Audit for botticelli_social

## Critical Violations

### 1. Public Fields on Structs (CRITICAL)

**Rule:** Types should be public, their fields should not. Use derive_getters for field access.

**Violations:**

#### Error Types
- `BotCommandError` - has `pub kind`, `pub line`, `pub file`
- `DiscordError` - has `pub kind`, `pub line`, `pub file`

**Fix:** Make fields private, add `#[derive(Getters)]` from derive_getters

#### Core Structs with Private Fields (Missing Getters)
- `DiscordCommandExecutor` - private `http`, `permission_checker` - needs getters
- `BotticelliBot` - private `client`, `repository` - needs getters  
- `SecureBotCommandExecutor<V>` - private `registry`, `security` - needs getters
- `BotCommandRegistryImpl` - private `commands` - needs getters
- `DiscordRepository` - private field - needs getters

#### Processor Structs (Missing Getters)
- `DiscordGuildProcessor` - private `cache`, `http` - needs getters
- `DiscordUserProcessor` - private `cache`, `http` - needs getters
- `DiscordChannelProcessor` - private `cache`, `http` - needs getters
- `DiscordRoleProcessor` - private `cache`, `http` - needs getters
- `DiscordGuildMemberProcessor` - private `cache`, `http` - needs getters
- `DiscordMemberRoleProcessor` - private `cache`, `http` - needs getters

### 2. Missing Derive Macros

**Rule:** Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible.

**Audit Needed:** Check all structs to ensure they have appropriate derives where applicable.

### 3. Missing Builder Patterns

**Rule:** Use derive_builders for complex structs with multiple optional fields.

**Candidates:**
- `SecureBotCommandExecutor::new()` - takes 6 parameters, should use builder
- Any other constructors with 4+ parameters

### 4. Missing Setter Patterns  

**Rule:** Use derive_setters for mutable field access instead of manual setters.

**Audit Needed:** Check for manual setter methods that should use derive_setters.

### 5. Documentation

**Rule:** All public types, functions, and methods must have documentation.

**Audit Needed:** Run `cargo clippy` to find missing documentation.

## Action Items

### High Priority - COMPLETED ✅
1. ✅ Add `#[derive(Getters)]` to error structs and core execution structs
2. ✅ Make error struct fields private (add getters)
3. ✅ Update test code to use getter methods instead of direct field access
4. ✅ Run `cargo clippy` - zero warnings/errors

### Completed Files
1. ✅ `src/bot_commands.rs` - BotCommandError (private fields + Getters), BotCommandRegistryImpl (Getters)
2. ✅ `src/discord/error.rs` - DiscordError (private fields + Getters)
3. ✅ `src/discord/commands.rs` - DiscordCommandExecutor (Getters)
4. ✅ `src/secure_executor.rs` - SecureBotCommandExecutor (Getters), test fixes
5. ✅ `src/secure_bot_executor.rs` - test fixes for getter usage

### Medium Priority - TODO
6. Add Getters to remaining structs (see below)
7. Add derive_builders to multi-parameter constructors
8. Add derive_setters for mutable access patterns
9. Audit and add missing Debug/Clone/etc derives where applicable
10. Audit and add missing documentation

### Low Priority - TODO
11. Consider builder patterns for complex initialization
12. Review serialization derives (Serialize/Deserialize)

## Remaining Files to Update

1. `src/discord/client.rs` - BotticelliBot (needs Getters)
2. `src/discord/repository.rs` - DiscordRepository (needs Getters)
3. `src/discord/processors.rs` - All processor structs (need Getters)
4. `src/discord/json_models.rs` - All JSON model structs (audit derives)
5. `src/discord/models/*.rs` - All database model structs (audit derives)

## Dependencies to Add

```toml
[dependencies]
derive_getters = "0.5"
derive_setters = "0.1"
derive_builder = "0.20"
```

## Testing Checklist

After fixes:
- [ ] `cargo check` passes
- [ ] `cargo test --lib --tests` passes
- [ ] `cargo clippy --all-targets` has zero warnings
- [ ] `cargo test --doc` passes
- [ ] Integration tests still work
