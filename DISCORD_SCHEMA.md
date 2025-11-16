# Discord Database Schema Guide

This guide explains how to store Discord data in PostgreSQL for bot applications. It's designed to help you understand Discord's data model, implement the schema correctly, and query it effectively.

## Who This Guide Is For

- **Bot developers** building Discord bots with persistent storage
- **Data analysts** wanting to analyze Discord community data
- **Integration developers** connecting Discord to other systems
- **Researchers** studying online communities

## What You'll Learn

1. How Discord's data model works (guilds, channels, users, roles)
2. What to store in your database and why
3. How to implement the schema in PostgreSQL
4. Common queries and patterns
5. Performance optimization strategies

## Prerequisites

- Basic understanding of Discord (servers, channels, roles)
- PostgreSQL knowledge (tables, indexes, foreign keys)
- Familiarity with Discord bots (we use Serenity framework examples)

## Quick Start

If you just want to get started quickly:

```bash
# Clone the repository
git clone https://github.com/crumplecup/boticelli.git
cd boticelli

# Set your database URL
export DATABASE_URL="postgres://user:password@localhost/discord_bot"

# Run migrations
diesel migration run

# Verify tables exist
psql $DATABASE_URL -c "\dt discord_*"
```

Now jump to [Common Queries](#common-queries) to see how to use it.

---

## Part 1: Understanding Discord's Data Model

### How Discord is Organized

Discord's structure is fundamentally different from other platforms:

**Guild-Centric Architecture**

Everything in Discord happens inside **guilds** (what users call "servers"). A guild is like a private community with its own members, channels, and rules.

```
Your Bot
├── Guild 1: "Gaming Community"
│   ├── Members: 5,000 people
│   ├── Channels: #general, #voice-chat, etc.
│   └── Roles: @Admin, @Moderator, @Member
├── Guild 2: "Study Group"
│   ├── Members: 50 people
│   └── Channels: #homework, #projects
└── Guild 3: "Art Collective"
    └── ...
```

**Key Concepts**

- **Guilds (Servers)**: Self-contained communities
- **Channels**: Places for conversation (text, voice, forums, threads)
- **Members**: Users + their guild-specific data (nickname, roles, join date)
- **Roles**: Permission groups with colors and hierarchy
- **Messages**: Content posted in channels with attachments, embeds, reactions

### What This Schema Captures

This database schema is designed for:

1. **Event Logging** - Track everything your bot sees and does
2. **State Management** - Remember guild configurations, member data, preferences
3. **Analytics** - Monitor usage, popular channels, command statistics
4. **Audit Trails** - Maintain history for moderation and compliance
5. **Narrative Integration** - Support LLM-generated content (via Boticelli)

### What This Schema Does NOT Capture

- **Voice channel audio** - Too large, use separate voice recording system
- **Real-time presence** - Changes too frequently (online/offline/idle status)
- **Typing indicators** - Ephemeral, not worth storing
- **Temporary data** - Gateway events, heartbeats, etc.

---

## Part 2: Core Tables Explained

### Table 1: Guilds (Discord Servers)

**What is a guild?**

A guild is what Discord users call a "server" - it's a community space with channels, members, and roles. When your bot joins a server, it receives a guild object.

**What to store:**

- **Identity**: ID, name, icon, banner
- **Owner**: Who created/owns the server
- **Configuration**: Verification level, content filters, boost status
- **Metrics**: Member count, feature flags
- **Bot metadata**: When the bot joined, if it's still active

**Why store guilds?**

- Track which servers your bot is in
- Remember guild-specific settings
- Analyze server growth over time
- Handle bot removal cleanly (left_at timestamp)

**The Table:**

```sql
CREATE TABLE discord_guilds (
    -- Identity
    id BIGINT PRIMARY KEY,              -- Discord's unique snowflake ID
    name VARCHAR(100) NOT NULL,         -- Server name (e.g., "Gaming Community")
    icon VARCHAR(255),                  -- Icon hash for CDN URLs
    banner VARCHAR(255),                -- Banner hash (nitro feature)

    -- Ownership
    owner_id BIGINT NOT NULL,           -- User ID of server owner

    -- Configuration
    verification_level SMALLINT,        -- 0-4 (none to very high)
    explicit_content_filter SMALLINT,   -- 0-2 (disabled, members, all)
    premium_tier SMALLINT,              -- 0-3 (boost level)

    -- Metrics
    member_count INTEGER,               -- Total members
    description TEXT,                   -- Server description
    features TEXT[],                    -- ["COMMUNITY", "DISCOVERABLE", ...]

    -- Bot tracking (custom fields)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    joined_at TIMESTAMP,                -- When bot joined this guild
    left_at TIMESTAMP,                  -- When bot left (NULL if still active)
    bot_active BOOLEAN DEFAULT TRUE     -- Is bot currently in this guild?
);

-- Indexes for common queries
CREATE INDEX idx_guilds_active ON discord_guilds(bot_active)
    WHERE bot_active = TRUE;
```

**Common Patterns:**

```sql
-- Get all active guilds
SELECT id, name, member_count
FROM discord_guilds
WHERE bot_active = TRUE
ORDER BY member_count DESC;

-- Mark guild as left when bot is kicked
UPDATE discord_guilds
SET bot_active = FALSE, left_at = CURRENT_TIMESTAMP
WHERE id = $1;

-- Find high-value guilds (boosted communities)
SELECT name, premium_tier, member_count
FROM discord_guilds
WHERE premium_tier >= 2 AND bot_active = TRUE;
```

**Real Example:**

```sql
INSERT INTO discord_guilds (id, name, owner_id, member_count, features, joined_at)
VALUES (
    123456789012345678,
    'Cozy Café',
    987654321098765432,
    150,
    ARRAY['COMMUNITY', 'NEWS'],
    '2024-01-15 10:30:00'
);
```

---

### Table 2: Channels

**What is a channel?**

Channels are spaces for conversation. Discord has many types:
- **Text channels**: #general, #announcements
- **Voice channels**: 🔊 Voice Chat
- **Categories**: Organizational folders
- **Threads**: Sub-conversations in text channels
- **Forums**: Reddit-like discussion boards
- **Stage channels**: Presentation/podcast mode

**What to store:**

- **Identity**: ID, name, type
- **Location**: Which guild, which category
- **Settings**: Topic, slowmode, NSFW flag
- **Thread data**: Archive status, message count
- **Bot tracking**: Last message processed, access status

**Why store channels?**

- Know where your bot can post
- Track channel activity
- Remember channel-specific bot settings
- Monitor thread archival

**The Table:**

```sql
-- Channel types enum
CREATE TYPE discord_channel_type AS ENUM (
    'guild_text',            -- Normal text channel
    'dm',                    -- Direct message
    'guild_voice',           -- Voice channel
    'guild_category',        -- Category (folder)
    'guild_announcement',    -- Announcement channel
    'public_thread',         -- Public thread
    'private_thread',        -- Private thread
    'guild_forum',           -- Forum channel
    'guild_stage_voice'      -- Stage channel
);

CREATE TABLE discord_channels (
    -- Identity
    id BIGINT PRIMARY KEY,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    name VARCHAR(100),                  -- Channel name (without #)
    channel_type discord_channel_type NOT NULL,

    -- Organization
    position INTEGER,                   -- Sort order in channel list
    parent_id BIGINT REFERENCES discord_channels(id),  -- Category or parent channel

    -- Settings
    topic TEXT,                         -- Channel description/topic
    nsfw BOOLEAN DEFAULT FALSE,         -- Age-restricted content
    rate_limit_per_user INTEGER DEFAULT 0,  -- Slowmode (seconds)

    -- Thread-specific fields
    archived BOOLEAN DEFAULT FALSE,     -- Is thread archived?
    auto_archive_duration INTEGER,      -- Minutes until auto-archive
    message_count INTEGER,              -- Messages in thread

    -- Bot tracking
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_message_at TIMESTAMP,          -- Most recent message timestamp
    bot_has_access BOOLEAN DEFAULT TRUE -- Can bot read/write here?
);

CREATE INDEX idx_channels_guild ON discord_channels(guild_id);
CREATE INDEX idx_channels_type ON discord_channels(channel_type);
```

**Common Patterns:**

```sql
-- Get all text channels in a guild
SELECT id, name, topic
FROM discord_channels
WHERE guild_id = $1
  AND channel_type = 'guild_text'
  AND bot_has_access = TRUE
ORDER BY position;

-- Find active threads
SELECT id, name, message_count
FROM discord_channels
WHERE parent_id = $1
  AND archived = FALSE
  AND channel_type IN ('public_thread', 'private_thread');

-- Get voice channels
SELECT id, name
FROM discord_channels
WHERE guild_id = $1
  AND channel_type IN ('guild_voice', 'guild_stage_voice')
ORDER BY position;
```

**Real Example:**

```sql
-- Create a text channel
INSERT INTO discord_channels (id, guild_id, name, channel_type, topic, position)
VALUES (
    111111111111111111,
    123456789012345678,
    'general',
    'guild_text',
    'Welcome to the café! Grab a coffee and chat.',
    0
);

-- Create a category
INSERT INTO discord_channels (id, guild_id, name, channel_type, position)
VALUES (
    222222222222222222,
    123456789012345678,
    'Community',
    'guild_category',
    0
);

-- Create a channel under the category
INSERT INTO discord_channels (id, guild_id, name, channel_type, parent_id, position)
VALUES (
    333333333333333333,
    123456789012345678,
    'introductions',
    'guild_text',
    222222222222222222,  -- parent_id = category
    1
);
```

---

### Table 3: Users

**What is a user?**

Users are Discord accounts. They exist globally across all guilds. A user object contains profile information.

**Important distinction:**
- **User** = Global Discord account (username, avatar)
- **Member** = User in a specific guild (nickname, roles, join date)

**What to store:**

- **Identity**: ID, username, discriminator
- **Profile**: Display name, avatar, banner
- **Flags**: Bot account, Nitro status
- **Bot tracking**: First/last seen timestamps

**Why store users?**

- Identify who interacted with your bot
- Track users across multiple guilds
- Distinguish bots from humans
- Remember when you first saw a user

**The Table:**

```sql
CREATE TABLE discord_users (
    -- Identity
    id BIGINT PRIMARY KEY,
    username VARCHAR(32) NOT NULL,      -- Username without @
    discriminator VARCHAR(4),           -- Legacy #1234 tag (being phased out)
    global_name VARCHAR(100),           -- Display name (new system)

    -- Profile
    avatar VARCHAR(255),                -- Avatar hash
    banner VARCHAR(255),                -- Profile banner hash

    -- Account flags
    bot BOOLEAN DEFAULT FALSE,          -- Is this a bot account?
    premium_type SMALLINT,              -- 0=none, 1=Classic, 2=Nitro

    -- Bot tracking
    first_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_bot ON discord_users(bot);
```

**Common Patterns:**

```sql
-- Find all bot accounts
SELECT id, username FROM discord_users WHERE bot = TRUE;

-- Get user by ID
SELECT username, global_name, avatar FROM discord_users WHERE id = $1;

-- Update last seen timestamp
UPDATE discord_users SET last_seen = CURRENT_TIMESTAMP WHERE id = $1;

-- Find Nitro users
SELECT id, username FROM discord_users WHERE premium_type >= 1;
```

**Real Example:**

```sql
INSERT INTO discord_users (id, username, global_name, avatar, bot)
VALUES (
    987654321098765432,
    'alice_wonderland',
    'Alice',
    'a1b2c3d4e5f6',
    FALSE
);
```

---

### Table 4: Guild Members

**What is a guild member?**

A guild member represents a user's presence in a specific guild. It's the junction between users and guilds, plus guild-specific data.

**What to store:**

- **Association**: Which user in which guild
- **Guild-specific data**: Nickname, guild avatar, joined date
- **Server boost**: Premium since date
- **Moderation**: Timeout status
- **Bot tracking**: When member left

**Why store members?**

- Track guild membership
- Remember nicknames and guild-specific settings
- Identify boosters
- Handle member removal

**The Table:**

```sql
CREATE TABLE discord_guild_members (
    -- Association (composite primary key)
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES discord_users(id) ON DELETE CASCADE,
    PRIMARY KEY (guild_id, user_id),

    -- Guild-specific data
    nick VARCHAR(32),                   -- Guild nickname (overrides username)
    avatar VARCHAR(255),                -- Guild-specific avatar

    -- Membership
    joined_at TIMESTAMP NOT NULL,       -- When user joined this guild
    premium_since TIMESTAMP,            -- When user started boosting

    -- Moderation
    communication_disabled_until TIMESTAMP,  -- Timeout end time

    -- Bot tracking
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP                   -- When user left (NULL if still member)
);

CREATE INDEX idx_members_guild ON discord_guild_members(guild_id);
CREATE INDEX idx_members_user ON discord_guild_members(user_id);
CREATE INDEX idx_members_boosters ON discord_guild_members(guild_id, premium_since)
    WHERE premium_since IS NOT NULL;
```

**Common Patterns:**

```sql
-- Get all members in a guild
SELECT u.username, m.nick, m.joined_at
FROM discord_guild_members m
JOIN discord_users u ON m.user_id = u.id
WHERE m.guild_id = $1 AND m.left_at IS NULL
ORDER BY m.joined_at;

-- Find server boosters
SELECT u.username, m.premium_since
FROM discord_guild_members m
JOIN discord_users u ON m.user_id = u.id
WHERE m.guild_id = $1 AND m.premium_since IS NOT NULL;

-- Get member count
SELECT COUNT(*) FROM discord_guild_members
WHERE guild_id = $1 AND left_at IS NULL;

-- Mark member as left
UPDATE discord_guild_members
SET left_at = CURRENT_TIMESTAMP
WHERE guild_id = $1 AND user_id = $2;
```

**Real Example:**

```sql
INSERT INTO discord_guild_members (guild_id, user_id, joined_at, nick)
VALUES (
    123456789012345678,
    987654321098765432,
    '2024-01-15 14:30:00',
    'Café Alice'
);
```

---

### Table 5: Roles

**What is a role?**

Roles are permission groups with names, colors, and hierarchy. They control what members can do and how they're displayed.

**What to store:**

- **Identity**: ID, name, color
- **Position**: Hierarchy (higher = more powerful)
- **Permissions**: Bitfield of allowed actions
- **Display**: Hoist (separate section), icon

**Why store roles?**

- Manage permission hierarchies
- Track role assignments
- Remember role colors and settings
- Identify managed roles (bot/boost roles)

**The Table:**

```sql
CREATE TABLE discord_roles (
    -- Identity
    id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,

    -- Display
    color INTEGER NOT NULL DEFAULT 0,   -- RGB color as decimal
    hoist BOOLEAN DEFAULT FALSE,        -- Show separately in member list
    icon VARCHAR(255),                  -- Role icon hash

    -- Hierarchy
    position INTEGER NOT NULL,          -- Higher = more powerful

    -- Permissions
    permissions BIGINT NOT NULL,        -- Bitfield of permissions

    -- Flags
    managed BOOLEAN DEFAULT FALSE,      -- Managed by integration (bot/boost)
    mentionable BOOLEAN DEFAULT FALSE,  -- Can @role mention work?

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_roles_guild ON discord_roles(guild_id, position DESC);
```

**Common Patterns:**

```sql
-- Get roles in hierarchy order
SELECT name, position, color
FROM discord_roles
WHERE guild_id = $1
ORDER BY position DESC;

-- Find mentionable roles
SELECT id, name
FROM discord_roles
WHERE guild_id = $1 AND mentionable = TRUE;

-- Get user-manageable roles (not bot-managed)
SELECT id, name, color
FROM discord_roles
WHERE guild_id = $1 AND managed = FALSE
ORDER BY position DESC;
```

**Real Example:**

```sql
INSERT INTO discord_roles (id, guild_id, name, color, position, permissions, hoist)
VALUES (
    444444444444444444,
    123456789012345678,
    'Café Staff',
    3447003,  -- Blue color
    5,
    8,        -- Administrator permission
    TRUE      -- Show separately
);
```

---

### Table 6: Member Roles (Junction)

**What is this table?**

This junction table connects members to their roles. A member can have multiple roles.

**What to store:**

- **Association**: guild_id + user_id + role_id
- **Metadata**: When assigned, who assigned it

**The Table:**

```sql
CREATE TABLE discord_member_roles (
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL REFERENCES discord_roles(id) ON DELETE CASCADE,

    -- Metadata
    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by BIGINT,  -- User ID who assigned (if known)

    PRIMARY KEY (guild_id, user_id, role_id),
    FOREIGN KEY (guild_id, user_id)
        REFERENCES discord_guild_members(guild_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_member_roles_user ON discord_member_roles(guild_id, user_id);
CREATE INDEX idx_member_roles_role ON discord_member_roles(role_id);
```

**Common Patterns:**

```sql
-- Get all roles for a member
SELECT r.name, r.color, mr.assigned_at
FROM discord_member_roles mr
JOIN discord_roles r ON mr.role_id = r.id
WHERE mr.guild_id = $1 AND mr.user_id = $2
ORDER BY r.position DESC;

-- Get all members with a specific role
SELECT u.username, m.nick
FROM discord_member_roles mr
JOIN discord_guild_members m ON mr.guild_id = m.guild_id AND mr.user_id = m.user_id
JOIN discord_users u ON m.user_id = u.id
WHERE mr.role_id = $1;

-- Assign role to member
INSERT INTO discord_member_roles (guild_id, user_id, role_id, assigned_by)
VALUES ($1, $2, $3, $4)
ON CONFLICT (guild_id, user_id, role_id) DO NOTHING;
```

---

## Part 3: Common Queries

### Query Patterns

**Get guild overview:**

```sql
SELECT
    g.name AS guild_name,
    COUNT(DISTINCT m.user_id) AS member_count,
    COUNT(DISTINCT c.id) AS channel_count,
    COUNT(DISTINCT r.id) AS role_count
FROM discord_guilds g
LEFT JOIN discord_guild_members m ON g.id = m.guild_id AND m.left_at IS NULL
LEFT JOIN discord_channels c ON g.id = c.guild_id
LEFT JOIN discord_roles r ON g.id = r.guild_id
WHERE g.id = $1
GROUP BY g.id, g.name;
```

**Find member's effective permissions:**

```sql
-- Get all roles for a member
WITH member_roles AS (
    SELECT r.*
    FROM discord_member_roles mr
    JOIN discord_roles r ON mr.role_id = r.id
    WHERE mr.guild_id = $1 AND mr.user_id = $2
)
SELECT BIT_OR(permissions) AS total_permissions
FROM member_roles;
```

**Track channel activity:**

```sql
SELECT
    c.name,
    c.last_message_at,
    COUNT(m.id) AS message_count
FROM discord_channels c
LEFT JOIN discord_messages m ON c.id = m.channel_id
WHERE c.guild_id = $1
GROUP BY c.id
ORDER BY message_count DESC;
```

---

## Part 4: Best Practices

### Performance

**Use appropriate indexes:**
- Index foreign keys (guild_id, user_id, channel_id)
- Index frequently filtered columns (bot_active, archived, channel_type)
- Use partial indexes for common WHERE clauses

**Partition large tables:**

```sql
-- Partition messages by month
CREATE TABLE discord_messages_y2024m01 PARTITION OF discord_messages
FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### Data Integrity

**CASCADE deletions appropriately:**
- Channels should CASCADE when guild is deleted
- Members should CASCADE when guild or user is deleted
- Messages should CASCADE when channel is deleted

**Soft deletes for important data:**
```sql
-- Don't delete guilds, mark as left
UPDATE discord_guilds SET bot_active = FALSE, left_at = NOW() WHERE id = $1;
```

### Maintenance

**Regular cleanup:**

```sql
-- Archive old messages (move to separate table)
INSERT INTO discord_messages_archive
SELECT * FROM discord_messages WHERE created_at < NOW() - INTERVAL '90 days';

DELETE FROM discord_messages WHERE created_at < NOW() - INTERVAL '90 days';
```

**Update timestamps:**

```sql
-- Use triggers for updated_at
CREATE TRIGGER update_guild_timestamp
    BEFORE UPDATE ON discord_guilds
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

---

## Part 5: Integration with Boticelli

This schema is designed to work seamlessly with the Boticelli narrative system. See [NARRATIVE_PROCESSORS.md](NARRATIVE_PROCESSORS.md) for details on generating Discord content via LLM narratives.

**Example workflow:**

1. Create narrative describing desired Discord content
2. Execute narrative with Gemini/Claude
3. Processors automatically extract JSON and insert into these tables
4. Query stored data for verification
5. Use BoticelliBot to post to real Discord servers

---

## Next Steps

- **Implement the schema**: Run the migrations in `migrations/`
- **Explore narratives**: See [DISCORD_NARRATIVE.md](DISCORD_NARRATIVE.md) for examples
- **Build processors**: Check [NARRATIVE_PROCESSORS.md](NARRATIVE_PROCESSORS.md)
- **Create your bot**: Use the Serenity framework with this database

---

## Full Schema Reference

For the complete SQL schema with all tables (messages, reactions, embeds, etc.), see the appendix or run:

```bash
diesel migration run
psql $DATABASE_URL -c "\d+ discord_guilds"
```

This will show you the complete table definitions with all fields, constraints, and indexes.
