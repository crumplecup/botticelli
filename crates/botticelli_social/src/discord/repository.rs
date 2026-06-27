//! Storage repository for Discord entities.
//!
//! This repository provides persistence operations for Discord entities
//! (guilds, channels, users, members, roles) backed by `Arc<dyn BotStorage>`.
//! Each entity type is stored as a `ContentRecord` with a typed `table_name`.

use botticelli_interface::{BotStorage, ContentRecord};
use chrono::Utc;
use std::sync::Arc;
use tracing::instrument;

use super::conversions::NewMemberRole;
use super::error::{DiscordError, DiscordErrorKind, DiscordResult};
use super::models::{
    ChannelRow, GuildMemberRow, GuildRow, NewChannel, NewGuild, NewGuildMember, NewRole, NewUser,
    RoleRow, UserRow,
};

const TABLE_GUILDS: &str = "discord_guilds";
const TABLE_USERS: &str = "discord_users";
const TABLE_CHANNELS: &str = "discord_channels";
const TABLE_MEMBERS: &str = "discord_guild_members";
const TABLE_ROLES: &str = "discord_roles";
const TABLE_MEMBER_ROLES: &str = "discord_member_roles";

/// KV-backed repository for Discord data.
pub struct DiscordRepository {
    storage: Arc<dyn BotStorage>,
}

impl DiscordRepository {
    /// Create a new Discord repository backed by the given storage.
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        Self { storage }
    }

    fn storage_err(e: impl std::fmt::Display) -> DiscordError {
        DiscordError::new(DiscordErrorKind::DatabaseError(format!("{e}")))
    }

    fn json_err(e: impl std::fmt::Display) -> DiscordError {
        DiscordError::new(DiscordErrorKind::DatabaseError(format!("JSON: {e}")))
    }

    fn content_id(table: &str, id: &str) -> String {
        format!("{}:{}", table, id)
    }

    async fn get_row<T>(&self, table: &str, id: &str) -> DiscordResult<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let cid = Self::content_id(table, id);
        self.storage
            .get_content(&cid)
            .await
            .map_err(Self::storage_err)?
            .map(|r| serde_json::from_value::<T>(r.content_json).map_err(Self::json_err))
            .transpose()
    }

    async fn save_row<T>(&self, table: &str, id: &str, value: &T) -> DiscordResult<()>
    where
        T: serde::Serialize,
    {
        let cid = Self::content_id(table, id);
        let content_json = serde_json::to_value(value).map_err(Self::json_err)?;
        let record = ContentRecord {
            id: cid,
            table_name: table.to_string(),
            content_json,
            created_at: Utc::now(),
        };
        self.storage
            .save_content(&record)
            .await
            .map_err(Self::storage_err)
    }

    async fn list_rows<T>(&self, table: &str) -> DiscordResult<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let records = self
            .storage
            .list_content(table, 10_000)
            .await
            .map_err(Self::storage_err)?;

        records
            .into_iter()
            .map(|r| serde_json::from_value::<T>(r.content_json).map_err(Self::json_err))
            .collect()
    }

    // ── Guild operations ──────────────────────────────────────────────────────

    /// Store or update a guild.
    #[instrument(skip(self), fields(guild_id = %guild.id()))]
    pub async fn store_guild(&self, guild: &NewGuild) -> DiscordResult<GuildRow> {
        let id = guild.id.to_string();
        let now = Utc::now();

        let existing: Option<GuildRow> = self.get_row(TABLE_GUILDS, &id).await?;
        let created_at = existing.as_ref().map_or(now, |e| *e.created_at());

        let row = GuildRow::from_new(guild, created_at, now);
        self.save_row(TABLE_GUILDS, &id, &row).await?;
        Ok(row)
    }

    /// Get a guild by ID.
    #[instrument(skip(self))]
    pub async fn get_guild(&self, guild_id: i64) -> DiscordResult<Option<GuildRow>> {
        self.get_row(TABLE_GUILDS, &guild_id.to_string()).await
    }

    /// List active guilds (bot_active = true, left_at = null).
    #[instrument(skip(self))]
    pub async fn list_active_guilds(&self) -> DiscordResult<Vec<GuildRow>> {
        let all: Vec<GuildRow> = self.list_rows(TABLE_GUILDS).await?;
        Ok(all
            .into_iter()
            .filter(|g| g.bot_active().unwrap_or(false) && g.left_at().is_none())
            .collect())
    }

    /// Mark a guild as left (soft delete).
    #[instrument(skip(self))]
    pub async fn mark_guild_left(&self, guild_id: i64) -> DiscordResult<()> {
        let id = guild_id.to_string();
        let mut row: GuildRow = self
            .get_row(TABLE_GUILDS, &id)
            .await?
            .ok_or_else(|| DiscordError::new(DiscordErrorKind::GuildNotFound(guild_id)))?;

        row.left_at = Some(Utc::now());
        row.bot_active = Some(false);
        row.updated_at = Utc::now();

        self.save_row(TABLE_GUILDS, &id, &row).await
    }

    // ── User operations ───────────────────────────────────────────────────────

    /// Store or update a user.
    #[instrument(skip(self), fields(user_id = %user.id()))]
    pub async fn store_user(&self, user: &NewUser) -> DiscordResult<UserRow> {
        let id = user.id.to_string();
        let now = Utc::now();

        let existing: Option<UserRow> = self.get_row(TABLE_USERS, &id).await?;
        let mut row = UserRow::from_new(user, now);

        if let Some(prev) = existing {
            // Preserve first_seen timestamp on update
            row.first_seen = *prev.first_seen();
            row.created_at = *prev.created_at();
        }

        self.save_row(TABLE_USERS, &id, &row).await?;
        Ok(row)
    }

    /// Get a user by ID.
    #[instrument(skip(self))]
    pub async fn get_user(&self, user_id: i64) -> DiscordResult<Option<UserRow>> {
        self.get_row(TABLE_USERS, &user_id.to_string()).await
    }

    // ── Channel operations ────────────────────────────────────────────────────

    /// Store or update a channel.
    #[instrument(skip(self), fields(channel_id = %channel.id()))]
    pub async fn store_channel(&self, channel: &NewChannel) -> DiscordResult<ChannelRow> {
        let id = channel.id.to_string();
        let now = Utc::now();

        let existing: Option<ChannelRow> = self.get_row(TABLE_CHANNELS, &id).await?;
        let created_at = existing.as_ref().map_or(now, |e| *e.created_at());

        let row = ChannelRow::from_new(channel, created_at, now);
        self.save_row(TABLE_CHANNELS, &id, &row).await?;
        Ok(row)
    }

    /// Get a channel by ID.
    #[instrument(skip(self))]
    pub async fn get_channel(&self, channel_id: i64) -> DiscordResult<Option<ChannelRow>> {
        self.get_row(TABLE_CHANNELS, &channel_id.to_string()).await
    }

    /// List all channels belonging to a guild.
    #[instrument(skip(self))]
    pub async fn list_guild_channels(&self, guild_id: i64) -> DiscordResult<Vec<ChannelRow>> {
        let all: Vec<ChannelRow> = self.list_rows(TABLE_CHANNELS).await?;
        Ok(all
            .into_iter()
            .filter(|c| c.guild_id() == &Some(guild_id))
            .collect())
    }

    // ── Guild member operations ───────────────────────────────────────────────

    /// Store or update a guild member.
    #[instrument(skip(self), fields(guild_id = %member.guild_id(), user_id = %member.user_id()))]
    pub async fn store_guild_member(
        &self,
        member: &NewGuildMember,
    ) -> DiscordResult<GuildMemberRow> {
        let id = format!("{}:{}", member.guild_id, member.user_id);
        let now = Utc::now();

        let existing: Option<GuildMemberRow> = self.get_row(TABLE_MEMBERS, &id).await?;
        let created_at = existing.as_ref().map_or(now, |e| *e.created_at());

        let row = GuildMemberRow::from_new(member, created_at, now);
        self.save_row(TABLE_MEMBERS, &id, &row).await?;
        Ok(row)
    }

    /// Get a guild member by guild and user ID.
    #[instrument(skip(self))]
    pub async fn get_guild_member(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> DiscordResult<Option<GuildMemberRow>> {
        let id = format!("{}:{}", guild_id, user_id);
        self.get_row(TABLE_MEMBERS, &id).await
    }

    /// List active members in a guild (left_at = null).
    #[instrument(skip(self))]
    pub async fn list_guild_members(&self, guild_id: i64) -> DiscordResult<Vec<GuildMemberRow>> {
        let all: Vec<GuildMemberRow> = self.list_rows(TABLE_MEMBERS).await?;
        Ok(all
            .into_iter()
            .filter(|m| m.guild_id == guild_id && m.left_at().is_none())
            .collect())
    }

    /// Mark a guild member as left (soft delete).
    #[instrument(skip(self))]
    pub async fn mark_member_left(&self, guild_id: i64, user_id: i64) -> DiscordResult<()> {
        let id = format!("{}:{}", guild_id, user_id);
        let mut row: GuildMemberRow = self
            .get_row(TABLE_MEMBERS, &id)
            .await?
            .ok_or_else(|| DiscordError::new(DiscordErrorKind::UserNotFound(user_id)))?;

        row.left_at = Some(Utc::now());
        row.updated_at = Utc::now();

        self.save_row(TABLE_MEMBERS, &id, &row).await
    }

    // ── Role operations ───────────────────────────────────────────────────────

    /// Store or update a role.
    #[instrument(skip(self), fields(role_id = %role.id()))]
    pub async fn store_role(&self, role: &NewRole) -> DiscordResult<RoleRow> {
        let id = role.id.to_string();
        let now = Utc::now();

        let existing: Option<RoleRow> = self.get_row(TABLE_ROLES, &id).await?;
        let created_at = existing.as_ref().map_or(now, |e| *e.created_at());

        let row = RoleRow::from_new(role, created_at, now);
        self.save_row(TABLE_ROLES, &id, &row).await?;
        Ok(row)
    }

    /// Get a role by ID.
    #[instrument(skip(self))]
    pub async fn get_role(&self, role_id: i64) -> DiscordResult<Option<RoleRow>> {
        self.get_row(TABLE_ROLES, &role_id.to_string()).await
    }

    /// List all roles in a guild, ordered by position descending.
    #[instrument(skip(self))]
    pub async fn list_guild_roles(&self, guild_id: i64) -> DiscordResult<Vec<RoleRow>> {
        let mut all: Vec<RoleRow> = self.list_rows(TABLE_ROLES).await?;
        all.retain(|r| r.guild_id() == &guild_id);
        all.sort_by(|a, b| b.position().cmp(a.position()));
        Ok(all)
    }

    /// Store a member role assignment.
    #[instrument(skip(self), fields(
        guild_id = %member_role.guild_id(),
        user_id  = %member_role.user_id(),
        role_id  = %member_role.role_id()
    ))]
    pub async fn store_member_role(&self, member_role: &NewMemberRole) -> DiscordResult<()> {
        let id = format!(
            "{}:{}:{}",
            member_role.guild_id(),
            member_role.user_id(),
            member_role.role_id()
        );
        self.save_row(TABLE_MEMBER_ROLES, &id, member_role).await
    }

    /// Assign a role to a guild member.
    #[instrument(skip(self))]
    pub async fn assign_role(
        &self,
        guild_id: i64,
        user_id: i64,
        role_id: i64,
        assigned_by: Option<i64>,
    ) -> DiscordResult<()> {
        let id = format!("{}:{}:{}", guild_id, user_id, role_id);

        // Only assign if not already present
        if self
            .get_row::<serde_json::Value>(TABLE_MEMBER_ROLES, &id)
            .await?
            .is_some()
        {
            return Ok(());
        }

        let record = serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "role_id": role_id,
            "assigned_at": Utc::now().to_rfc3339(),
            "assigned_by": assigned_by,
        });
        self.save_row(TABLE_MEMBER_ROLES, &id, &record).await
    }

    /// Remove a role from a guild member.
    #[instrument(skip(self))]
    pub async fn remove_role(
        &self,
        guild_id: i64,
        user_id: i64,
        role_id: i64,
    ) -> DiscordResult<()> {
        let cid = Self::content_id(
            TABLE_MEMBER_ROLES,
            &format!("{}:{}:{}", guild_id, user_id, role_id),
        );
        self.storage
            .delete_content(&cid)
            .await
            .map_err(Self::storage_err)
    }
}
