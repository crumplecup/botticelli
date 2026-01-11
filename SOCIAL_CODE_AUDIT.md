# Social Crate Code Audit

## Summary

Audit Date: 2026-01-11
Status: **Needs Refactoring**

### Critical Issues Found

1. **BotCommandExecutor trait should be in interface** (P0)
2. **Discord commands.rs is 5189 lines** - massive violation (P0)
3. **Public fields in database model structs** - should use getters (P1)
4. **Error conversion loses source** - map_err throwing away errors (P1)
5. **Manual Clone impl in TokenBucket resets Instant** - semantic bug (P2)

### Metrics

- **Lines of Code**: 5189 (commands.rs alone)
- **Public Functions**: 58
- **Instrumented Functions**: 91 (exceeds public - good!)
- **Manual Constructors**: 0 (good - using derive_new/Setters)
- **Public Fields**: 12 in database models (violates encapsulation)

---

## P0 Issues (Must Fix)

### 1. BotCommandExecutor Trait in Wrong Crate

**Location**: `src/bot_commands.rs:218-253`

**Problem**: `BotCommandExecutor` trait is defined in `botticelli_social` but should be in `botticelli_interface` alongside `BotCommandRegistry`.

**Current**:
```rust
// src/bot_commands.rs
#[async_trait]
pub trait BotCommandExecutor: Send + Sync {
    fn platform(&self) -> &str;
    async fn execute(&self, command: &str, args: &HashMap<String, JsonValue>) 
        -> BotCommandResult<JsonValue>;
    // ... many more methods
}
```

**Why This Matters**:
- Interface traits should be in `botticelli_interface`
- Social is a **concrete implementation**, not an abstraction layer
- Other crates (narrative, actor) depend on the abstraction, not the impl
- Current: interface → social (circular dependency risk)
- Should be: interface → social, narrative → interface

**Fix**:
1. Move `BotCommandExecutor` trait to `botticelli_interface/src/repository/bot_command.rs`
2. Keep only `DiscordCommandExecutor` (impl) in social
3. Update `BotCommandRegistry` to use `BotCommandExecutor` from interface
4. Social becomes pure implementation crate

---

### 2. Discord commands.rs is 5189 Lines

**Location**: `src/discord/commands.rs`

**Problem**: Single file contains 68 private functions + 3 public functions = massive god object

**Breakdown**:
- 68 command handler functions (channels_list, members_get, roles_assign, etc.)
- Each 50-150 lines
- Mix of concerns: parsing, validation, API calls, response formatting
- Zero modularization

**Why This is Critical**:
- Impossible to navigate or audit
- High merge conflict risk
- Violates single responsibility principle
- Testing nightmare
- AI context window overflow

**Refactoring Strategy**:

```
src/discord/
├── commands/
│   ├── mod.rs              # Re-export + executor
│   ├── server.rs           # server.* commands
│   ├── channels.rs         # channels.* commands  
│   ├── members.rs          # members.* commands
│   ├── roles.rs            # roles.* commands
│   ├── messages.rs         # messages.* commands
│   ├── reactions.rs        # reactions.* commands
│   ├── threads.rs          # threads.* commands
│   ├── events.rs           # events.* commands
│   └── moderation.rs       # bans, kicks, timeouts
└── commands.rs             # DELETE after split
```

Each module:
- 300-500 lines max
- Single concern (server operations, channel management, etc.)
- Own tests in `tests/discord_commands_{module}_test.rs`
- Clear function boundaries

**Example Split** (server.rs):
```rust
// src/discord/commands/server.rs
use super::shared::{parse_guild_id, BotCommandResult};

#[instrument(skip(http))]
pub(super) async fn get_stats(
    http: &Http,
    args: &HashMap<String, JsonValue>
) -> BotCommandResult<JsonValue> {
    // Just server.get_stats logic
}

#[instrument(skip(http))]
pub(super) async fn get_info(
    http: &Http,
    args: &HashMap<String, JsonValue>
) -> BotCommandResult<JsonValue> {
    // Just server.get_info logic
}
```

Then commands/mod.rs routes to submodules.

---

## P1 Issues (Should Fix)

### 3. Public Fields in Database Models

**Location**: 
- `src/discord/models/guild.rs:14-24` (GuildRow)
- `src/discord/models/user.rs:14-26` (UserRow)
- `src/discord/models/member.rs` (MemberRow)

**Problem**: Diesel Queryable structs have `pub` fields, violating encapsulation.

**Current**:
```rust
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, derive_getters::Getters)]
pub struct GuildRow {
    pub id: i64,              // ❌ Public field
    pub name: String,         // ❌ Public field
    pub icon: Option<String>, // ❌ Public field
    // ... more pub fields
    
    features: Option<Vec<Option<String>>>, // ✅ Private
    description: Option<String>,           // ✅ Private
}
```

**Why This Matters**:
- External crates can mutate fields directly
- Breaks invariants (e.g., ID should never change)
- No validation on field updates
- Already has `derive_getters` - contradicts intent

**Fix**:
```rust
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, derive_getters::Getters)]
pub struct GuildRow {
    id: i64,              // ✅ Private with getter
    name: String,         // ✅ Private with getter
    icon: Option<String>, // ✅ Private with getter
    // ... all private
}

// derive_getters provides:
// guild.id() -> &i64
// guild.name() -> &String
// guild.icon() -> &Option<String>
```

Diesel Queryable works with private fields. Only needs public visibility during SQL mapping, not after construction.

---

### 4. Error Conversions Lose Source

**Location**: 
- `src/secure_bot_executor.rs:99-102`
- `src/secure_executor.rs:134-139`

**Problem**: `map_err` creates new error, discarding source error chain.

**Current**:
```rust
// src/secure_bot_executor.rs:99
.map_err(|e| {
    error!("Security check failed: {}", e);
    security_error_to_bot_error(command, e)  // ❌ Loses source error
})?;

// src/secure_executor.rs:134
other => serde_json::to_string(other).map_err(|e| {
    BotCommandError::new(BotCommandErrorKind::SerializationError {
        command: "convert_args".to_string(),
        reason: format!("Failed to serialize argument '{}': {}", key, e),
        // ❌ Error message embedded as string - source lost
    })
})?,
```

**Why This Matters**:
- Stack traces stop at conversion point
- Can't access underlying error for debugging
- Violates error chaining best practices
- Makes root cause analysis impossible

**Fix Option 1** - Add source field to ErrorKind:
```rust
#[derive(Debug, Display)]
pub enum BotCommandErrorKind {
    SerializationError {
        command: String,
        reason: String,
        source: Box<dyn std::error::Error + Send + Sync>,  // ✅ Keep source
    },
}
```

**Fix Option 2** - Use derive_more::From:
```rust
use derive_more::From;

#[derive(Debug, Display, Error, From)]
pub enum BotCommandError {
    Security(SecurityError),           // ✅ Automatic From impl
    Serialization(serde_json::Error),  // ✅ Automatic From impl
    Command(BotCommandErrorKind),
}

// Then just use ?:
serde_json::to_string(other)?  // ✅ Auto-converts to BotCommandError
```

**Best**: Use Option 2 for automatic error propagation with source preservation.

---

### 5. Error Types Should Be in _error Crate

**Location**: `src/bot_commands.rs:40-163` (BotCommandError, BotCommandErrorKind)

**Problem**: Error types defined in implementation crate instead of dedicated error crate.

**Why This Matters**:
- Error types are cross-cutting concerns
- Multiple crates need to construct/handle these errors
- Creates circular dependency if interface needs to reference social errors
- Violates "errors in _error" pattern from other crates

**Current Structure**:
```
botticelli_social/
└── src/
    ├── bot_commands.rs
    │   ├── BotCommandError        // ❌ Should be in error crate
    │   ├── BotCommandErrorKind     // ❌ Should be in error crate
    │   ├── BotCommandExecutor      // ❌ Should be in interface
    │   └── BotCommandRegistry      // ✅ OK (registry is impl)
```

**Should Be**:
```
botticelli_error/
└── src/
    └── social.rs                   // ✅ New module
        ├── BotCommandError
        └── BotCommandErrorKind

botticelli_interface/
└── src/
    └── repository/
        └── bot_command.rs
            └── BotCommandExecutor  // ✅ Trait moved here

botticelli_social/
└── src/
    ├── discord/
    │   └── commands.rs
    │       └── DiscordCommandExecutor  // ✅ Just impl
    └── bot_commands.rs
        └── BotCommandRegistry          // ✅ Registry impl
```

**Fix**:
1. Create `botticelli_error/src/social.rs`
2. Move `BotCommandError` and `BotCommandErrorKind` there
3. Export from `botticelli_error` crate root
4. Update imports in social to use `botticelli_error::BotCommandError`

---

## P2 Issues (Nice to Have)

### 6. TokenBucket Clone Resets Instant

**Location**: `crates/botticelli_security/src/rate_limit.rs:50-69`

**Problem**: Manual Clone impl resets `last_refill` to `Instant::now()` instead of cloning it.

**Current**:
```rust
impl Clone for TokenBucket {
    fn clone(&self) -> Self {
        Self {
            limit: self.limit.clone(),
            tokens: self.tokens,
            last_refill: Instant::now(),  // ❌ Resets to now
        }
    }
}
```

**Why This Matters**:
- Semantic surprise - Clone should preserve state
- Cloned bucket has artificially recent `last_refill`
- Leads to incorrect token refill calculations
- Breaks rate limiting if bucket is cloned mid-usage

**When This Could Break**:
```rust
let bucket = TokenBucket::new(limit);
// Use bucket for 5 seconds
let cloned = bucket.clone();
// Cloned bucket thinks it was JUST created
// Refill calculation will be wrong for next 5 seconds
```

**Fix Option 1** - Don't allow Clone:
```rust
// Remove Clone derive and manual impl
#[derive(Debug)]  // No Clone
struct TokenBucket {
    // ... fields
}
```

This is probably correct - rate limiters shouldn't be cloned. They track stateful time-based data.

**Fix Option 2** - Clone properly with caveat:
```rust
impl Clone for TokenBucket {
    /// Clones the token bucket including current state.
    /// 
    /// WARNING: Cloning splits the rate limit budget between instances.
    /// Both buckets will allow tokens independently, effectively
    /// doubling the allowed rate. Only clone if you're replacing
    /// the original bucket.
    fn clone(&self) -> Self {
        Self {
            limit: self.limit.clone(),
            tokens: self.tokens,
            last_refill: self.last_refill,  // ✅ Preserve actual time
        }
    }
}
```

But given the warning, probably better to just remove Clone support entirely.

---

### 7. Instrumentation Coverage

**Good News**: 91 instrumented functions vs 58 public functions means private helpers are instrumented too!

**Gaps to Check**:
```bash
# Find uninstrumented public functions:
cd crates/botticelli_social
grep -r "pub fn\|pub async fn" src/ | grep -v "#\[instrument" | head -20
```

Most likely candidates:
- Model conversion functions
- Simple getters/setters (may be OK to skip)
- Database helper functions in discord/repository.rs

**Recommendation**: Audit `discord/repository.rs` and `discord/conversions.rs` for missing instrumentation.

---

## Recommendations Summary

### Immediate Actions (P0)

1. **Split discord/commands.rs**:
   - Create `src/discord/commands/` directory
   - Split into ~10 modules by command category
   - Max 500 lines per module
   - Test compilation after each split

2. **Move BotCommandExecutor to interface**:
   - Add to `botticelli_interface/src/repository/bot_command.rs`
   - Update social to import from interface
   - Remove from social crate

### High Priority (P1)

3. **Fix database model encapsulation**:
   - Remove `pub` from all fields in GuildRow, UserRow, MemberRow
   - Verify `derive_getters` provides access
   - Test Diesel queries still work (they will)

4. **Fix error source chain**:
   - Add `#[from]` derives to BotCommandError
   - Use `?` instead of `map_err` where possible
   - Preserve source errors in conversion

5. **Move errors to botticelli_error**:
   - Create `botticelli_error/src/social.rs`
   - Move BotCommandError and BotCommandErrorKind
   - Update social imports

### Medium Priority (P2)

6. **Fix TokenBucket Clone**:
   - Remove Clone support (preferred)
   - Or fix to preserve `last_refill` with big warning

7. **Audit instrumentation coverage**:
   - Check discord/repository.rs
   - Check discord/conversions.rs
   - Add missing `#[instrument]` to public functions

---

## Testing Strategy

After each refactor:

```bash
# Check compilation
cargo check --package botticelli_social

# Run clippy
cargo clippy --package botticelli_social -- -D warnings

# Run tests
cargo test --package botticelli_social

# Verify no regressions
git diff --stat
```

---

## Notes

- Good: Using derive_getters/setters instead of manual implementations
- Good: High instrumentation coverage (91 spans for 58 public functions)
- Good: No manual constructors found (all using derive_new/Setters)
- Good: Proper error types with derive_more::Display + derive_more::Error
- Bad: Massive 5189-line god object file
- Bad: Traits and errors in wrong crates
- Bad: Public fields breaking encapsulation

This crate has **good practices** (derives, instrumentation) but **poor architecture** (wrong crate boundaries, giant files).
