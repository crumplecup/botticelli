-- Create discord_users table for storing Discord user information
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

-- Indexes for common queries
CREATE INDEX idx_users_username ON discord_users(username);
CREATE INDEX idx_users_bot ON discord_users(bot);
CREATE INDEX idx_users_last_seen ON discord_users(last_seen);
