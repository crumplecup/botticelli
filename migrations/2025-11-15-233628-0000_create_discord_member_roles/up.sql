-- Create discord_member_roles junction table for member-role assignments
CREATE TABLE discord_member_roles (
    guild_id BIGINT,
    user_id BIGINT,
    role_id BIGINT REFERENCES discord_roles(id) ON DELETE CASCADE,

    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by BIGINT,  -- User who assigned the role

    PRIMARY KEY (guild_id, user_id, role_id),
    FOREIGN KEY (guild_id, user_id) REFERENCES discord_guild_members(guild_id, user_id) ON DELETE CASCADE
);

-- Indexes for common queries
CREATE INDEX idx_member_roles_user ON discord_member_roles(guild_id, user_id);
CREATE INDEX idx_member_roles_role ON discord_member_roles(role_id);
