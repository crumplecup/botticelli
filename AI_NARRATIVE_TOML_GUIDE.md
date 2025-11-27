# AI Guide to Writing Narrative TOML Files

This guide helps AI assistants (and humans) write correct narrative TOML files by highlighting the most common mistakes and showing correct patterns from working tests.

**Always read `NARRATIVE_TOML_SPEC.md` first** - this guide supplements it with pitfall warnings based on real implementation experience.

---

## CRITICAL ERROR #1: Act Syntax

This is the #1 most common mistake. Get this right or nothing works.

### ❌ COMPLETELY WRONG - Never Use This
```toml
[[acts]]  # NEVER EVER use bare [[acts]]
name = "my_act"
prompt = "Do something"
```

### ✅ CORRECT - Acts Are Named Sections
```toml
[acts.my_act]  # Acts are NAMED SECTIONS with [acts.name]
prompt = "Do something"

[[acts.my_act.input]]  # Inputs are arrays UNDER the named act
type = "text"
content = "Input text"

[[acts.my_act.input]]  # Second input (array element)
type = "bot_command"
platform = "discord"
command = "channels.list"
```

### Key Rules (Read These Three Times)
1. **Acts are named sections**: `[acts.act_name]` (single bracket)
2. **Inputs are array elements** under the act: `[[acts.act_name.input]]` (double bracket)
3. **NEVER EVER** use `[[acts]]` or `[[acts.input]]` directly
4. The act name in `[acts.act_name]` must match what's in `[toc]` order

### Working Example from Real Tests
```toml
[narrative]
name = "test_channels_list"
description = "Test listing guild channels"
skip_content_generation = true

[toc]
order = ["list_channels"]  # Act name referenced here

[acts.list_channels]  # CORRECT: [acts.act_name]

[[acts.list_channels.input]]  # CORRECT: [[acts.act_name.input]]
type = "bot_command"
platform = "discord"
command = "channels.list"
required = true

[acts.list_channels.input.args]
guild_id = "${TEST_GUILD_ID}"
```

---

## CRITICAL ERROR #2: Non-Existent Environment Variables

### ❌ WRONG - Inventing Variables That Don't Exist
```toml
[acts.create_channel]
[[acts.create_channel.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"

[acts.create_channel.input.args]
guild_id = "${TEST_GUILD_ID}"    # ✅ This exists in .env
channel_id = "${TEST_CHANNEL_ID}"  # ❌ Does NOT exist!
name = "${CHANNEL_NAME}"           # ❌ Does NOT exist!
```

### ✅ CORRECT - Only Use Real Env Vars OR State Management

**Option 1: Use only environment variables that actually exist**
```toml
[acts.create_channel]
[[acts.create_channel.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"

[acts.create_channel.input.args]
guild_id = "${TEST_GUILD_ID}"  # ✅ Exists in .env
name = "test-channel"          # ✅ Literal value
```

**Option 2: Create resources and cache IDs in state management**
```toml
[toc]
order = ["create_channel", "use_channel"]

# First act: Create channel and cache its ID
[acts.create_channel]
[[acts.create_channel.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"

[acts.create_channel.input.args]
guild_id = "${TEST_GUILD_ID}"
name = "test-channel"

[acts.create_channel.input.state]
cache_key = "test_channel_id"  # Save ID for later
extract_field = "id"

# Second act: Use the cached ID
[acts.use_channel]
[[acts.use_channel.input]]
type = "bot_command"
platform = "discord"
command = "channels.get"

[acts.use_channel.input.args]
guild_id = "${TEST_GUILD_ID}"
channel_id = "${test_channel_id}"  # ✅ From state management
```

### How Environment Variable Expansion Works

Botticelli automatically expands environment variables in bot command arguments using the `shellexpand` crate:
- Supports `${VAR_NAME}` and `$VAR_NAME` syntax
- Variables are loaded from `.env` file or exported in shell
- Expansion happens at parse time when loading the narrative
- If a variable doesn't exist, the expansion fails silently (keeps original string)

### Environment Variables That Actually Exist
- `${TEST_GUILD_ID}` - ✅ Real, defined in `.env`
- `${DISCORD_TOKEN}` - ✅ Real, defined in `.env`
- `${GEMINI_API_KEY}` - ✅ Real, defined in `.env`

**Everything else must be:**
- A literal value: `"test-channel"`
- Created and cached via state management: `${cached_key_name}` (retrieved from state, NOT env vars)

---

## CRITICAL ERROR #3: State Management Pattern

### The Correct Pattern for Creating and Reusing IDs

```toml
[toc]
order = ["setup", "test", "cleanup"]

# ACT 1: Create resource, cache ID
[acts.setup]
[[acts.setup.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"

[acts.setup.input.args]
guild_id = "${TEST_GUILD_ID}"
name = "test-channel"

[acts.setup.input.state]
cache_key = "my_channel_id"  # Choose a descriptive key
extract_field = "id"          # Extract 'id' field from response

# ACT 2: Use cached ID
[acts.test]
[[acts.test.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"

[acts.test.input.args]
channel_id = "${my_channel_id}"  # Reference cached ID
content = "Test message"

# ACT 3: Cleanup using same cached ID
[acts.cleanup]
[[acts.cleanup.input]]
type = "bot_command"
platform = "discord"
command = "channels.delete"

[acts.cleanup.input.args]
channel_id = "${my_channel_id}"  # Same cached ID
```

---

## Common Working Patterns from Real Tests

### Pattern 1: Simple Read Command
```toml
[narrative]
name = "test_guilds_get"
description = "Test getting guild info"
skip_content_generation = true

[toc]
order = ["get_guild"]

[acts.get_guild]
[[acts.get_guild.input]]
type = "bot_command"
platform = "discord"
command = "guilds.get"
required = true

[acts.get_guild.input.args]
guild_id = "${TEST_GUILD_ID}"
```

### Pattern 2: Create-Use-Delete Workflow
```toml
[narrative]
name = "test_channel_lifecycle"
description = "Create, use, and delete a channel"
skip_content_generation = true

[toc]
order = ["create", "send_message", "delete"]

[acts.create]
[[acts.create.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"

[acts.create.input.args]
guild_id = "${TEST_GUILD_ID}"
name = "temp-test-channel"
type = "text"

[acts.create.input.state]
cache_key = "temp_channel_id"
extract_field = "id"

[acts.send_message]
[[acts.send_message.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"

[acts.send_message.input.args]
channel_id = "${temp_channel_id}"
content = "Test message"

[acts.send_message.input.state]
cache_key = "test_message_id"
extract_field = "id"

[acts.delete]
[[acts.delete.input]]
type = "bot_command"
platform = "discord"
command = "channels.delete"

[acts.delete.input.args]
channel_id = "${temp_channel_id}"
```

---

## Token Optimization with History Retention

In multi-act narratives, large inputs (especially table queries and bot commands) can cause token explosion as they're re-sent with every subsequent act. Use `history_retention` to control this behavior and reduce token usage by 80%+.

### ❌ INEFFICIENT - Large Data in Multi-Act Narrative
```toml
[toc]
order = ["fetch_data", "analyze", "decide", "execute"]

[tables.large_dataset]
table_name = "analytics"
limit = 1000  # Returns 500KB of JSON
# No history_retention specified - defaults to "full"

[acts]
fetch_data = "tables.large_dataset"    # Act 1: Uses 500KB
analyze = "Analyze the data"           # Act 2: Re-sends 500KB
decide = "Make a decision"             # Act 3: Re-sends 500KB again
execute = "Execute the decision"       # Act 4: Re-sends 500KB yet again
# Total: 2,000KB (4x the data size!) across all acts
```

### ✅ OPTIMIZED - Use history_retention = "summary"
```toml
[toc]
order = ["fetch_data", "analyze", "decide", "execute"]

[tables.large_dataset]
table_name = "analytics"
limit = 1000
history_retention = "summary"  # Replace with concise summary after Act 1

[acts]
fetch_data = "tables.large_dataset"    # Act 1: Uses 500KB
analyze = "Analyze the data"           # Act 2: Sees "[Table: analytics, 1000 rows]" (~30 bytes)
decide = "Make a decision"             # Act 3: Sees summary (~30 bytes)
execute = "Execute the decision"       # Act 4: Sees summary (~30 bytes)
# Total: ~500KB (just the initial data!) - 75% reduction
```

### History Retention Options

- **`history_retention = "full"`** (default)
  - Retains entire input unchanged
  - Use for: Single-act narratives, small data (< 5KB), or when subsequent acts need to re-examine raw data

- **`history_retention = "summary"`**
  - Replaces input with concise summary like `[Table: name, N rows]` after processing
  - Use for: Multi-act narratives with large data where subsequent acts only need the decision/result
  - Typical savings: 80%+ token reduction

- **`history_retention = "drop"`**
  - Completely removes input from history after processing
  - Use for: Maximum token savings when input is truly one-time
  - Warning: Use carefully - subsequent acts won't know the input existed

### Applying to Different Input Types

**Tables:**
```toml
[tables.large_query]
table_name = "events"
limit = 500
history_retention = "summary"
```

**Bot Commands:**
```toml
[bots.fetch_members]
platform = "discord"
command = "members.list"
guild_id = "123"
history_retention = "drop"  # Just needed for initial count, don't keep in history
```

**Narratives:**
```toml
# In verbose syntax only
[[acts.preprocess.input]]
type = "narrative"
name = "data_preparation"
history_retention = "summary"
```

### Auto-Summarization Safety

Inputs exceeding 10KB are automatically summarized even with `history_retention = "full"` to prevent token overflow. This is a safety guardrail - you don't need to do anything.

### When to Use History Retention

✅ **DO use `history_retention = "summary"` when:**
- Multi-act narratives (3+ acts)
- Large table queries (100+ rows or > 5KB)
- Bot commands returning large datasets
- Subsequent acts only need to know "what was decided" not "what the raw data was"

❌ **DON'T use history retention when:**
- Single-act narratives (no subsequent acts to optimize)
- Small data (< 5KB)
- Subsequent acts need to reference specific data points from the input
- You're unsure - stick with the default `"full"`

---

## Checklist Before Writing a Narrative

1. ✅ Am I using `[acts.act_name]` (NOT `[[acts]]`)?
2. ✅ Am I using `[[acts.act_name.input]]` for inputs?
3. ✅ Are my act names in `[toc]` order matching the `[acts.name]` sections?
4. ✅ Am I only using environment variables that exist (`TEST_GUILD_ID`, `DISCORD_TOKEN`)?
5. ✅ If I need an ID that doesn't exist, am I creating it in a setup act and caching it?
6. ✅ Am I using state management with `cache_key` and `extract_field`?
7. ✅ Have I included cleanup/teardown acts to delete created resources?

---

## When In Doubt

**Look at working test examples in:**
- `crates/botticelli_social/tests/narratives/discord/*.toml`
- `crates/botticelli_narrative/narratives/discord/*.toml`

**Copy the structure from a working test** and modify it for your needs. Don't try to invent new syntax.
