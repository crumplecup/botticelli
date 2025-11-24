# Botticelli Context for LLMs

**Purpose**: This document provides context about Botticelli for AI content generation.

## What is Botticelli?

Botticelli is a Rust framework for orchestrating multi-step AI workflows called **narratives**. It enables complex LLM interactions through simple TOML configuration files, with support for Discord integration, database persistence, and content-addressable storage.

## Core Concepts

### Narratives
Multi-act workflows defined in TOML where each act executes sequentially with full context from previous acts.

**Example**: Generate → Critique → Refine → Publish

### Acts
Individual steps in a narrative. Each act can:
- Accept text, images, audio, video, documents
- Execute bot commands (Discord, etc.)
- Query database tables
- Reference previous act outputs via `{{act_name}}`

### Input Types
- **Text**: Plain text prompts
- **Media**: Images, audio, video (URL, base64, or file path)
- **BotCommand**: Execute platform commands (Discord server stats, user info, etc.)
- **Table**: Query database tables with filters, limits, ordering

## TOML Syntax

### Simple Narrative
```toml
[narrative]
name = "my_narrative"
description = "What it does"

[toc]
order = ["step1", "step2"]

[acts]
step1 = "Generate content about X"
step2 = "Refine {{step1}} for clarity"
```

### With Bot Commands
```toml
[bots.server_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123456789"

[acts]
analyze = ["bots.server_stats", "Analyze these metrics"]
```

### With Media
```toml
[media.logo]
file = "./logo.png"

[acts]
describe = ["media.logo", "Describe this image"]
```

## Discord Integration

### Bot Commands
Available via `Input::BotCommand`:
- `server.get_stats` - Guild statistics
- `server.get_channels` - Channel list
- `server.get_roles` - Role information
- `server.get_members` - Member list with filters
- `server.get_emoji` - Custom emoji
- `content.get_welcome_messages` - Database content

### Content Storage
Discord content stored in PostgreSQL:
- Guilds (servers) with member counts
- Channels (text, voice, categories)
- Users and guild members
- Roles and permissions
- Messages and attachments

## Actor System

**Actors** are autonomous bots that:
1. Query knowledge from database tables
2. Generate content using narratives
3. Post to social platforms (Discord, Twitter, Bluesky)
4. Schedule recurring workflows

**Example**: Daily showcase actor queries recent Discord messages, generates summary, posts to channel.

## Content Generation Best Practices

### For Discord
- Use Discord markdown (**, *, __, ~~, ||, ``)
- Keep under 2000 characters (Discord limit)
- Use emojis for visual appeal (🎭, 🚀, ✨, etc.)
- Structure with headers and bullet points
- Include calls-to-action

### For Narrative Design
- Start with broad generation
- Add critique/refinement steps
- Include context from bot commands
- Reference previous acts with `{{act_name}}`
- Use descriptive act names

### For Technical Content
- Explain features concisely
- Include practical examples
- Link concepts to user benefits
- Balance detail with readability

## Common Patterns

**Three-Act Structure**: Generate → Review → Polish  
**Research Pipeline**: Query → Analyze → Summarize → Format  
**Content Carousel**: Generate multiple variants → Select best → Refine  
**Community Showcase**: Fetch activity → Highlight → Compose post  

## Key Features to Mention

- **TOML-based workflows** - No code required
- **Multimodal support** - Text, images, audio, video, documents
- **Discord bot integration** - Query server data, post content
- **Database persistence** - Store executions and content
- **Rate limiting** - Automatic retry with exponential backoff
- **Actor automation** - Scheduled content generation
- **Content-addressable storage** - Efficient media management
- **Type-safe Rust** - Reliable execution

## Workspace Crates

- `botticelli_core` - Data structures (Input, Output, Message)
- `botticelli_narrative` - Execution engine
- `botticelli_models` - LLM providers (Gemini)
- `botticelli_social` - Discord integration
- `botticelli_database` - PostgreSQL persistence
- `botticelli_actor` - Social media automation
- `botticelli_storage` - Content-addressable files
- `botticelli_tui` - Terminal UI for review

## Tone Guidelines

**Enthusiastic but professional** - Highlight capabilities without hype  
**Clear and concise** - Technical accuracy, easy understanding  
**Community-focused** - Emphasize collaboration and exploration  
**Example-driven** - Show don't tell  

## Context Interpolation

Use `{{act_name}}` to reference previous outputs:
```toml
[acts]
research = "Find facts about X"
draft = "Write article using {{research}}"
polish = "Improve {{draft}} for readability"
```

## Error Handling

Narratives fail gracefully:
- Bot commands can be optional (`required = false`)
- Acts see context from successful steps
- Errors include location tracking
- Observability via `#[instrument]` tracing
