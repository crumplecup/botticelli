<!-- markdownlint-disable MD024 -->
# Discord Narrative Preambles

This document provides narrative preambles for generating content to populate Discord database tables. Each preamble includes the database schema and guidance for LLM-generated content that conforms to Discord's data model.

## Overview

When creating narratives to generate Discord content, use these preambles to ensure the LLM outputs conform to the database schema. Each table has specific field types, constraints, and relationships that must be respected.

## Table of Contents

1. [Discord Guilds (Servers)](#discord-guilds-servers)
2. [Discord Users](#discord-users)
3. [Discord Channels](#discord-channels)
4. [Discord Guild Members](#discord-guild-members)
5. [Discord Roles](#discord-roles)
6. [Discord Member Roles](#discord-member-roles)

---

## Discord Guilds (Servers)

### Purpose
Discord guilds represent servers - the top-level communities where all Discord activity happens. Each guild contains channels, roles, and members.

### Database Schema

```sql
CREATE TABLE discord_guilds (
    id BIGINT PRIMARY KEY,                    -- Discord snowflake ID (must be unique)
    name VARCHAR(100) NOT NULL,               -- Guild name (required, max 100 chars)
    icon VARCHAR(255),                        -- Icon hash (optional)
    banner VARCHAR(255),                      -- Banner hash (optional)
    splash VARCHAR(255),                      -- Splash screen hash (optional)
    owner_id BIGINT NOT NULL,                 -- User ID of guild owner (required)

    features TEXT[],                          -- Array of feature flags (e.g., 'COMMUNITY', 'VERIFIED')
    description TEXT,                         -- Guild description
    vanity_url_code VARCHAR(50),              -- Custom invite URL code (e.g., 'discord-api')

    member_count INTEGER,                     -- Total member count
    approximate_member_count INTEGER,         -- Approximate online members
    approximate_presence_count INTEGER,       -- Approximate online presences

    afk_channel_id BIGINT,                    -- AFK voice channel ID
    afk_timeout INTEGER,                      -- AFK timeout in seconds
    system_channel_id BIGINT,                 -- System messages channel ID
    rules_channel_id BIGINT,                  -- Rules channel ID
    public_updates_channel_id BIGINT,         -- Public updates channel ID

    verification_level SMALLINT,              -- 0-4 (none to highest)
    explicit_content_filter SMALLINT,         -- 0-2 (disabled to all members)
    mfa_level SMALLINT,                       -- 0-1 (none or required)

    premium_tier SMALLINT,                    -- 0-3 (none, tier 1-3)
    premium_subscription_count INTEGER,       -- Number of boosts

    max_presences INTEGER,                    -- Max presences (usually 25000)
    max_members INTEGER,                      -- Max members
    max_video_channel_users INTEGER,          -- Max video channel users

    large BOOLEAN DEFAULT FALSE,              -- True if >250 members
    unavailable BOOLEAN DEFAULT FALSE,        -- True if server is unavailable

    joined_at TIMESTAMP,                      -- When bot joined
    created_at TIMESTAMP NOT NULL,            -- Record creation timestamp
    updated_at TIMESTAMP NOT NULL,            -- Last update timestamp
    left_at TIMESTAMP,                        -- When bot left (NULL if active)

    bot_permissions BIGINT,                   -- Bot's permission bitfield
    bot_active BOOLEAN DEFAULT TRUE           -- Whether bot is active in guild
);
```

### Narrative Preamble

```markdown
You are generating data for a Discord server (guild). Please provide the following information conforming to this schema:

**Required Fields:**
- id: A unique 18-digit Discord snowflake ID (e.g., 1234567890123456789)
- name: Server name (max 100 characters)
- owner_id: Discord user ID of the server owner (18-digit snowflake)

**Optional Fields:**
- icon: Icon hash (hex string)
- description: Server description (text)
- member_count: Total number of members (integer)
- verification_level: Security level 0-4 (0=none, 1=low, 2=medium, 3=high, 4=highest)
- premium_tier: Boost level 0-3 (0=none, 1=tier 1, 2=tier 2, 3=tier 3)
- features: Array of feature flags (e.g., ["COMMUNITY", "DISCOVERABLE", "VERIFIED"])

**Example Output:**
{
  "id": 1234567890123456789,
  "name": "Awesome Gaming Community",
  "owner_id": 9876543210987654321,
  "description": "A friendly community for gamers of all skill levels",
  "member_count": 1500,
  "verification_level": 2,
  "premium_tier": 2,
  "features": ["COMMUNITY", "DISCOVERABLE"]
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- Start your response with { and end with }
- Use null for optional fields you don't want to set
```

---

## Discord Users

### Purpose
Discord users represent individual accounts. This table tracks user profile information across all guilds the bot can see.

### Database Schema

```sql
CREATE TABLE discord_users (
    id BIGINT PRIMARY KEY,                    -- Discord snowflake ID
    username VARCHAR(32) NOT NULL,            -- Username (required, max 32 chars)
    discriminator VARCHAR(4),                 -- Legacy #1234 discriminator (nullable)
    global_name VARCHAR(32),                  -- Display name (optional)
    avatar VARCHAR(255),                      -- Avatar hash (optional)
    banner VARCHAR(255),                      -- Profile banner hash (optional)
    accent_color INTEGER,                     -- Accent color as integer (optional)

    bot BOOLEAN DEFAULT FALSE,                -- True if bot account
    system BOOLEAN DEFAULT FALSE,             -- True if system account
    mfa_enabled BOOLEAN,                      -- True if 2FA enabled
    verified BOOLEAN,                         -- True if email verified

    premium_type SMALLINT,                    -- 0=none, 1=Nitro Classic, 2=Nitro, 3=Nitro Basic
    public_flags INTEGER,                     -- Public user flags bitfield

    locale VARCHAR(10),                       -- User locale (e.g., 'en-US')

    first_seen TIMESTAMP NOT NULL,            -- When first seen by bot
    last_seen TIMESTAMP NOT NULL,             -- Last activity timestamp
    created_at TIMESTAMP NOT NULL,            -- Record creation timestamp
    updated_at TIMESTAMP NOT NULL             -- Last update timestamp
);
```

### Narrative Preamble

```markdown
You are generating data for Discord user profiles. Please provide the following information conforming to this schema:

**Required Fields:**
- id: Unique 18-digit Discord snowflake ID (e.g., 1234567890123456789)
- username: Username without @ (max 32 characters, lowercase, no spaces)

**Optional Fields:**
- global_name: Display name (max 32 characters, can include spaces and mixed case)
- discriminator: Legacy 4-digit discriminator (e.g., "0001") - leave null for new usernames
- avatar: Avatar hash (hex string)
- bot: Set to true for bot accounts, false for users (default: false)
- premium_type: Nitro subscription 0-3 (0=none, 1=Nitro Classic, 2=Nitro, 3=Nitro Basic)
- locale: Language code (e.g., "en-US", "es-ES", "ja")

**Example Output:**
{
  "id": 1234567890123456789,
  "username": "cooluser123",
  "global_name": "Cool User",
  "discriminator": null,
  "bot": false,
  "premium_type": 2,
  "locale": "en-US"
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- Start your response with { and end with }
- Use null for optional fields you don't want to set
```

---

## Discord Channels

### Purpose
Discord channels represent text channels, voice channels, threads, forums, and categories within a guild. They organize guild communication.

### Database Schema

```sql
CREATE TYPE discord_channel_type AS ENUM (
    'guild_text',           -- Text channel
    'dm',                   -- Direct message
    'guild_voice',          -- Voice channel
    'group_dm',             -- Group DM
    'guild_category',       -- Category
    'guild_announcement',   -- Announcement channel
    'announcement_thread',  -- Announcement thread
    'public_thread',        -- Public thread
    'private_thread',       -- Private thread
    'guild_stage_voice',    -- Stage channel
    'guild_directory',      -- Directory channel
    'guild_forum',          -- Forum channel
    'guild_media'           -- Media channel
);

CREATE TABLE discord_channels (
    id BIGINT PRIMARY KEY,                    -- Discord snowflake ID
    guild_id BIGINT,                          -- Parent guild ID (nullable for DMs)
    name VARCHAR(100),                        -- Channel name
    channel_type discord_channel_type NOT NULL, -- Channel type (required)
    position INTEGER,                         -- Sort position in channel list

    topic TEXT,                               -- Channel topic/description

    nsfw BOOLEAN DEFAULT FALSE,               -- Age-restricted content
    rate_limit_per_user INTEGER DEFAULT 0,    -- Slowmode seconds
    bitrate INTEGER,                          -- Voice channel bitrate
    user_limit INTEGER,                       -- Voice channel user limit

    parent_id BIGINT,                         -- Parent category or channel ID
    owner_id BIGINT,                          -- Thread owner ID
    message_count INTEGER,                    -- Thread message count
    member_count INTEGER,                     -- Thread member count
    archived BOOLEAN DEFAULT FALSE,           -- Thread archived status
    auto_archive_duration INTEGER,            -- Thread auto-archive minutes
    archive_timestamp TIMESTAMP,              -- When thread was archived
    locked BOOLEAN DEFAULT FALSE,             -- Thread locked status
    invitable BOOLEAN DEFAULT TRUE,           -- Thread invite permission

    available_tags JSONB,                     -- Forum tags
    default_reaction_emoji JSONB,             -- Forum default emoji
    default_thread_rate_limit INTEGER,        -- Forum thread slowmode
    default_sort_order SMALLINT,              -- Forum sort order
    default_forum_layout SMALLINT,            -- Forum layout style

    created_at TIMESTAMP NOT NULL,            -- Record creation timestamp
    updated_at TIMESTAMP NOT NULL,            -- Last update timestamp
    last_message_at TIMESTAMP,                -- Last message timestamp

    last_read_message_id BIGINT,              -- Bot's last read message
    bot_has_access BOOLEAN DEFAULT TRUE       -- Whether bot can access
);
```

### Narrative Preamble

```markdown
You are generating data for Discord channels. Please provide the following information conforming to this schema:

**Required Fields:**
- id: Unique 18-digit Discord snowflake ID
- channel_type: One of: 'guild_text', 'guild_voice', 'guild_category', 'guild_announcement', 'public_thread', 'private_thread', 'guild_stage_voice', 'guild_forum', 'guild_media'
- guild_id: Parent guild ID (required for guild channels)

**Common Optional Fields:**
- name: Channel name (max 100 chars, lowercase-with-dashes for text channels)
- topic: Channel description or topic
- position: Sort order (0-based integer)
- parent_id: Parent category ID (for channels) or parent channel ID (for threads)
- nsfw: True if age-restricted (default: false)

**Text Channel Fields:**
- rate_limit_per_user: Slowmode in seconds (0-21600)

**Voice Channel Fields:**
- bitrate: Audio quality in bits (8000-384000)
- user_limit: Max users (0-99, 0=unlimited)

**Thread Fields:**
- owner_id: Thread creator user ID
- archived: Thread archived status (default: false)
- auto_archive_duration: Auto-archive after minutes (60, 1440, 4320, 10080)

**Example Output (Text Channel):**
{
  "id": 1234567890123456789,
  "guild_id": 9876543210987654321,
  "name": "general-chat",
  "channel_type": "guild_text",
  "topic": "General discussion for all topics",
  "position": 0,
  "parent_id": 1111111111111111111,
  "nsfw": false,
  "rate_limit_per_user": 0
}

**Example Output (Voice Channel):**
{
  "id": 2222222222222222222,
  "guild_id": 9876543210987654321,
  "name": "Voice Lounge",
  "channel_type": "guild_voice",
  "position": 1,
  "parent_id": 1111111111111111111,
  "bitrate": 64000,
  "user_limit": 10
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- For multiple channels, use a JSON array: [{...}, {...}]
- Start your response with [ or { and end with ] or }
- Use null for optional fields you don't want to set
```

---

## Discord Guild Members

### Purpose
Guild members represent the many-to-many relationship between users and guilds. A user can be a member of multiple guilds, and each guild has many members. This table captures guild-specific user data.

### Database Schema

```sql
CREATE TABLE discord_guild_members (
    guild_id BIGINT,                          -- Guild ID (composite key)
    user_id BIGINT,                           -- User ID (composite key)

    nick VARCHAR(32),                         -- Guild-specific nickname
    avatar VARCHAR(255),                      -- Guild-specific avatar hash

    joined_at TIMESTAMP NOT NULL,             -- When user joined guild
    premium_since TIMESTAMP,                  -- Server boost start date
    communication_disabled_until TIMESTAMP,   -- Timeout expiration

    deaf BOOLEAN DEFAULT FALSE,               -- Server deafened
    mute BOOLEAN DEFAULT FALSE,               -- Server muted
    pending BOOLEAN DEFAULT FALSE,            -- Passed membership screening

    created_at TIMESTAMP NOT NULL,            -- Record creation timestamp
    updated_at TIMESTAMP NOT NULL,            -- Last update timestamp
    left_at TIMESTAMP,                        -- When user left (NULL if active)

    PRIMARY KEY (guild_id, user_id)
);
```

### Narrative Preamble

```markdown
You are generating data for Discord guild members (the relationship between users and servers). Please provide the following information conforming to this schema:

**Required Fields:**
- guild_id: Guild (server) ID the user belongs to
- user_id: User ID being added to the guild
- joined_at: ISO 8601 timestamp when user joined (e.g., "2024-01-15T14:30:00Z")

**Optional Fields:**
- nick: Guild-specific nickname (max 32 characters, different from username)
- avatar: Guild-specific avatar hash (overrides user avatar in this guild)
- premium_since: ISO timestamp when user started boosting server (null if not boosting)
- communication_disabled_until: ISO timestamp when timeout expires (null if not timed out)
- deaf: Server deafened status (default: false)
- mute: Server muted status (default: false)
- pending: Whether user is pending membership screening (default: false)

**Example Output:**
{
  "guild_id": 9876543210987654321,
  "user_id": 1234567890123456789,
  "nick": "CoolNickname",
  "joined_at": "2024-01-15T14:30:00Z",
  "premium_since": "2024-03-01T10:00:00Z",
  "deaf": false,
  "mute": false,
  "pending": false
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- For multiple members, use a JSON array: [{...}, {...}]
- Use ISO 8601 format for all timestamps
- Use null for optional fields you don't want to set
```

---

## Discord Roles

### Purpose
Roles define permissions and visual hierarchy within a guild. Members can have multiple roles, with permissions and colors stacking.

### Database Schema

```sql
CREATE TABLE discord_roles (
    id BIGINT PRIMARY KEY,                    -- Discord snowflake ID
    guild_id BIGINT NOT NULL,                 -- Parent guild ID
    name VARCHAR(100) NOT NULL,               -- Role name (required)
    color INTEGER NOT NULL DEFAULT 0,         -- RGB color as integer (0=no color)
    hoist BOOLEAN DEFAULT FALSE,              -- Display separately in member list
    icon VARCHAR(255),                        -- Role icon hash
    unicode_emoji VARCHAR(100),               -- Unicode emoji for role
    position INTEGER NOT NULL,                -- Role hierarchy position (0-based)
    permissions BIGINT NOT NULL,              -- Permission bitfield
    managed BOOLEAN DEFAULT FALSE,            -- Managed by integration (bot role)
    mentionable BOOLEAN DEFAULT FALSE,        -- Can be @mentioned

    tags JSONB,                               -- Role tags (bot_id, integration_id, etc.)

    created_at TIMESTAMP NOT NULL,            -- Record creation timestamp
    updated_at TIMESTAMP NOT NULL             -- Last update timestamp
);
```

### Narrative Preamble

```markdown
You are generating data for Discord roles. Please provide the following information conforming to this schema:

**Required Fields:**
- id: Unique 18-digit Discord snowflake ID
- guild_id: Parent guild ID this role belongs to
- name: Role name (max 100 characters)
- position: Role hierarchy (0=lowest, higher numbers = more powerful)
- permissions: Permission bitfield as integer (0 for no permissions, 8 for admin, etc.)

**Optional Fields:**
- color: RGB color as decimal integer (e.g., 16711680 for red #FF0000, 0 for no color)
- hoist: Display separately in member list (default: false)
- icon: Role icon hash (for boosted servers)
- unicode_emoji: Unicode emoji for role (e.g., "🎮")
- managed: True if role is managed by bot/integration (default: false)
- mentionable: True if @role works (default: false)

**Common Permission Values:**
- 0: No permissions
- 8: Administrator
- 2048: Send Messages
- 8192: Manage Messages
- 16: Manage Channels
- 268435456: View Channels

**Example Output:**
{
  "id": 3333333333333333333,
  "guild_id": 9876543210987654321,
  "name": "Moderators",
  "color": 3447003,
  "hoist": true,
  "position": 5,
  "permissions": 8,
  "managed": false,
  "mentionable": true,
  "unicode_emoji": "🛡️"
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- For multiple roles, use a JSON array: [{...}, {...}]
- Color must be decimal integer (not hex string)
- Use null for optional fields you don't want to set
```

---

## Discord Member Roles

### Purpose
Member roles represent the many-to-many relationship between guild members and roles. A member can have multiple roles, and a role can be assigned to multiple members.

### Database Schema

```sql
CREATE TABLE discord_member_roles (
    guild_id BIGINT,                          -- Guild ID (composite key)
    user_id BIGINT,                           -- User ID (composite key)
    role_id BIGINT,                           -- Role ID (composite key)

    assigned_at TIMESTAMP NOT NULL,           -- When role was assigned
    assigned_by BIGINT,                       -- User ID who assigned the role

    PRIMARY KEY (guild_id, user_id, role_id)
);
```

### Narrative Preamble

```markdown
You are generating data for Discord member role assignments (which users have which roles). Please provide the following information conforming to this schema:

**Required Fields:**
- guild_id: Guild ID where the role is being assigned
- user_id: User ID receiving the role
- role_id: Role ID being assigned
- assigned_at: ISO 8601 timestamp when role was assigned (e.g., "2024-01-15T14:30:00Z")

**Optional Fields:**
- assigned_by: User ID of the person who assigned the role (null if assigned by system)

**Notes:**
- The combination of (guild_id, user_id, role_id) must be unique
- Ensure the guild_member exists (user must be in guild)
- Ensure the role exists in the specified guild
- Everyone role (@everyone) usually has same ID as guild_id

**Example Output:**
{
  "guild_id": 9876543210987654321,
  "user_id": 1234567890123456789,
  "role_id": 3333333333333333333,
  "assigned_at": "2024-01-20T16:45:00Z",
  "assigned_by": 5555555555555555555
}

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- For multiple role assignments, use a JSON array: [{...}, {...}]
- Use ISO 8601 format for timestamps
- Use null for assigned_by if assigned by system
```

---

## Output Format Recommendations

### JSON (Recommended for single records)

**Advantages:**
- Standard format, easy to validate
- Direct deserialization to Rust structs
- Well-supported by all LLMs

**Best practices:**
- Request JSON-only output (no markdown, no commentary)
- Use structured output mode if supported by the LLM API
- Implement extraction for responses with code blocks or extra text

**Example instruction:**
```
CRITICAL: Output ONLY valid JSON with no additional text.
Do not use markdown code blocks (no ```json).
Do not add explanations before or after the JSON.
Start your response with { and end with }.
```

### TOML (Recommended for multiple records)

**Advantages:**
- More human-readable than JSON
- Handles multi-record generation naturally
- Less escaping issues with strings
- Already used in Boticelli narrative definitions

**Example instruction:**
```
Output data in TOML format only.
Do not use code blocks or markdown.
Follow this exact structure: [examples]
```

### Extracting JSON from LLM Responses

LLMs often wrap JSON in markdown or add commentary. Implement extraction logic:

```rust
// Pseudocode for extraction
fn extract_json(response: &str) -> Result<String> {
    // 1. Check for markdown code blocks: ```json ... ```
    if let Some(json) = extract_from_code_block(response) {
        return Ok(json);
    }

    // 2. Find balanced braces: { ... }
    if let Some(json) = extract_balanced_braces(response) {
        return Ok(json);
    }

    // 3. Return error if no JSON found
    Err("No JSON found in response")
}
```

## Usage in Narratives

### Example Narrative Structure

When creating a narrative to generate Discord data, structure it like this:

```toml
[narration]
name = "discord_server_generation"
description = "Generate a fictional Discord server with channels and members"

[toc]
order = ["setup_guild", "create_channels", "create_roles"]

[acts.setup_guild]
model = "gemini-2.0-flash-lite"

[[acts.setup_guild.input]]
type = "text"
content = """
[Insert Discord Guilds preamble here]

Generate a gaming community server called "Epic Gamers Hub" with 500 members and tier 2 boost status.

CRITICAL: Output ONLY valid JSON with no additional text, markdown, or code blocks.
Start your response with { and end with }.
"""

[acts.create_channels]
model = "gemini-2.0-flash-lite"

[[acts.create_channels.input]]
type = "text"
content = """
[Insert Discord Channels preamble here]

Create the following channels for the guild (ID: {guild_id from previous act}):
1. A "rules" text channel
2. A "general-chat" text channel
3. A "gaming-lounge" voice channel
4. A "Community" category containing the channels above

Output as a JSON array: [{...}, {...}]
CRITICAL: Output ONLY valid JSON with no additional text or markdown.
"""

[acts.create_roles]
model = "gemini-2.0-flash-lite"

[[acts.create_roles.input]]
type = "text"
content = """
[Insert Discord Roles preamble here]

Create these roles for the guild:
1. "Admin" - red color, admin permissions
2. "Moderator" - blue color, manage messages permission
3. "Member" - no color, basic permissions

Output as a JSON array: [{...}, {...}, {...}]
CRITICAL: Output ONLY valid JSON with no additional text or markdown.
"""
```

### Best Practices

1. **Sequential Dependencies**: Create guilds before channels, users before guild members, roles before member roles
2. **Consistent IDs**: Use the generated IDs from previous acts when referencing entities
3. **Realistic Data**: Generate realistic snowflake IDs (18-digit numbers), timestamps, and member counts
4. **Validation**: Ensure foreign key relationships are valid (e.g., channel's guild_id matches an existing guild)
5. **Timestamps**: Use ISO 8601 format for all timestamps (e.g., "2024-01-15T14:30:00Z")

### Snowflake ID Generation

Discord uses Twitter-style snowflake IDs (64-bit integers). When generating IDs:
- Use 18-digit numbers (e.g., 1234567890123456789)
- Ensure uniqueness across the same table
- Later IDs should be larger than earlier IDs (they encode timestamps)

---

## Complete Example: Building a Server

Here's how to structure a multi-act narrative that builds a complete Discord server:

**Act 1: Create Guild**
```json
{
  "id": 1000000000000000001,
  "name": "Cozy Café",
  "owner_id": 2000000000000000001,
  "description": "A warm community for coffee lovers",
  "member_count": 250
}
```

**Act 2: Create Owner User**
```json
{
  "id": 2000000000000000001,
  "username": "cafeowner",
  "global_name": "Café Owner",
  "bot": false
}
```

**Act 3: Create Category Channel**
```json
{
  "id": 3000000000000000001,
  "guild_id": 1000000000000000001,
  "name": "MAIN LOBBY",
  "channel_type": "guild_category",
  "position": 0
}
```

**Act 4: Create Text Channels**
```json
[
  {
    "id": 4000000000000000001,
    "guild_id": 1000000000000000001,
    "name": "welcome",
    "channel_type": "guild_text",
    "topic": "Start here!",
    "position": 0,
    "parent_id": 3000000000000000001
  },
  {
    "id": 4000000000000000002,
    "guild_id": 1000000000000000001,
    "name": "general",
    "channel_type": "guild_text",
    "topic": "General coffee chat",
    "position": 1,
    "parent_id": 3000000000000000001
  }
]
```

**Act 5: Create Roles**
```json
[
  {
    "id": 5000000000000000001,
    "guild_id": 1000000000000000001,
    "name": "Barista",
    "color": 10181046,
    "position": 2,
    "permissions": 8
  },
  {
    "id": 5000000000000000002,
    "guild_id": 1000000000000000001,
    "name": "Regular",
    "color": 3066993,
    "position": 1,
    "permissions": 104324673
  }
]
```

**Act 6: Add Guild Member (Owner)**
```json
{
  "guild_id": 1000000000000000001,
  "user_id": 2000000000000000001,
  "joined_at": "2024-01-01T00:00:00Z"
}
```

**Act 7: Assign Owner Role**
```json
{
  "guild_id": 1000000000000000001,
  "user_id": 2000000000000000001,
  "role_id": 5000000000000000001,
  "assigned_at": "2024-01-01T00:00:00Z"
}
```

This creates a basic Discord server with categories, channels, roles, and an owner member!
