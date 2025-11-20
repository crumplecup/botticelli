# Narrative Examples

This directory contains example narratives demonstrating various Botticelli features.

## Bot Command Execution

### discord_bot_commands.toml

Demonstrates bot command execution (Phase 2 feature) by:
1. Querying Discord server for real-time statistics
2. Analyzing the community data with an LLM
3. Generating an engaging community update post

**Prerequisites**:
- Discord bot token in `DISCORD_BOT_TOKEN` environment variable
- Bot must be a member of the target Discord server
- Bot needs these permissions: `READ_MESSAGES`, `VIEW_CHANNEL`

**Usage**:
1. Replace `YOUR_GUILD_ID_HERE` with your Discord server ID
2. Run with: `cargo run -- narrative execute examples/narratives/discord_bot_commands.toml`

**What it demonstrates**:
- Bot command resources (`[bots.name]` sections)
- Multi-command execution in a single act
- JSON data processing by LLM
- Three-act narrative flow (fetch → analyze → generate)

## Future Examples

- `table_references.toml` - Query database tables (Phase 3)
- `media_generation.toml` - Generate and process images
- `multi_platform.toml` - Combine Discord + Slack commands
