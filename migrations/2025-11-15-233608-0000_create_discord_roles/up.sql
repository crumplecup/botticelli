-- Create discord_roles table for storing Discord role information
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

-- Indexes for common queries
CREATE INDEX idx_roles_guild ON discord_roles(guild_id);
CREATE INDEX idx_roles_position ON discord_roles(guild_id, position);
