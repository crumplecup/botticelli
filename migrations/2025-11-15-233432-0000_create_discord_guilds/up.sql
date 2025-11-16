-- Create discord_guilds table for storing Discord server (guild) information
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

-- Indexes for common queries
CREATE INDEX idx_guilds_owner ON discord_guilds(owner_id);
CREATE INDEX idx_guilds_active ON discord_guilds(bot_active) WHERE bot_active = TRUE;
CREATE INDEX idx_guilds_left_at ON discord_guilds(left_at);
