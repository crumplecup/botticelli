-- Create ENUM type for Discord channel types
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

-- Create discord_channels table for storing Discord channel information
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

-- Indexes for common queries
CREATE INDEX idx_channels_guild ON discord_channels(guild_id);
CREATE INDEX idx_channels_parent ON discord_channels(parent_id);
CREATE INDEX idx_channels_type ON discord_channels(channel_type);
CREATE INDEX idx_channels_active_threads ON discord_channels(archived, channel_type)
    WHERE archived = FALSE AND channel_type IN ('public_thread', 'private_thread', 'announcement_thread');
