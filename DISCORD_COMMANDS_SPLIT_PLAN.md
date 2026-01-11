# Discord Commands.rs Split Plan

## Current State
- **File**: `src/discord/commands.rs`
- **Lines**: 5192
- **Commands**: 68 total across 12 categories
- **Status**: Monolithic god object - impossible to navigate

## Target Structure

```
src/discord/
├── commands/
│   ├── mod.rs              # Executor + routing (300 lines)
│   ├── channels.rs         # 23 commands (~450 lines)
│   ├── messages.rs         # 27 commands (~550 lines)
│   ├── threads.rs          # 27 commands (~500 lines)
│   ├── reactions.rs        # 13 commands (~300 lines)
│   ├── members.rs          # 22 commands (~450 lines)
│   ├── roles.rs            # 17 commands (~400 lines)
│   ├── events.rs           # 15 commands (~350 lines)
│   ├── forum.rs            # 9 commands (~250 lines)
│   ├── moderation.rs       # bans, kicks, timeouts (~300 lines)
│   ├── server.rs           # 4 commands (~200 lines)
│   └── misc.rs             # webhooks, stickers, etc. (~250 lines)
└── commands.rs             # DELETE after migration
```

## Command Categories

### 1. channels.rs (23 commands)
- channels.list, channels.get, channels.create, channels.edit, channels.delete
- channels.get_or_create, channels.create_invite, channels.typing
- Plus thread-related channel operations

### 2. messages.rs (27 commands)
- messages.send, messages.edit, messages.delete, messages.get, messages.list
- messages.clear, messages.pin, messages.unpin, messages.bulk_delete

### 3. threads.rs (27 commands)
- threads.create, threads.list, threads.get, threads.edit, threads.delete
- threads.join, threads.leave
- threads.add_member, threads.remove_member

### 4. reactions.rs (13 commands)
- reactions.add, reactions.remove, reactions.list
- reactions.clear, reactions.clear_emoji

### 5. members.rs (22 commands)
- members.list, members.get, members.edit
- members.timeout, members.remove_timeout

### 6. roles.rs (17 commands)
- roles.list, roles.get, roles.create, roles.edit, roles.delete
- roles.assign, roles.remove

### 7. moderation.rs (bans + kicks)
- members.ban, members.unban, members.kick
- bans.list

### 8. events.rs (15 commands)
- events.list, events.get, events.create, events.edit, events.delete

### 9. forum.rs (9 commands)
- forum.create_post, forum.list_posts, forum.get_post

### 10. server.rs (4 commands)
- server.get_stats (main one)

### 11. misc.rs (remaining)
- webhooks.list
- stickers.list
- invites.list
- integrations.list
- emojis.list
- voice_regions.list

## Implementation Strategy

### Phase 1: Setup Structure
1. Create `src/discord/commands/` directory
2. Create `mod.rs` with executor shell and route function
3. Verify it compiles with empty modules

### Phase 2: Extract Categories (one at a time)
For each category module:
1. Create file with imports
2. Copy command functions from commands.rs
3. Make functions `pub(super)` 
4. Update mod.rs to route to new module
5. Run tests after each module
6. Commit after each successful module

### Phase 3: Cleanup
1. Delete old commands.rs
2. Update any external references
3. Final test run
4. Update documentation

## Module Template

```rust
//! Channel management commands.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serenity::all::Http;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// List all channels in a guild.
#[instrument(skip(http), fields(command = "channels.list"))]
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    // Implementation
}

/// Get specific channel details.
#[instrument(skip(http), fields(command = "channels.get"))]
pub(super) async fn get(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    // Implementation
}
```

## Routing Pattern (mod.rs)

```rust
impl BotCommandExecutor for DiscordCommandExecutor {
    type Error = BotCommandError;
    
    async fn execute(&self, command: &str, args: &HashMap<String, JsonValue>) 
        -> Result<JsonValue, Self::Error> 
    {
        match command {
            // Channels
            "channels.list" => channels::list(&self.http, args).await,
            "channels.get" => channels::get(&self.http, args).await,
            
            // Messages
            "messages.send" => messages::send(&self.http, args).await,
            
            // ... etc
            
            _ => Err(BotCommandError::new(
                BotCommandErrorKind::CommandNotFound(command.to_string())
            )),
        }
    }
}
```

## Testing Strategy

After each module extraction:
```bash
# Check compilation
cargo check -p botticelli_social

# Run tests
cargo test -p botticelli_social

# Verify no regressions
git diff --stat
```

## Success Criteria

- ✅ All 68 commands migrated
- ✅ No single file > 600 lines
- ✅ Each module has single responsibility
- ✅ All tests passing
- ✅ Zero clippy warnings
- ✅ Commands grouped by logical domain

## Estimated Effort

- Phase 1 (Setup): 30 minutes
- Phase 2 (11 modules): 2-3 hours (15-20 min per module)
- Phase 3 (Cleanup): 15 minutes
- **Total**: 3-4 hours

## Risks

- **Breaking change**: External code referencing private functions (unlikely - all private)
- **Import issues**: May need to add re-exports
- **Test failures**: Should catch immediately after each module

## Next Steps

1. Create directory structure
2. Start with smallest module (server.rs - 4 commands)
3. Validate pattern works
4. Continue with remaining modules
5. Clean up and commit

Ready to proceed!
