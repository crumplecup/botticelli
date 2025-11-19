# Example Narratives

This directory contains example narratives demonstrating how to use Botticelli's narrative system and processor pipeline.

## Available Narratives

### Content Generation Narratives (NEW)

These narratives demonstrate the content generation workflow - generating potential content for review before promoting to production.

#### generate_channel_posts.toml

Generates potential Discord channel configurations for review.

**Template:** discord_channels  
**Table:** potential_channel_posts

Generates 5 different channel types:
- Announcements channel (with slowmode)
- General chat channel (casual conversation)
- Tech support channel (with helpful guidelines)
- Showcase channel (for member projects)
- Off-topic channel (relaxed discussion)

**Complete Workflow:**

```bash
# 1. Generate content
botticelli run --narrative narratives/generate_channel_posts.toml

# 2. Review what was generated
botticelli content list potential_channel_posts

# 3. View a specific channel
botticelli content show potential_channel_posts 1

# 4. Rate your favorites
botticelli content tag potential_channel_posts 1 --rating 5 --tags "useful,professional"

# 5. Approve for production
botticelli content review potential_channel_posts 1 approved

# 6. Promote to production
botticelli content promote potential_channel_posts 1 --target discord_channels
```

#### generate_users.toml

Generates diverse user profiles for testing and demonstration.

**Template:** discord_users  
**Table:** potential_users

Generates 5 persona types:
- Active developer (helpful contributor)
- Helpful moderator (trusted community member)
- Creative artist (shares work and encourages)
- Enthusiastic gamer (social and fun)
- Community builder (organizes and welcomes)

**Workflow:**

```bash
# Generate profiles
botticelli run --narrative narratives/generate_users.toml

# Review and tag by persona
botticelli content list potential_users
botticelli content tag potential_users 2 --tags "moderator,trusted"

# Approve and promote
botticelli content review potential_users 2 approved
botticelli content promote potential_users 2 --target discord_users
```

#### generate_guilds.toml

Generates potential Discord guild (server) configurations.

**Template:** discord_guilds  
**Table:** potential_guilds

Generates 5 community types:
- Creative community (artists, writers, musicians)
- Tech learning hub (programming education)
- Gaming squad (competitive and social)
- Indie dev collective (game developer support)
- Study group (academic collaboration)

**Workflow:**

```bash
# Generate server ideas
botticelli run --narrative narratives/generate_guilds.toml

# Review and compare
botticelli content list potential_guilds
botticelli content show potential_guilds 1

# Tag by theme
botticelli content tag potential_guilds 1 --tags "creative,active" --rating 5

# Select and promote
botticelli content review potential_guilds 1 approved
botticelli content promote potential_guilds 1 --target discord_guilds
```

**Key Features:**
- Each guild has realistic member counts, descriptions, and features
- Diverse themes demonstrate different community types
- Professional descriptions that would attract real members

---

### Direct Insertion Narratives

These narratives use the `--process-discord` flag to insert directly into production tables.

### discord_infrastructure.toml

Creates basic Discord infrastructure in the database:

- **Guild (Server)**: Creates a demo Discord server
- **Bot User**: Creates a bot user account
- **Guild Member**: Adds the bot to the server
- **Channel**: Creates a text channel for content

**Run with:**

```bash
botticelli run --narrative narratives/discord_infrastructure.toml --process-discord
```

**What it demonstrates:**

- Using narrative preambles from DISCORD_NARRATIVE.md
- JSON generation following Discord schema
- Automatic processor pipeline with `--process-discord` flag
- Database insertion via Discord processors

**After running**, you can query the database to see the created entities:

```sql
-- View the created guild
SELECT * FROM discord_guilds WHERE id = 1100000000000000001;

-- View the bot user
SELECT * FROM discord_users WHERE id = 1100000000000000200;

-- View the guild membership
SELECT * FROM discord_guild_members
WHERE guild_id = 1100000000000000001
  AND user_id = 1100000000000000200;

-- View the channel
SELECT * FROM discord_channels WHERE id = 1100000000000000300;
```

### discord_content_examples.toml

Generates creative content examples for Discord bot posts:

- Daily motivational messages
- Technology tips and tutorials
- Creative writing prompts
- Community discussion questions
- Interesting facts

**Run with:**

```bash
botticelli run --narrative narratives/discord_content_examples.toml
```

**What it demonstrates:**

- Plain text generation (not JSON)
- Multiple themed acts in a single narrative
- Content variety for different use cases
- Creative prompt engineering

**Note:** This narrative generates content ideas but does NOT create Discord messages in the database. Discord messages are sent via the Discord API, not stored locally.

### test_minimal.toml

Minimal narrative for API testing and quota conservation.

## Narrative Format

All narratives use TOML format with this structure:

```toml
[narrative]
name = "Narrative Name"
description = "What this narrative does"

[toc]
order = ["act1", "act2", "act3"]

[acts]
act1 = "Prompt for first act"
act2 = "Prompt for second act"
act3 = "Prompt for third act"
```

## Using the Processor Pipeline

To enable automatic Discord data processing:

1. **Add the flag**: Use `--process-discord` when running the narrative
2. **Use proper JSON**: Follow the schema from DISCORD_NARRATIVE.md
3. **Check logs**: Use `RUST_LOG=botticelli=info` to see processor activity

**Example:**

```bash
RUST_LOG=botticelli=info botticelli run \
  --narrative narratives/discord_infrastructure.toml \
  --process-discord
```

You'll see logs showing:

- ✓ Registered 6 Discord processors
- Processing act with registered processors
- Processing Discord guilds/users/channels/etc.
- Successfully stored entities

## Creating Your Own Narratives

See [DISCORD_NARRATIVE.md](../DISCORD_NARRATIVE.md) for:

- Complete database schemas
- Required and optional fields
- Example JSON outputs
- Narrative preambles

See [NARRATIVE_PROCESSORS.md](../NARRATIVE_PROCESSORS.md) for:

- How processors work
- Extending with new processors
- Error handling
- Testing strategies

## Tips

1. **Test incrementally**: Start with one act, verify it works, then add more
2. **Check the database**: Query after running to verify data was inserted
3. **Use logging**: `RUST_LOG=botticelli=trace` shows detailed processor activity
4. **Handle errors**: Processor errors are logged but don't fail the narrative
5. **Unique IDs**: Use different Discord snowflake IDs for each entity to avoid conflicts

## Troubleshooting

**Processor doesn't run:**

- Check that you used `--process-discord` flag
- Verify JSON is valid (no markdown code blocks)
- Check `should_process()` logic matches your act name or JSON content

**Database errors:**

- Ensure foreign key relationships are correct (e.g., guild_id references existing guild)
- Check for unique constraint violations (duplicate IDs)
- Verify required fields are present

**JSON parsing errors:**

- Remove markdown code blocks (no ```json)
- Start with `{` and end with `}`
- Use `null` for optional fields, not missing keys
- Check for trailing commas

## Content Generation vs Direct Insertion

Botticelli supports two workflows for working with Discord data:

### Content Generation (Review Workflow)

**Use when:**
- You want to review and refine generated content before using it
- Generating multiple variations to choose from
- Creating content that needs approval or rating
- Planning and brainstorming configurations
- Testing different ideas without committing

**Process:**
1. Narrative has `template` field in `[narrative]`
2. Table named `<narrative_name>` is created automatically
3. Content stored with metadata (generated_at, review_status, rating, tags)
4. Review with `content list/show/tag/review` commands
5. Promote approved items with `content promote`

**Example narratives:**
- `generate_channel_posts.toml`
- `generate_users.toml`
- `generate_guilds.toml`

### Direct Insertion (Immediate Workflow)

**Use when:**
- You trust the LLM output completely
- Setting up infrastructure or test data quickly
- Following strict schemas that must be correct
- No review needed before database insertion

**Process:**
1. Narrative uses standard format (no `template` field)
2. Run with `--process-discord` flag
3. JSON is parsed and inserted immediately
4. Data goes directly to production tables (discord_guilds, discord_users, etc.)

**Example narratives:**
- `discord_infrastructure.toml`
- Any narrative with `--process-discord` flag

### Comparison

| Feature | Content Generation | Direct Insertion |
|---------|-------------------|------------------|
| **Review before use** | ✅ Yes | ❌ No |
| **Edit capability** | ✅ Via tag/review | ❌ DB only |
| **Multiple variations** | ✅ Generate many | ⚠️ One at a time |
| **Metadata tracking** | ✅ Full history | ❌ None |
| **Speed** | ⚠️ Multi-step | ✅ Immediate |
| **Use case** | Planning, testing | Infrastructure |

### Choosing Your Approach

**Go with Content Generation if:**
- "I want to see options and pick the best"
- "I need to refine the output"
- "Multiple people should review this"
- "I'm brainstorming ideas"

**Go with Direct Insertion if:**
- "I need test data now"
- "The schema is strict and well-defined"
- "I trust the LLM completely"
- "This is infrastructure setup"
