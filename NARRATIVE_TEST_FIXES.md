# Narrative Test Fixes Needed

## Overview
Multiple test narrative files are using the old `[[act]]` array syntax instead of the current `[toc]` + `[acts]` + friendly syntax pattern.

## Status: ✅ COMPLETE

All test narratives have been converted to the current TOML spec.

## Files Fixed

1. test_channel_commands.toml ✅
2. test_members.toml ✅  
3. test_message_commands.toml ✅
4. test_messages.toml ✅
5. test_reaction_commands.toml ✅
6. test_roles.toml ✅
7. test_server_commands.toml ✅
8. test_server_stats.toml ✅
9. test_role_commands.toml ✅
10. test_member_commands.toml ✅

## Already Correct

- test_channels_list.toml ✅
- test_guilds_get.toml ✅
- test_members_list.toml ✅
- test_messages_send.toml ✅
- test_roles_list.toml ✅

## Conversion Pattern

### OLD (deprecated):
```toml
[[act]]
name = "my_act"
prompt = "Do something"

[[act.input]]
type = "bot_command"
platform = "discord"
command = "roles.list"

[act.input.args]
guild_id = "${TEST_GUILD_ID}"
```

### NEW (current):
```toml
[toc]
order = ["my_act"]

[bots.my_cmd]
platform = "discord"
command = "roles.list"
guild_id = "${TEST_GUILD_ID}"

[acts]
my_act = "bots.my_cmd"
```

## Next Steps
1. Fix each file one by one following the pattern
2. Run tests to verify each fix
3. Update TEST_FIXES_NEEDED when complete
