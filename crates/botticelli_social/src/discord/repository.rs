//! PostgreSQL repository for Discord data.
//!
//! This repository provides database operations for Discord entities including
//! guilds, channels, users, members, and roles.

use botticelli_database::schema::{
    discord_channels, discord_guild_members, discord_guilds, discord_member_roles, discord_roles,
    discord_users,
};
use botticelli_error::DatabaseError;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

use super::conversions::NewMemberRole;
use super::models::{
    ChannelRow, GuildMemberRow, GuildRow, NewChannel, NewGuild, NewGuildMember, NewRole, NewUser,
    RoleRow, UserRow,
};

/// Result type for Discord repository operations.
pub type DiscordResult<T> = Result<T, DatabaseError>;

/// PostgreSQL repository for Discord data.
///
/// Provides CRUD operations for Discord entities with proper error handling
/// and transaction support.
///
/// # Example
/// ```no_run
/// use botticelli_social::DiscordRepository;
/// use botticelli_database::establish_connection;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let conn = establish_connection()?;
///     let repo = DiscordRepository::new(conn);
///     // Use repo.store_guild(), get_guild(), etc.
///     Ok(())
/// }
/// ```
pub struct DiscordRepository {
    /// Database connection wrapped in Arc<Mutex> for async safety.
    ///
    /// Note: This is a simple implementation. For production use with high
    /// concurrency, consider using a connection pool like r2d2 or deadpool.
    conn: Arc<Mutex<PgConnection>>,
}

impl DiscordRepository {
    /// Create a new Discord repository.
    ///
    /// # Arguments
    /// * `conn` - A PostgreSQL connection
    ///
    /// # Note
    /// The connection is wrapped in Arc<Mutex> to allow async access.
    pub fn new(conn: PgConnection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create a repository from an Arc<Mutex<PgConnection>> (for sharing connections).
    pub fn from_arc(conn: Arc<Mutex<PgConnection>>) -> Self {
        Self { conn }
    }

    // ============================================================================
    // Guild Operations
    // ============================================================================

    /// Store or update a guild in the database.
    ///
    /// Uses INSERT ... ON CONFLICT to upsert the guild.
    #[instrument(skip(self), fields(guild_id = %guild.id))]
    pub async fn store_guild(&self, guild: &NewGuild) -> DiscordResult<GuildRow> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_guilds::table)
            .values(guild)
            .on_conflict(discord_guilds::id)
            .do_update()
            .set((
                discord_guilds::name.eq(&guild.name),
                discord_guilds::icon.eq(&guild.icon),
                discord_guilds::banner.eq(&guild.banner),
                discord_guilds::owner_id.eq(guild.owner_id),
                discord_guilds::features.eq(&guild.features),
                discord_guilds::description.eq(&guild.description),
                discord_guilds::member_count.eq(guild.member_count),
                discord_guilds::updated_at.eq(diesel::dsl::now),
            ))
            .get_result(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Get a guild by ID.
    #[instrument(skip(self))]
    pub async fn get_guild(&self, guild_id: i64) -> DiscordResult<Option<GuildRow>> {
        let mut conn = self.conn.lock().await;

        discord_guilds::table
            .find(guild_id)
            .first(&mut *conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    /// List all active guilds (where bot_active = true and left_at is null).
    #[instrument(skip(self))]
    pub async fn list_active_guilds(&self) -> DiscordResult<Vec<GuildRow>> {
        let mut conn = self.conn.lock().await;

        discord_guilds::table
            .filter(discord_guilds::bot_active.eq(true))
            .filter(discord_guilds::left_at.is_null())
            .order(discord_guilds::name.asc())
            .load(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Mark a guild as left (soft delete).
    #[instrument(skip(self))]
    pub async fn mark_guild_left(&self, guild_id: i64) -> DiscordResult<()> {
        let mut conn = self.conn.lock().await;

        diesel::update(discord_guilds::table.find(guild_id))
            .set((
                discord_guilds::left_at.eq(diesel::dsl::now),
                discord_guilds::bot_active.eq(false),
            ))
            .execute(&mut *conn)
            .map_err(DatabaseError::from)?;

        Ok(())
    }

    // ============================================================================
    // User Operations
    // ============================================================================

    /// Store or update a user in the database.
    #[instrument(skip(self), fields(user_id = %user.id))]
    pub async fn store_user(&self, user: &NewUser) -> DiscordResult<UserRow> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_users::table)
            .values(user)
            .on_conflict(discord_users::id)
            .do_update()
            .set((
                discord_users::username.eq(&user.username),
                discord_users::discriminator.eq(&user.discriminator),
                discord_users::global_name.eq(&user.global_name),
                discord_users::avatar.eq(&user.avatar),
                discord_users::last_seen.eq(diesel::dsl::now),
                discord_users::updated_at.eq(diesel::dsl::now),
            ))
            .get_result(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Get a user by ID.
    #[instrument(skip(self))]
    pub async fn get_user(&self, user_id: i64) -> DiscordResult<Option<UserRow>> {
        let mut conn = self.conn.lock().await;

        discord_users::table
            .find(user_id)
            .first(&mut *conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    // ============================================================================
    // Channel Operations
    // ============================================================================

    /// Store or update a channel in the database.
    #[instrument(skip(self), fields(channel_id = %channel.id))]
    pub async fn store_channel(&self, channel: &NewChannel) -> DiscordResult<ChannelRow> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_channels::table)
            .values(channel)
            .on_conflict(discord_channels::id)
            .do_update()
            .set((
                discord_channels::name.eq(&channel.name),
                discord_channels::channel_type.eq(channel.channel_type),
                discord_channels::position.eq(channel.position),
                discord_channels::topic.eq(&channel.topic),
                discord_channels::nsfw.eq(channel.nsfw),
                discord_channels::parent_id.eq(channel.parent_id),
                discord_channels::updated_at.eq(diesel::dsl::now),
            ))
            .get_result(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Get a channel by ID.
    #[instrument(skip(self))]
    pub async fn get_channel(&self, channel_id: i64) -> DiscordResult<Option<ChannelRow>> {
        let mut conn = self.conn.lock().await;

        discord_channels::table
            .find(channel_id)
            .first(&mut *conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    /// List all channels in a guild.
    #[instrument(skip(self))]
    pub async fn list_guild_channels(&self, guild_id: i64) -> DiscordResult<Vec<ChannelRow>> {
        let mut conn = self.conn.lock().await;

        discord_channels::table
            .filter(discord_channels::guild_id.eq(guild_id))
            .order(discord_channels::position.asc())
            .load(&mut *conn)
            .map_err(DatabaseError::from)
    }

    // ============================================================================
    // Guild Member Operations
    // ============================================================================

    /// Store or update a guild member in the database.
    #[instrument(skip(self), fields(guild_id = %member.guild_id, user_id = %member.user_id))]
    pub async fn store_guild_member(
        &self,
        member: &NewGuildMember,
    ) -> DiscordResult<GuildMemberRow> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_guild_members::table)
            .values(member)
            .on_conflict((
                discord_guild_members::guild_id,
                discord_guild_members::user_id,
            ))
            .do_update()
            .set((
                discord_guild_members::nick.eq(&member.nick),
                discord_guild_members::avatar.eq(&member.avatar),
                discord_guild_members::premium_since.eq(member.premium_since),
                discord_guild_members::updated_at.eq(diesel::dsl::now),
            ))
            .get_result(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Get a guild member by guild ID and user ID.
    #[instrument(skip(self))]
    pub async fn get_guild_member(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> DiscordResult<Option<GuildMemberRow>> {
        let mut conn = self.conn.lock().await;

        discord_guild_members::table
            .filter(discord_guild_members::guild_id.eq(guild_id))
            .filter(discord_guild_members::user_id.eq(user_id))
            .first(&mut *conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    /// List all active members in a guild (where left_at is null).
    #[instrument(skip(self))]
    pub async fn list_guild_members(&self, guild_id: i64) -> DiscordResult<Vec<GuildMemberRow>> {
        let mut conn = self.conn.lock().await;

        discord_guild_members::table
            .filter(discord_guild_members::guild_id.eq(guild_id))
            .filter(discord_guild_members::left_at.is_null())
            .order(discord_guild_members::joined_at.asc())
            .load(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Mark a guild member as left (soft delete).
    #[instrument(skip(self))]
    pub async fn mark_member_left(&self, guild_id: i64, user_id: i64) -> DiscordResult<()> {
        let mut conn = self.conn.lock().await;

        diesel::update(
            discord_guild_members::table
                .filter(discord_guild_members::guild_id.eq(guild_id))
                .filter(discord_guild_members::user_id.eq(user_id)),
        )
        .set(discord_guild_members::left_at.eq(diesel::dsl::now))
        .execute(&mut *conn)
        .map_err(DatabaseError::from)?;

        Ok(())
    }

    // ============================================================================
    // Role Operations
    // ============================================================================

    /// Store or update a role in the database.
    #[instrument(skip(self), fields(role_id = %role.id))]
    pub async fn store_role(&self, role: &NewRole) -> DiscordResult<RoleRow> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_roles::table)
            .values(role)
            .on_conflict(discord_roles::id)
            .do_update()
            .set((
                discord_roles::name.eq(&role.name),
                discord_roles::color.eq(role.color),
                discord_roles::position.eq(role.position),
                discord_roles::permissions.eq(role.permissions),
                discord_roles::hoist.eq(role.hoist),
                discord_roles::mentionable.eq(role.mentionable),
                discord_roles::updated_at.eq(diesel::dsl::now),
            ))
            .get_result(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Get a role by ID.
    #[instrument(skip(self))]
    pub async fn get_role(&self, role_id: i64) -> DiscordResult<Option<RoleRow>> {
        let mut conn = self.conn.lock().await;

        discord_roles::table
            .find(role_id)
            .first(&mut *conn)
            .optional()
            .map_err(DatabaseError::from)
    }

    /// List all roles in a guild ordered by position.
    #[instrument(skip(self))]
    pub async fn list_guild_roles(&self, guild_id: i64) -> DiscordResult<Vec<RoleRow>> {
        let mut conn = self.conn.lock().await;

        discord_roles::table
            .filter(discord_roles::guild_id.eq(guild_id))
            .order(discord_roles::position.desc())
            .load(&mut *conn)
            .map_err(DatabaseError::from)
    }

    /// Store a member role assignment in the database.
    ///
    /// Uses INSERT ... ON CONFLICT to upsert the role assignment.
    #[instrument(skip(self), fields(guild_id = %member_role.guild_id, user_id = %member_role.user_id, role_id = %member_role.role_id))]
    pub async fn store_member_role(&self, member_role: &NewMemberRole) -> DiscordResult<()> {
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_member_roles::table)
            .values(member_role)
            .on_conflict((
                discord_member_roles::guild_id,
                discord_member_roles::user_id,
                discord_member_roles::role_id,
            ))
            .do_update()
            .set((
                discord_member_roles::assigned_at.eq(member_role.assigned_at),
                discord_member_roles::assigned_by.eq(member_role.assigned_by),
            ))
            .execute(&mut *conn)
            .map_err(DatabaseError::from)?;

        Ok(())
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
        let mut conn = self.conn.lock().await;

        diesel::insert_into(discord_member_roles::table)
            .values((
                discord_member_roles::guild_id.eq(guild_id),
                discord_member_roles::user_id.eq(user_id),
                discord_member_roles::role_id.eq(role_id),
                discord_member_roles::assigned_by.eq(assigned_by),
            ))
            .on_conflict((
                discord_member_roles::guild_id,
                discord_member_roles::user_id,
                discord_member_roles::role_id,
            ))
            .do_nothing()
            .execute(&mut *conn)
            .map_err(DatabaseError::from)?;

        Ok(())
    }

    /// Remove a role from a guild member.
    #[instrument(skip(self))]
    pub async fn remove_role(
        &self,
        guild_id: i64,
        user_id: i64,
        role_id: i64,
    ) -> DiscordResult<()> {
        let mut conn = self.conn.lock().await;

        diesel::delete(
            discord_member_roles::table
                .filter(discord_member_roles::guild_id.eq(guild_id))
                .filter(discord_member_roles::user_id.eq(user_id))
                .filter(discord_member_roles::role_id.eq(role_id)),
        )
        .execute(&mut *conn)
        .map_err(DatabaseError::from)?;

        Ok(())
    }
}
