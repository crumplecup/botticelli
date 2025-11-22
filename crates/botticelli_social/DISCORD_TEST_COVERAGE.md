# Discord Command Test Coverage

Last Updated: 2025-11-22

## Test Statistics
- **Total Commands Implemented**: ~60
- **Commands with Tests**: 31
- **Parse Tests**: 31 (validate TOML syntax)
- **Integration Tests**: 8 (execute against Discord API)

## Tested Commands

### Server Commands
- ✅ server.get_stats (NEW)

### Channel Commands  
- ✅ channels.list
- ✅ channels.get
- ✅ channels.create
- ✅ channels.delete

### Message Commands
- ✅ messages.list
- ✅ messages.get
- ✅ messages.send
- ✅ messages.edit
- ✅ messages.delete
- ✅ messages.pin
- ✅ messages.unpin

### Member Commands
- ✅ members.list
- ✅ members.get

### Role Commands
- ✅ roles.list
- ✅ roles.get

### Reaction Commands
- ✅ reactions.add
- ✅ reactions.remove
- ✅ reactions.list (NEW)

### Thread Commands
- ✅ threads.list
- ✅ threads.create
- ✅ threads.get (NEW)

### Other Read Commands
- ✅ emojis.list
- ✅ events.list
- ✅ events.get (NEW)
- ✅ invites.list
- ✅ bans.list
- ✅ stickers.list
- ✅ voice_regions.list
- ✅ webhooks.list (NEW)
- ✅ integrations.list (NEW)

## Commands Needing Tests

### Write Commands (require state setup)
- ❌ channels.edit
- ❌ channels.get_or_create
- ❌ channels.create_invite
- ❌ channels.typing
- ❌ messages.clear
- ❌ messages.bulk_delete
- ❌ members.ban
- ❌ members.kick
- ❌ members.timeout
- ❌ members.unban
- ❌ members.edit
- ❌ members.remove_timeout
- ❌ roles.create
- ❌ roles.assign
- ❌ roles.remove
- ❌ roles.edit
- ❌ roles.delete
- ❌ threads.edit
- ❌ threads.delete
- ❌ threads.join
- ❌ threads.leave
- ❌ threads.add_member
- ❌ threads.remove_member
- ❌ reactions.clear
- ❌ reactions.clear_emoji
- ❌ forum.create_post
- ❌ forum.list_posts
- ❌ forum.get_post
- ❌ events.create
- ❌ events.edit
- ❌ events.delete

## Testing Strategy

### Parse Tests
Fast validation that narrative TOML files are syntactically correct. Run with:
```bash
cargo test --package botticelli_social --test discord_command_test parse_
```

### Integration Tests
Full execution tests that call Discord APIs. Require:
- `DISCORD_TOKEN` environment variable
- `TEST_GUILD_ID` environment variable
- Discord bot with appropriate permissions

Run with:
```bash
cargo test --package botticelli_social --test discord_command_test test_ --features discord -- --test-threads=1
```

### Test Narratives
All test narratives are in: `crates/botticelli_social/tests/narratives/discord/`

Naming convention: `{command}_test.toml` (e.g., `channels_list_test.toml`)

## Next Steps
1. Add tests for remaining read commands (forum, events mutations)
2. Implement state management tests for write commands
3. Add permission-checking tests with botticelli_security integration
