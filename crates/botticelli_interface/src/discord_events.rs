//! Discord event processing traits.
//!
//! This module defines the interface for processing Discord events with
//! proper error handling. All types are aliases to allow different
//! implementations to use their own concrete types.

use async_trait::async_trait;

/// Result type for event processing operations.
///
/// The error type is generic to allow different implementations
/// to use their own error types (DiscordError, TestError, etc.).
pub type EventResult<T, E> = Result<T, E>;

/// Discord event processor trait.
///
/// This trait defines the interface for processing Discord events with
/// proper error handling. Implementations define their own error types
/// and severity types via associated type aliases.
///
/// # Type Parameters
///
/// All types are aliases to allow different implementations:
/// - `Error`: The error type (must be Send + Sync + std::error::Error)
/// - `Severity`: The severity type (implementation-specific)
/// - `Guild`, `Channel`, etc.: Entity types (Serenity, mock, custom)
///
/// # Example
///
/// ```rust
/// use botticelli_interface::{DiscordEventProcessor, EventResult};
/// use async_trait::async_trait;
/// use std::fmt;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum MySeverity { Critical, Warning, Info }
///
/// #[derive(Debug)]
/// struct MyError(String);
///
/// impl fmt::Display for MyError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// impl std::error::Error for MyError {}
///
/// struct MyGuild;
/// struct MyChannel;
/// struct MyMember;
/// struct MyRole;
/// struct MyUser;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl DiscordEventProcessor for MyProcessor {
///     type Error = MyError;
///     type Severity = MySeverity;
///     type Guild = MyGuild;
///     type Channel = MyChannel;
///     type Member = MyMember;
///     type Role = MyRole;
///     type User = MyUser;
///     
///     fn error_severity(&self, _error: &Self::Error) -> Self::Severity {
///         MySeverity::Warning
///     }
///     
///     fn error_is_retryable(&self, _error: &Self::Error) -> bool {
///         false
///     }
///     
///     fn error_context(&self, error: &Self::Error) -> String {
///         error.to_string()
///     }
///     
///     async fn process_guild_create(
///         &self,
///         _guild: &Self::Guild,
///         _is_new: Option<bool>,
///     ) -> EventResult<(), Self::Error> {
///         Ok(())
///     }
///     
///     async fn process_channel_create(
///         &self,
///         _channel: &Self::Channel,
///     ) -> EventResult<(), Self::Error> {
///         Ok(())
///     }
///     
///     async fn process_member_add(
///         &self,
///         _member: &Self::Member,
///     ) -> EventResult<(), Self::Error> {
///         Ok(())
///     }
///     
///     async fn process_role_create(
///         &self,
///         _role: &Self::Role,
///     ) -> EventResult<(), Self::Error> {
///         Ok(())
///     }
///     
///     async fn process_ready(
///         &self,
///         _user: &Self::User,
///         _guild_count: usize,
///     ) -> EventResult<(), Self::Error> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait DiscordEventProcessor {
    /// Error type for this processor.
    ///
    /// Must implement std::error::Error + Send + Sync for async compatibility.
    type Error: std::error::Error + Send + Sync;
    
    /// Severity type for error classification.
    ///
    /// Implementation defines the severity levels (e.g., Critical/Warning/Info).
    type Severity;
    
    /// Guild type (allows different representations: Serenity, mock, etc.)
    type Guild;
    
    /// Channel type
    type Channel;
    
    /// Member type
    type Member;
    
    /// Role type
    type Role;
    
    /// User type
    type User;
    
    /// Get the severity level for an error.
    ///
    /// Used by the framework to decide whether to abort or continue processing.
    fn error_severity(&self, error: &Self::Error) -> Self::Severity;
    
    /// Check if an error is retryable.
    ///
    /// Used for retry logic, circuit breakers, etc.
    fn error_is_retryable(&self, error: &Self::Error) -> bool;
    
    /// Get human-readable context for an error.
    ///
    /// Used for logging and diagnostics.
    fn error_context(&self, error: &Self::Error) -> String;
    
    /// Process a guild_create event.
    ///
    /// This should store the guild and all its entities (channels, roles, members).
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be processed. The severity
    /// determines whether processing should abort or continue.
    async fn process_guild_create(
        &self,
        guild: &Self::Guild,
        is_new: Option<bool>,
    ) -> EventResult<(), Self::Error>;
    
    /// Process a channel_create event.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel cannot be stored.
    async fn process_channel_create(
        &self,
        channel: &Self::Channel,
    ) -> EventResult<(), Self::Error>;
    
    /// Process a guild_member_addition event.
    ///
    /// # Errors
    ///
    /// Returns an error if the member cannot be stored.
    async fn process_member_add(
        &self,
        member: &Self::Member,
    ) -> EventResult<(), Self::Error>;
    
    /// Process a role_create event.
    ///
    /// # Errors
    ///
    /// Returns an error if the role cannot be stored.
    async fn process_role_create(
        &self,
        role: &Self::Role,
    ) -> EventResult<(), Self::Error>;
    
    /// Process when bot connects (ready event).
    ///
    /// # Errors
    ///
    /// Returns an error if the ready state cannot be processed.
    async fn process_ready(
        &self,
        user: &Self::User,
        guild_count: usize,
    ) -> EventResult<(), Self::Error>;
}
