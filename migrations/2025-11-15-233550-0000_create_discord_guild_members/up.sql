-- Create discord_guild_members table for storing guild-specific member data
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

-- Indexes for common queries
CREATE INDEX idx_guild_members_user ON discord_guild_members(user_id);
CREATE INDEX idx_guild_members_joined ON discord_guild_members(joined_at);
CREATE INDEX idx_guild_members_active ON discord_guild_members(left_at) WHERE left_at IS NULL;
CREATE INDEX idx_guild_members_boosters ON discord_guild_members(premium_since) WHERE premium_since IS NOT NULL;
