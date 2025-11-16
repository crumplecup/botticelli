# Discord Database Schema for Serenity

This document defines a comprehensive PostgreSQL database schema for Discord bots built with Serenity. The schema captures Discord's complex data model including guilds, channels, messages, users, roles, and interactions.

## Design Philosophy

Discord's data model is fundamentally different from other social media platforms:

- **Guild-centric**: Everything revolves around guilds (servers)
- **Real-time events**: Heavy emphasis on gateway events and live interactions
- **Rich permissions**: Complex role-based permission systems
- **Multiple content types**: Text, voice, threads, forums, stages
- **Interactive elements**: Slash commands, buttons, select menus, modals
- **Ephemeral data**: Voice states, presences, typing indicators

This schema is designed for:

1. **Event logging**: Track all bot interactions and events
2. **State management**: Cache guild configurations and member data
3. **Analytics**: Monitor bot usage, command statistics, and engagement
4. **Audit trails**: Maintain history for moderation and compliance
5. **Integration with Boticelli**: Support narrative-driven content posting to Discord

## Core Entity Relationships

```
Guild (Server)
├── Channels (Text, Voice, Threads, Forums)
│   ├── Messages
│   │   ├── Attachments
│   │   ├── Embeds
│   │   ├── Reactions
│   │   └── Components (Buttons, Menus)
│   └── Permissions (Role/Member overrides)
├── Members (User + Guild context)
│   ├── Roles
│   └── Voice States
├── Roles
│   └── Permissions
├── Emojis
├── Stickers
├── Webhooks
└── Scheduled Events

Users (Global)
├── Bot interactions across guilds
└── Direct Messages

Application Commands
├── Slash Commands
├── User Commands
└── Message Commands

Interactions
├── Command Executions
├── Component Interactions
└── Modal Submissions
```

## Database Schema

### 1. Guilds (Servers)

```sql
CREATE TABLE discord_guilds (
    id BIGINT PRIMARY KEY,  -- Discord snowflake ID
    name VARCHAR(100) NOT NULL,
    icon VARCHAR(255),  -- Image hash
    banner VARCHAR(255),
    splash VARCHAR(255),
    owner_id BIGINT NOT NULL,

    -- Guild features
    features TEXT[],  -- Array of feature flags
    description TEXT,
    vanity_url_code VARCHAR(50),

    -- Member counts
    member_count INTEGER,
    approximate_member_count INTEGER,
    approximate_presence_count INTEGER,

    -- Guild settings
    afk_channel_id BIGINT,
    afk_timeout INTEGER,
    system_channel_id BIGINT,
    rules_channel_id BIGINT,
    public_updates_channel_id BIGINT,

    -- Verification and content filtering
    verification_level SMALLINT,
    explicit_content_filter SMALLINT,
    mfa_level SMALLINT,

    -- Premium features
    premium_tier SMALLINT,
    premium_subscription_count INTEGER,

    -- Server boost progress
    max_presences INTEGER,
    max_members INTEGER,
    max_video_channel_users INTEGER,

    -- Status flags
    large BOOLEAN DEFAULT FALSE,
    unavailable BOOLEAN DEFAULT FALSE,

    -- Timestamps
    joined_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP,  -- Track when bot left the guild

    -- Bot-specific metadata
    bot_permissions BIGINT,  -- Permissions the bot has in this guild
    bot_active BOOLEAN DEFAULT TRUE
);

CREATE INDEX idx_guilds_owner ON discord_guilds(owner_id);
CREATE INDEX idx_guilds_active ON discord_guilds(bot_active) WHERE bot_active = TRUE;
CREATE INDEX idx_guilds_left_at ON discord_guilds(left_at);
```

### 2. Channels

```sql
CREATE TYPE discord_channel_type AS ENUM (
    'guild_text',
    'dm',
    'guild_voice',
    'group_dm',
    'guild_category',
    'guild_announcement',
    'announcement_thread',
    'public_thread',
    'private_thread',
    'guild_stage_voice',
    'guild_directory',
    'guild_forum',
    'guild_media'
);

CREATE TABLE discord_channels (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    name VARCHAR(100),
    channel_type discord_channel_type NOT NULL,
    position INTEGER,

    -- Topic and description
    topic TEXT,

    -- Channel settings
    nsfw BOOLEAN DEFAULT FALSE,
    rate_limit_per_user INTEGER DEFAULT 0,  -- Slowmode in seconds
    bitrate INTEGER,  -- For voice channels
    user_limit INTEGER,  -- For voice channels

    -- Thread-specific
    parent_id BIGINT REFERENCES discord_channels(id) ON DELETE CASCADE,
    owner_id BIGINT,  -- Thread creator
    message_count INTEGER,  -- Thread message count
    member_count INTEGER,  -- Thread member count
    archived BOOLEAN DEFAULT FALSE,
    auto_archive_duration INTEGER,
    archive_timestamp TIMESTAMP,
    locked BOOLEAN DEFAULT FALSE,
    invitable BOOLEAN DEFAULT TRUE,

    -- Forum-specific
    available_tags JSONB,  -- Forum tags
    default_reaction_emoji JSONB,
    default_thread_rate_limit INTEGER,
    default_sort_order SMALLINT,
    default_forum_layout SMALLINT,

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_message_at TIMESTAMP,

    -- Bot tracking
    last_read_message_id BIGINT,  -- Last message bot processed
    bot_has_access BOOLEAN DEFAULT TRUE
);

CREATE INDEX idx_channels_guild ON discord_channels(guild_id);
CREATE INDEX idx_channels_parent ON discord_channels(parent_id);
CREATE INDEX idx_channels_type ON discord_channels(channel_type);
CREATE INDEX idx_channels_active_threads ON discord_channels(archived, channel_type)
    WHERE archived = FALSE AND channel_type IN ('public_thread', 'private_thread', 'announcement_thread');
```

### 3. Users

```sql
CREATE TABLE discord_users (
    id BIGINT PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    discriminator VARCHAR(4),  -- Legacy discriminator, nullable for new usernames
    global_name VARCHAR(32),  -- Display name
    avatar VARCHAR(255),
    banner VARCHAR(255),
    accent_color INTEGER,

    -- Account flags
    bot BOOLEAN DEFAULT FALSE,
    system BOOLEAN DEFAULT FALSE,
    mfa_enabled BOOLEAN,
    verified BOOLEAN,

    -- Premium status
    premium_type SMALLINT,
    public_flags INTEGER,

    -- Locale
    locale VARCHAR(10),

    -- Timestamps
    first_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_username ON discord_users(username);
CREATE INDEX idx_users_bot ON discord_users(bot);
CREATE INDEX idx_users_last_seen ON discord_users(last_seen);
```

### 4. Guild Members

```sql
CREATE TABLE discord_guild_members (
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES discord_users(id) ON DELETE CASCADE,

    -- Member-specific data
    nick VARCHAR(32),
    avatar VARCHAR(255),  -- Guild-specific avatar

    -- Timestamps
    joined_at TIMESTAMP NOT NULL,
    premium_since TIMESTAMP,  -- Server boost date
    communication_disabled_until TIMESTAMP,  -- Timeout

    -- Flags
    deaf BOOLEAN DEFAULT FALSE,
    mute BOOLEAN DEFAULT FALSE,
    pending BOOLEAN DEFAULT FALSE,  -- Passed membership screening

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP,

    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX idx_guild_members_user ON discord_guild_members(user_id);
CREATE INDEX idx_guild_members_joined ON discord_guild_members(joined_at);
CREATE INDEX idx_guild_members_active ON discord_guild_members(left_at) WHERE left_at IS NULL;
CREATE INDEX idx_guild_members_boosters ON discord_guild_members(premium_since) WHERE premium_since IS NOT NULL;
```

### 5. Roles

```sql
CREATE TABLE discord_roles (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    color INTEGER NOT NULL DEFAULT 0,
    hoist BOOLEAN DEFAULT FALSE,  -- Display separately
    icon VARCHAR(255),
    unicode_emoji VARCHAR(100),
    position INTEGER NOT NULL,
    permissions BIGINT NOT NULL,
    managed BOOLEAN DEFAULT FALSE,  -- Managed by integration
    mentionable BOOLEAN DEFAULT FALSE,

    -- Role tags (bot, integration, premium subscriber)
    tags JSONB,

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_roles_guild ON discord_roles(guild_id);
CREATE INDEX idx_roles_position ON discord_roles(guild_id, position);
```

### 6. Member Roles (Junction Table)

```sql
CREATE TABLE discord_member_roles (
    guild_id BIGINT,
    user_id BIGINT,
    role_id BIGINT REFERENCES discord_roles(id) ON DELETE CASCADE,

    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by BIGINT,  -- User who assigned the role

    PRIMARY KEY (guild_id, user_id, role_id),
    FOREIGN KEY (guild_id, user_id) REFERENCES discord_guild_members(guild_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_member_roles_user ON discord_member_roles(guild_id, user_id);
CREATE INDEX idx_member_roles_role ON discord_member_roles(role_id);
```

### 7. Channel Permission Overrides

```sql
CREATE TYPE discord_permission_type AS ENUM ('role', 'member');

CREATE TABLE discord_channel_permissions (
    id SERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES discord_channels(id) ON DELETE CASCADE,
    permission_type discord_permission_type NOT NULL,
    target_id BIGINT NOT NULL,  -- Role ID or User ID

    allow BIGINT NOT NULL DEFAULT 0,
    deny BIGINT NOT NULL DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (channel_id, permission_type, target_id)
);

CREATE INDEX idx_channel_perms_channel ON discord_channel_permissions(channel_id);
CREATE INDEX idx_channel_perms_target ON discord_channel_permissions(target_id);
```

### 8. Messages

```sql
CREATE TYPE discord_message_type AS ENUM (
    'regular',
    'recipient_add',
    'recipient_remove',
    'call',
    'channel_name_change',
    'channel_icon_change',
    'channel_pinned_message',
    'guild_member_join',
    'user_premium_guild_subscription',
    'user_premium_guild_subscription_tier_1',
    'user_premium_guild_subscription_tier_2',
    'user_premium_guild_subscription_tier_3',
    'channel_follow_add',
    'guild_discovery_disqualified',
    'guild_discovery_requalified',
    'guild_discovery_grace_period_initial_warning',
    'guild_discovery_grace_period_final_warning',
    'thread_created',
    'reply',
    'chat_input_command',
    'thread_starter_message',
    'guild_invite_reminder',
    'context_menu_command',
    'auto_moderation_action',
    'role_subscription_purchase',
    'interaction_premium_upsell',
    'stage_start',
    'stage_end',
    'stage_speaker',
    'stage_topic',
    'guild_application_premium_subscription'
);

CREATE TABLE discord_messages (
    id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES discord_channels(id) ON DELETE CASCADE,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    author_id BIGINT NOT NULL REFERENCES discord_users(id),

    -- Content
    content TEXT,
    message_type discord_message_type NOT NULL DEFAULT 'regular',

    -- Flags
    tts BOOLEAN DEFAULT FALSE,
    mention_everyone BOOLEAN DEFAULT FALSE,
    pinned BOOLEAN DEFAULT FALSE,

    -- Webhook
    webhook_id BIGINT,

    -- Application (for bot/application messages)
    application_id BIGINT,

    -- Interaction
    interaction_id BIGINT,
    interaction_type SMALLINT,

    -- Reply/Reference
    referenced_message_id BIGINT,

    -- Nonce for deduplication
    nonce VARCHAR(100),

    -- Timestamps
    timestamp TIMESTAMP NOT NULL,
    edited_timestamp TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Bot processing
    processed BOOLEAN DEFAULT FALSE,
    processed_at TIMESTAMP
);

CREATE INDEX idx_messages_channel ON discord_messages(channel_id, timestamp DESC);
CREATE INDEX idx_messages_guild ON discord_messages(guild_id, timestamp DESC);
CREATE INDEX idx_messages_author ON discord_messages(author_id);
CREATE INDEX idx_messages_timestamp ON discord_messages(timestamp DESC);
CREATE INDEX idx_messages_referenced ON discord_messages(referenced_message_id);
CREATE INDEX idx_messages_interaction ON discord_messages(interaction_id);
CREATE INDEX idx_messages_unprocessed ON discord_messages(processed, created_at) WHERE processed = FALSE;

-- Partition by month for better performance
-- CREATE TABLE discord_messages_2024_01 PARTITION OF discord_messages
-- FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### 9. Message Mentions

```sql
CREATE TYPE discord_mention_type AS ENUM ('user', 'role', 'channel');

CREATE TABLE discord_message_mentions (
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,
    mention_type discord_mention_type NOT NULL,
    target_id BIGINT NOT NULL,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (message_id, mention_type, target_id)
);

CREATE INDEX idx_mentions_target ON discord_message_mentions(mention_type, target_id);
```

### 10. Message Attachments

```sql
CREATE TABLE discord_message_attachments (
    id BIGINT PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,

    filename VARCHAR(255) NOT NULL,
    description TEXT,
    content_type VARCHAR(100),
    size INTEGER NOT NULL,
    url VARCHAR(500) NOT NULL,
    proxy_url VARCHAR(500) NOT NULL,

    -- Image/video metadata
    height INTEGER,
    width INTEGER,
    duration_secs NUMERIC(10, 3),

    -- Flags
    ephemeral BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_attachments_message ON discord_message_attachments(message_id);
CREATE INDEX idx_attachments_content_type ON discord_message_attachments(content_type);
```

### 11. Message Embeds

```sql
CREATE TABLE discord_message_embeds (
    id SERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,

    -- Embed content
    title VARCHAR(256),
    embed_type VARCHAR(50),  -- 'rich', 'image', 'video', 'link', etc.
    description TEXT,
    url VARCHAR(500),
    timestamp TIMESTAMP,
    color INTEGER,

    -- Footer
    footer_text VARCHAR(2048),
    footer_icon_url VARCHAR(500),

    -- Image
    image_url VARCHAR(500),
    image_proxy_url VARCHAR(500),
    image_height INTEGER,
    image_width INTEGER,

    -- Thumbnail
    thumbnail_url VARCHAR(500),
    thumbnail_proxy_url VARCHAR(500),
    thumbnail_height INTEGER,
    thumbnail_width INTEGER,

    -- Video
    video_url VARCHAR(500),
    video_proxy_url VARCHAR(500),
    video_height INTEGER,
    video_width INTEGER,

    -- Provider
    provider_name VARCHAR(256),
    provider_url VARCHAR(500),

    -- Author
    author_name VARCHAR(256),
    author_url VARCHAR(500),
    author_icon_url VARCHAR(500),

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_embeds_message ON discord_message_embeds(message_id);
```

### 12. Embed Fields

```sql
CREATE TABLE discord_embed_fields (
    id SERIAL PRIMARY KEY,
    embed_id INTEGER NOT NULL REFERENCES discord_message_embeds(id) ON DELETE CASCADE,

    name VARCHAR(256) NOT NULL,
    value VARCHAR(1024) NOT NULL,
    inline BOOLEAN DEFAULT FALSE,
    position INTEGER NOT NULL,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_embed_fields_embed ON discord_embed_fields(embed_id, position);
```

### 13. Message Reactions

```sql
CREATE TABLE discord_message_reactions (
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES discord_users(id),

    -- Emoji can be custom or unicode
    emoji_id BIGINT,  -- NULL for unicode emoji
    emoji_name VARCHAR(100) NOT NULL,
    emoji_animated BOOLEAN DEFAULT FALSE,

    reacted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (message_id, user_id, emoji_name, emoji_id)
);

CREATE INDEX idx_reactions_message ON discord_message_reactions(message_id);
CREATE INDEX idx_reactions_user ON discord_message_reactions(user_id);
CREATE INDEX idx_reactions_emoji ON discord_message_reactions(emoji_id) WHERE emoji_id IS NOT NULL;
```

### 14. Message Components

```sql
CREATE TYPE discord_component_type AS ENUM (
    'action_row',
    'button',
    'select_menu',
    'text_input',
    'user_select',
    'role_select',
    'mentionable_select',
    'channel_select'
);

CREATE TYPE discord_button_style AS ENUM (
    'primary',
    'secondary',
    'success',
    'danger',
    'link'
);

CREATE TABLE discord_message_components (
    id SERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,

    component_type discord_component_type NOT NULL,
    custom_id VARCHAR(100),

    -- Button-specific
    button_style discord_button_style,
    label VARCHAR(80),
    emoji_id BIGINT,
    emoji_name VARCHAR(100),
    url VARCHAR(500),  -- For link buttons
    disabled BOOLEAN DEFAULT FALSE,

    -- Select menu-specific
    placeholder VARCHAR(150),
    min_values INTEGER,
    max_values INTEGER,

    -- Options stored as JSONB for flexibility
    options JSONB,

    -- Position in action row
    row_position INTEGER,
    component_position INTEGER,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_components_message ON discord_message_components(message_id);
CREATE INDEX idx_components_custom_id ON discord_message_components(custom_id);
```

### 15. Emojis

```sql
CREATE TABLE discord_emojis (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,

    name VARCHAR(100) NOT NULL,
    animated BOOLEAN DEFAULT FALSE,
    managed BOOLEAN DEFAULT FALSE,
    require_colons BOOLEAN DEFAULT TRUE,
    available BOOLEAN DEFAULT TRUE,

    -- Creator
    creator_id BIGINT REFERENCES discord_users(id),

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_emojis_guild ON discord_emojis(guild_id);
CREATE INDEX idx_emojis_name ON discord_emojis(name);
```

### 16. Emoji Roles (which roles can use the emoji)

```sql
CREATE TABLE discord_emoji_roles (
    emoji_id BIGINT NOT NULL REFERENCES discord_emojis(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES discord_roles(id) ON DELETE CASCADE,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (emoji_id, role_id)
);

CREATE INDEX idx_emoji_roles_emoji ON discord_emoji_roles(emoji_id);
```

### 17. Stickers

```sql
CREATE TYPE discord_sticker_type AS ENUM ('standard', 'guild');

CREATE TYPE discord_sticker_format AS ENUM ('png', 'apng', 'lottie', 'gif');

CREATE TABLE discord_stickers (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,

    name VARCHAR(100) NOT NULL,
    description VARCHAR(200),
    tags VARCHAR(200),  -- Autocomplete tags

    sticker_type discord_sticker_type NOT NULL,
    format_type discord_sticker_format NOT NULL,

    available BOOLEAN DEFAULT TRUE,
    sort_value INTEGER,

    -- Creator (for guild stickers)
    creator_id BIGINT REFERENCES discord_users(id),

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_stickers_guild ON discord_stickers(guild_id);
CREATE INDEX idx_stickers_type ON discord_stickers(sticker_type);
```

### 18. Message Stickers (which stickers were used in a message)

```sql
CREATE TABLE discord_message_stickers (
    message_id BIGINT NOT NULL REFERENCES discord_messages(id) ON DELETE CASCADE,
    sticker_id BIGINT NOT NULL REFERENCES discord_stickers(id),

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (message_id, sticker_id)
);

CREATE INDEX idx_message_stickers_sticker ON discord_message_stickers(sticker_id);
```

### 19. Application Commands

```sql
CREATE TYPE discord_command_type AS ENUM (
    'chat_input',      -- Slash commands
    'user',            -- User context menu
    'message'          -- Message context menu
);

CREATE TABLE discord_application_commands (
    id BIGINT PRIMARY KEY,
    application_id BIGINT NOT NULL,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,  -- NULL for global commands

    command_type discord_command_type NOT NULL,
    name VARCHAR(32) NOT NULL,
    name_localizations JSONB,
    description VARCHAR(100),
    description_localizations JSONB,

    -- Command options (parameters)
    options JSONB,

    -- Permissions
    default_member_permissions BIGINT,
    dm_permission BOOLEAN DEFAULT TRUE,
    nsfw BOOLEAN DEFAULT FALSE,

    -- Versioning
    version BIGINT NOT NULL,

    -- Status
    enabled BOOLEAN DEFAULT TRUE,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_commands_guild ON discord_application_commands(guild_id);
CREATE INDEX idx_commands_name ON discord_application_commands(name);
CREATE INDEX idx_commands_type ON discord_application_commands(command_type);
```

### 20. Command Executions (Interactions)

```sql
CREATE TYPE discord_interaction_type AS ENUM (
    'ping',
    'application_command',
    'message_component',
    'application_command_autocomplete',
    'modal_submit'
);

CREATE TABLE discord_command_executions (
    id BIGINT PRIMARY KEY,  -- Interaction ID
    application_id BIGINT NOT NULL,
    command_id BIGINT REFERENCES discord_application_commands(id),

    interaction_type discord_interaction_type NOT NULL,

    -- Context
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT REFERENCES discord_channels(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES discord_users(id),

    -- Command data
    command_name VARCHAR(32),
    command_type discord_command_type,
    options JSONB,  -- Command options/parameters

    -- Component interaction data
    custom_id VARCHAR(100),
    component_type discord_component_type,
    values JSONB,  -- Selected values

    -- Modal data
    modal_custom_id VARCHAR(100),
    modal_components JSONB,

    -- Response
    response_type SMALLINT,
    response_content TEXT,
    response_embed_count INTEGER DEFAULT 0,
    response_component_count INTEGER DEFAULT 0,
    deferred BOOLEAN DEFAULT FALSE,
    ephemeral BOOLEAN DEFAULT FALSE,

    -- Timing
    executed_at TIMESTAMP NOT NULL,
    responded_at TIMESTAMP,
    response_time_ms INTEGER,

    -- Error tracking
    error BOOLEAN DEFAULT FALSE,
    error_message TEXT,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_executions_guild ON discord_command_executions(guild_id, executed_at DESC);
CREATE INDEX idx_executions_user ON discord_command_executions(user_id, executed_at DESC);
CREATE INDEX idx_executions_command ON discord_command_executions(command_id, executed_at DESC);
CREATE INDEX idx_executions_executed_at ON discord_command_executions(executed_at DESC);
CREATE INDEX idx_executions_errors ON discord_command_executions(error, executed_at DESC) WHERE error = TRUE;
```

### 21. Voice States

```sql
CREATE TABLE discord_voice_states (
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT REFERENCES discord_channels(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES discord_users(id),

    -- Session
    session_id VARCHAR(100) NOT NULL,

    -- States
    deaf BOOLEAN DEFAULT FALSE,
    mute BOOLEAN DEFAULT FALSE,
    self_deaf BOOLEAN DEFAULT FALSE,
    self_mute BOOLEAN DEFAULT FALSE,
    self_stream BOOLEAN DEFAULT FALSE,
    self_video BOOLEAN DEFAULT FALSE,
    suppress BOOLEAN DEFAULT FALSE,  -- Suppressed in stage channel

    -- Timestamps
    joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP,

    PRIMARY KEY (guild_id, user_id, session_id)
);

CREATE INDEX idx_voice_states_channel ON discord_voice_states(channel_id, left_at) WHERE left_at IS NULL;
CREATE INDEX idx_voice_states_user ON discord_voice_states(user_id);
CREATE INDEX idx_voice_states_active ON discord_voice_states(guild_id, channel_id) WHERE left_at IS NULL;
```

### 22. Webhooks

```sql
CREATE TYPE discord_webhook_type AS ENUM (
    'incoming',
    'channel_follower',
    'application'
);

CREATE TABLE discord_webhooks (
    id BIGINT PRIMARY KEY,
    webhook_type discord_webhook_type NOT NULL,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES discord_channels(id) ON DELETE CASCADE,

    name VARCHAR(80),
    avatar VARCHAR(255),
    token VARCHAR(255),  -- Webhook token (sensitive!)

    application_id BIGINT,
    creator_id BIGINT REFERENCES discord_users(id),

    -- Source (for follower webhooks)
    source_guild_id BIGINT,
    source_channel_id BIGINT,

    -- Bot tracking
    created_by_bot BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP
);

CREATE INDEX idx_webhooks_channel ON discord_webhooks(channel_id);
CREATE INDEX idx_webhooks_guild ON discord_webhooks(guild_id);
CREATE INDEX idx_webhooks_active ON discord_webhooks(deleted_at) WHERE deleted_at IS NULL;
```

### 23. Invites

```sql
CREATE TYPE discord_invite_target_type AS ENUM (
    'stream',
    'embedded_application'
);

CREATE TABLE discord_invites (
    code VARCHAR(50) PRIMARY KEY,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES discord_channels(id) ON DELETE CASCADE,
    inviter_id BIGINT REFERENCES discord_users(id),

    target_type discord_invite_target_type,
    target_user_id BIGINT REFERENCES discord_users(id),
    target_application_id BIGINT,

    -- Invite settings
    max_age INTEGER,  -- Seconds, 0 for infinite
    max_uses INTEGER,  -- 0 for infinite
    temporary BOOLEAN DEFAULT FALSE,

    -- Usage tracking
    uses INTEGER DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    revoked_at TIMESTAMP
);

CREATE INDEX idx_invites_guild ON discord_invites(guild_id);
CREATE INDEX idx_invites_inviter ON discord_invites(inviter_id);
CREATE INDEX idx_invites_expires ON discord_invites(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_invites_active ON discord_invites(revoked_at, expires_at)
    WHERE revoked_at IS NULL;
```

### 24. Scheduled Events

```sql
CREATE TYPE discord_event_status AS ENUM (
    'scheduled',
    'active',
    'completed',
    'canceled'
);

CREATE TYPE discord_event_entity_type AS ENUM (
    'stage_instance',
    'voice',
    'external'
);

CREATE TABLE discord_scheduled_events (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT REFERENCES discord_channels(id) ON DELETE SET NULL,

    creator_id BIGINT REFERENCES discord_users(id),

    name VARCHAR(100) NOT NULL,
    description TEXT,

    scheduled_start_time TIMESTAMP NOT NULL,
    scheduled_end_time TIMESTAMP,

    privacy_level SMALLINT NOT NULL,
    status discord_event_status NOT NULL,
    entity_type discord_event_entity_type NOT NULL,

    -- External location
    entity_metadata JSONB,

    -- Cover image
    image VARCHAR(255),

    -- User count
    user_count INTEGER DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_events_guild ON discord_scheduled_events(guild_id, scheduled_start_time);
CREATE INDEX idx_events_status ON discord_scheduled_events(status, scheduled_start_time);
CREATE INDEX idx_events_upcoming ON discord_scheduled_events(scheduled_start_time)
    WHERE status = 'scheduled';
```

### 25. Event Participants

```sql
CREATE TABLE discord_event_participants (
    event_id BIGINT NOT NULL REFERENCES discord_scheduled_events(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES discord_users(id),

    interested_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    removed_at TIMESTAMP,

    PRIMARY KEY (event_id, user_id)
);

CREATE INDEX idx_event_participants_user ON discord_event_participants(user_id);
CREATE INDEX idx_event_participants_active ON discord_event_participants(event_id, removed_at)
    WHERE removed_at IS NULL;
```

### 26. Audit Log (Bot Actions)

```sql
CREATE TYPE discord_audit_action AS ENUM (
    'message_sent',
    'message_edited',
    'message_deleted',
    'member_banned',
    'member_kicked',
    'member_role_added',
    'member_role_removed',
    'channel_created',
    'channel_updated',
    'channel_deleted',
    'command_executed',
    'webhook_created',
    'webhook_deleted',
    'other'
);

CREATE TABLE discord_audit_log (
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT REFERENCES discord_guilds(id) ON DELETE CASCADE,

    action discord_audit_action NOT NULL,
    actor_id BIGINT REFERENCES discord_users(id),  -- Who performed the action
    target_id BIGINT,  -- What was acted upon

    -- Context
    channel_id BIGINT REFERENCES discord_channels(id) ON DELETE SET NULL,
    message_id BIGINT,

    -- Details
    reason TEXT,
    metadata JSONB,

    -- Timestamp
    performed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_audit_guild ON discord_audit_log(guild_id, performed_at DESC);
CREATE INDEX idx_audit_action ON discord_audit_log(action, performed_at DESC);
CREATE INDEX idx_audit_actor ON discord_audit_log(actor_id, performed_at DESC);
```

### 27. Bot Statistics

```sql
CREATE TABLE discord_bot_statistics (
    id SERIAL PRIMARY KEY,

    -- Counts
    total_guilds INTEGER NOT NULL,
    total_channels INTEGER NOT NULL,
    total_users INTEGER NOT NULL,
    total_messages_today INTEGER NOT NULL,
    total_commands_today INTEGER NOT NULL,

    -- Performance
    average_response_time_ms INTEGER,
    error_rate NUMERIC(5, 2),

    -- System
    uptime_seconds BIGINT,
    memory_usage_mb INTEGER,

    -- Timestamp
    recorded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_stats_recorded_at ON discord_bot_statistics(recorded_at DESC);
```

### 28. Integration with Boticelli Narratives

```sql
CREATE TABLE discord_narrative_posts (
    id SERIAL PRIMARY KEY,
    narrative_execution_id INTEGER REFERENCES narrative_executions(id) ON DELETE CASCADE,
    act_number INTEGER NOT NULL,

    -- Discord context
    guild_id BIGINT NOT NULL REFERENCES discord_guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES discord_channels(id) ON DELETE CASCADE,
    message_id BIGINT REFERENCES discord_messages(id) ON DELETE SET NULL,

    -- Post configuration
    content TEXT NOT NULL,
    embed_config JSONB,  -- Embed configuration
    components_config JSONB,  -- Button/menu configuration

    -- Scheduling
    scheduled_for TIMESTAMP,
    posted_at TIMESTAMP,

    -- Status
    status VARCHAR(50) NOT NULL,  -- 'pending', 'posted', 'failed', 'scheduled'
    error_message TEXT,

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_narrative_posts_execution ON discord_narrative_posts(narrative_execution_id);
CREATE INDEX idx_narrative_posts_guild ON discord_narrative_posts(guild_id, posted_at DESC);
CREATE INDEX idx_narrative_posts_scheduled ON discord_narrative_posts(scheduled_for)
    WHERE status = 'scheduled';
CREATE INDEX idx_narrative_posts_pending ON discord_narrative_posts(status, created_at)
    WHERE status = 'pending';
```

## Diesel Migration Structure

To implement this schema with Diesel (matching Boticelli's existing database setup):

```bash
# Core entities
diesel migration generate create_discord_guilds
diesel migration generate create_discord_channels
diesel migration generate create_discord_users
diesel migration generate create_discord_guild_members
diesel migration generate create_discord_roles
diesel migration generate create_discord_member_roles

# Permissions
diesel migration generate create_discord_channel_permissions

# Messages and content
diesel migration generate create_discord_messages
diesel migration generate create_discord_message_mentions
diesel migration generate create_discord_message_attachments
diesel migration generate create_discord_message_embeds
diesel migration generate create_discord_embed_fields
diesel migration generate create_discord_message_reactions
diesel migration generate create_discord_message_components

# Guild features
diesel migration generate create_discord_emojis
diesel migration generate create_discord_emoji_roles
diesel migration generate create_discord_stickers
diesel migration generate create_discord_message_stickers

# Commands and interactions
diesel migration generate create_discord_application_commands
diesel migration generate create_discord_command_executions

# Voice and presence
diesel migration generate create_discord_voice_states

# Webhooks and invites
diesel migration generate create_discord_webhooks
diesel migration generate create_discord_invites

# Events
diesel migration generate create_discord_scheduled_events
diesel migration generate create_discord_event_participants

# Logging and analytics
diesel migration generate create_discord_audit_log
diesel migration generate create_discord_bot_statistics

# Boticelli integration
diesel migration generate create_discord_narrative_posts
```

## Schema Design Decisions

### 1. Snowflake IDs as BIGINT

Discord uses Twitter-style snowflake IDs (64-bit integers). PostgreSQL's `BIGINT` type is perfect for this.

### 2. Denormalization for Performance

Some fields are intentionally denormalized for query performance:
- `guild_id` in messages (also available via channel)
- Message counts and member counts cached on channels/guilds

### 3. Soft Deletes

Key tables track "left" or "deleted" timestamps rather than hard deletes:
- `discord_guilds.left_at`
- `discord_guild_members.left_at`
- `discord_webhooks.deleted_at`
- `discord_invites.revoked_at`

This preserves historical data for analytics.

### 4. JSONB for Flexibility

Discord's API evolves frequently. Using JSONB for complex/nested structures provides flexibility:
- Command options
- Embed configurations
- Component configurations
- Event metadata

### 5. Partitioning for Scale

The `discord_messages` table should be partitioned by time (monthly or quarterly) as it will grow the fastest.

### 6. Indexes for Common Queries

Indexes are designed around expected query patterns:
- Recent messages by channel
- Commands executed in a guild
- Active voice states
- Pending/scheduled posts

### 7. Integration with Boticelli

The `discord_narrative_posts` table links Discord posts to Boticelli's narrative execution system, enabling:
- Scheduled multi-channel posts
- Act-based content distribution
- Post tracking and analytics
- Error handling and retry logic

## Next Steps

1. **Implement Diesel models** for each table
2. **Create repository layer** following Boticelli's pattern
3. **Build Serenity event handlers** to populate the database
4. **Implement caching strategy** (which data to cache vs. query)
5. **Create analytics queries** for bot insights
6. **Build admin dashboard** for guild management
7. **Integrate with narrative system** for automated posting
8. **Add rate limiting** per guild/channel
9. **Implement message queue** for scheduled posts
10. **Create backup/archival strategy** for old data

## Performance Considerations

- **Connection pooling**: Use r2d2 with Diesel
- **Batch inserts**: For bulk message/event ingestion
- **Read replicas**: For analytics queries
- **Materialized views**: For dashboard statistics
- **Table partitioning**: For messages and audit logs
- **Archive old data**: Move data older than 90 days to cold storage
- **Cache layer**: Redis for frequently accessed guild configs

## Security Notes

- **Never log** webhook tokens in plain text
- **Encrypt sensitive data** (webhook tokens, potentially message content)
- **Rate limit** database writes from event handlers
- **Validate** all data from Discord API before insertion
- **Monitor** for unusual patterns (spam, abuse)
- **GDPR compliance**: Support user data deletion requests
- **Backup regularly**: Especially guild configurations
