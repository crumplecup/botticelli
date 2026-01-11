# Interface Dependency Fix: Event Processing Traits

## Problem Statement

Currently, `botticelli_social` event handlers violate the dependency boundary by implementing Serenity's `EventHandler` trait directly. This creates several issues:

1. **No error propagation**: Serenity's trait returns `()`, hiding failures
2. **Inconsistent error handling**: Each handler decides independently how to handle errors
3. **Tight coupling**: Social crate depends on Serenity specifics
4. **Untestable**: Can't test event processing logic without mocking Serenity
5. **No retry logic**: Errors are logged but can't be retried

## Solution: Internal Trait with Type Aliases

Create an internal trait in `botticelli_interface` that defines event processing behavior with proper error handling, using type aliases to avoid concrete type coupling.

### Architecture

```
Serenity EventHandler (external, can't change)
    ↓ (thin adapter layer)
botticelli_interface::DiscordEventProcessor (our trait, Result-returning)
    ↓ (implements with ?)
Helper methods (store_guild, store_channel, etc.)
```

## Implementation Plan

### Phase 1: Define Core Types in botticelli_interface

```rust
// crates/botticelli_interface/src/discord_events.rs

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
/// 
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum MySeverity { Critical, Warning, Info }
/// 
/// #[derive(Debug)]
/// struct MyError(String);
/// 
/// impl std::error::Error for MyError {}
/// 
/// struct MyProcessor;
/// 
/// #[async_trait::async_trait]
/// impl DiscordEventProcessor for MyProcessor {
///     type Error = MyError;
///     type Severity = MySeverity;
///     type Guild = MyGuild;
///     // ... other types
///     
///     fn error_severity(&self, error: &Self::Error) -> Self::Severity {
///         MySeverity::Warning
///     }
///     
///     fn error_is_retryable(&self, error: &Self::Error) -> bool {
///         false
///     }
///     
///     // ... implement event methods
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
    async fn process_channel_create(
        &self,
        channel: &Self::Channel,
    ) -> EventResult<(), Self::Error>;
    
    /// Process a guild_member_addition event.
    async fn process_member_add(
        &self,
        member: &Self::Member,
    ) -> EventResult<(), Self::Error>;
    
    /// Process a role_create event.
    async fn process_role_create(
        &self,
        role: &Self::Role,
    ) -> EventResult<(), Self::Error>;
    
    /// Process when bot connects (ready event).
    async fn process_ready(
        &self,
        user: &Self::User,
        guild_count: usize,
    ) -> EventResult<(), Self::Error>;
}
```

### Phase 2: Define Severity Type and Implement Trait Methods

```rust
// crates/botticelli_error/src/discord.rs

/// Error severity level for Discord event processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordErrorSeverity {
    /// Critical error - abort event processing immediately
    Critical,
    /// Non-critical - log and continue with other entities
    Warning,
    /// Informational - entity already processed or skipped
    Info,
}

impl DiscordError {
    /// Get the severity level for this error.
    pub fn severity(&self) -> DiscordErrorSeverity {
        use DiscordErrorSeverity::*;
        use DiscordErrorKind::*;
        
        match self.kind() {
            // Critical errors - can't continue processing
            ConnectionFailed(_) | ConnectionFailedWithSource { .. } => Critical,
            InvalidToken => Critical,
            DatabaseError(_) => Critical,
            DataConversionError(_) => Critical,
            ConfigurationError(_) => Critical,
            
            // Warnings - log and continue
            GuildNotFound(_) => Warning,
            ChannelNotFound(_) => Warning,
            UserNotFound(_) => Warning,
            RoleNotFound(_) => Warning,
            MessageSendFailed(_) => Warning,
            InteractionFailed(_) => Warning,
            InvalidId(_) => Warning,
            
            // Info - expected conditions
            SerenityError(_) => Info,
            InsufficientPermissions(_) => Info,
        }
    }
    
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        use DiscordErrorKind::*;
        
        match self.kind() {
            // Network/transient errors - retryable
            ConnectionFailed(_) | ConnectionFailedWithSource { .. } => true,
            SerenityError(_) => true, // May be rate limit
            DatabaseError(_) => true, // May be temporary lock
            
            // Permanent errors - not retryable
            InvalidToken => false,
            DataConversionError(_) => false,
            GuildNotFound(_) => false,
            ChannelNotFound(_) => false,
            UserNotFound(_) => false,
            RoleNotFound(_) => false,
            InvalidId(_) => false,
            InsufficientPermissions(_) => false,
            MessageSendFailed(_) => false,
            InteractionFailed(_) => false,
            ConfigurationError(_) => false,
        }
    }
    
    /// Get human-readable context for logging.
    pub fn error_context(&self) -> String {
        format!("{} at {}:{}", self.kind(), self.file(), self.line())
    }
}
```

### Phase 3: Implement DiscordEventProcessor for BotticelliHandler

```rust
// crates/botticelli_social/src/discord/handler.rs

#[async_trait]
impl DiscordEventProcessor for BotticelliHandler {
    type Error = crate::DiscordError;
    type Guild = serenity::model::guild::Guild;
    type Channel = serenity::model::channel::GuildChannel;
    type Member = serenity::model::guild::Member;
    type Role = serenity::model::guild::Role;
    type User = serenity::model::user::User;
    
    #[instrument(skip(self, guild), fields(guild_id = %guild.id, guild_name = %guild.name))]
    async fn process_guild_create(
        &self,
        guild: &Self::Guild,
        is_new: Option<bool>,
    ) -> EventResult<(), Self::Error> {
        debug!(is_new = ?is_new, "Processing guild_create event");
        
        // Critical: Store the guild first
        self.store_guild(guild).await?;
        
        // Best effort: Store all channels (collect errors)
        let mut channel_errors = Vec::new();
        for channel in guild.channels.values() {
            if let Err(e) = self.store_channel(Some(guild.id), &Channel::Guild(channel.clone())).await {
                if e.severity() == ErrorSeverity::Critical {
                    return Err(e); // Abort on critical
                }
                channel_errors.push((channel.id, e));
            }
        }
        
        // Best effort: Store all roles
        let mut role_errors = Vec::new();
        for role in guild.roles.values() {
            if let Err(e) = self.store_role(guild.id, role).await {
                if e.severity() == ErrorSeverity::Critical {
                    return Err(e);
                }
                role_errors.push((role.id, e));
            }
        }
        
        // Best effort: Store all members
        let mut member_errors = Vec::new();
        for member in guild.members.values() {
            if let Err(e) = self.store_member(guild.id, member).await {
                if e.severity() == ErrorSeverity::Critical {
                    return Err(e);
                }
                member_errors.push((member.user.id, e));
            }
        }
        
        // Log any warnings
        if !channel_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = channel_errors.len(),
                "Some channels failed to store"
            );
        }
        if !role_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = role_errors.len(),
                "Some roles failed to store"
            );
        }
        if !member_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = member_errors.len(),
                "Some members failed to store"
            );
        }
        
        info!(
            guild_id = %guild.id,
            channels_stored = guild.channels.len() - channel_errors.len(),
            roles_stored = guild.roles.len() - role_errors.len(),
            members_stored = guild.members.len() - member_errors.len(),
            "Successfully processed guild_create"
        );
        
        Ok(())
    }
    
    #[instrument(skip(self, channel))]
    async fn process_channel_create(
        &self,
        channel: &Self::Channel,
    ) -> EventResult<(), Self::Error> {
        self.store_channel(Some(channel.guild_id), &Channel::Guild(channel.clone())).await
    }
    
    #[instrument(skip(self, member))]
    async fn process_member_add(
        &self,
        member: &Self::Member,
    ) -> EventResult<(), Self::Error> {
        self.store_member(member.guild_id, member).await
    }
    
    #[instrument(skip(self, role))]
    async fn process_role_create(
        &self,
        role: &Self::Role,
    ) -> EventResult<(), Self::Error> {
        self.store_role(role.guild_id, role).await
    }
    
    #[instrument(skip(self, user), fields(username = %user.name))]
    async fn process_ready(
        &self,
        user: &Self::User,
        guild_count: usize,
    ) -> EventResult<(), Self::Error> {
        info!(
            bot_user = %user.name,
            bot_id = %user.id,
            guilds = guild_count,
            "Bot connected to Discord"
        );
        Ok(())
    }
}
```

### Phase 4: Thin Adapter in Serenity EventHandler

```rust
// crates/botticelli_social/src/discord/handler.rs

#[async_trait]
impl EventHandler for BotticelliHandler {
    /// Called when the bot successfully connects to Discord.
    async fn ready(&self, _ctx: Context, ready: Ready) {
        if let Err(e) = self.process_ready(&ready.user, ready.guilds.len()).await {
            error!(
                error = %e,
                context = %e.context(),
                severity = ?e.severity(),
                retryable = e.is_retryable(),
                "Failed to process ready event"
            );
        }
    }
    
    /// Called when a guild becomes available or the bot joins a guild.
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: Option<bool>) {
        if let Err(e) = self.process_guild_create(&guild, is_new).await {
            error!(
                guild_id = %guild.id,
                error = %e,
                context = %e.context(),
                severity = ?e.severity(),
                retryable = e.is_retryable(),
                "Failed to process guild_create event"
            );
            
            // Future: Could add retry logic here
            // if e.is_retryable() {
            //     self.schedule_retry(...);
            // }
        }
    }
    
    /// Called when a channel is created.
    async fn channel_create(&self, _ctx: Context, channel: GuildChannel) {
        if let Err(e) = self.process_channel_create(&channel).await {
            error!(
                channel_id = %channel.id,
                error = %e,
                context = %e.context(),
                severity = ?e.severity(),
                retryable = e.is_retryable(),
                "Failed to process channel_create event"
            );
        }
    }
    
    /// Called when a member joins a guild.
    async fn guild_member_addition(&self, _ctx: Context, new_member: Member) {
        if let Err(e) = self.process_member_add(&new_member).await {
            error!(
                guild_id = %new_member.guild_id,
                user_id = %new_member.user.id,
                error = %e,
                context = %e.context(),
                severity = ?e.severity(),
                retryable = e.is_retryable(),
                "Failed to process guild_member_addition event"
            );
        }
    }
    
    /// Called when a role is created.
    async fn guild_role_create(&self, _ctx: Context, new: Role) {
        if let Err(e) = self.process_role_create(&new).await {
            error!(
                guild_id = %new.guild_id,
                role_id = %new.id,
                error = %e,
                context = %e.context(),
                severity = ?e.severity(),
                retryable = e.is_retryable(),
                "Failed to process guild_role_create event"
            );
        }
    }
}
```

## Benefits

### 1. **Type Alias Flexibility**
- No concrete type coupling in the trait
- Can swap Serenity types for mocks in tests
- Different implementations can use different representations
- Trait stays generic and reusable

### 2. **Proper Error Handling**
- Errors propagate naturally with `?`
- Error severity encoded in type system
- Retry semantics available at decision point
- Structured error context for observability

### 3. **Clean Separation of Concerns**
```
┌─────────────────────────────────────┐
│  Serenity EventHandler (external)  │  ← Can't change
│  - Returns ()                        │
│  - Thin adapter only                │
└─────────────────────────────────────┘
              ↓ delegates to
┌─────────────────────────────────────┐
│  DiscordEventProcessor (our trait)  │  ← We control
│  - Returns Result<(), E>            │
│  - Type aliases for flexibility     │
│  - Business logic here              │
└─────────────────────────────────────┘
              ↓ uses
┌─────────────────────────────────────┐
│  Helper methods (store_*)           │  ← Implementation
│  - Returns DiscordResult<()>        │
│  - Database operations              │
└─────────────────────────────────────┘
```

### 4. **Testability**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockGuild {
        id: i64,
        name: String,
    }
    
    struct TestError(String);
    
    impl EventError for TestError {
        fn severity(&self) -> ErrorSeverity { ErrorSeverity::Warning }
        fn is_retryable(&self) -> bool { false }
        fn context(&self) -> String { self.0.clone() }
    }
    
    struct MockProcessor;
    
    #[async_trait]
    impl DiscordEventProcessor for MockProcessor {
        type Error = TestError;
        type Guild = MockGuild;
        type Channel = ();
        type Member = ();
        type Role = ();
        type User = ();
        
        async fn process_guild_create(
            &self,
            guild: &Self::Guild,
            _is_new: Option<bool>,
        ) -> EventResult<(), Self::Error> {
            // Test logic without Serenity
            assert_eq!(guild.name, "Test Guild");
            Ok(())
        }
        
        // ... other methods
    }
    
    #[tokio::test]
    async fn test_guild_processing() {
        let processor = MockProcessor;
        let guild = MockGuild {
            id: 123,
            name: "Test Guild".to_string(),
        };
        
        let result = processor.process_guild_create(&guild, Some(true)).await;
        assert!(result.is_ok());
    }
}
```

### 5. **Future Extensibility**
- Add retry queue easily (check `is_retryable()`)
- Add metrics based on severity
- Add circuit breaker for critical failures
- Add error aggregation for batch operations
- All without changing Serenity adapter

### 6. **Explicit Error Policy**
```rust
// In process_guild_create, we can now be explicit:
if let Err(e) = self.store_channel(...).await {
    match e.severity() {
        ErrorSeverity::Critical => return Err(e), // Abort
        ErrorSeverity::Warning => {
            // Log and continue
            warn!("Channel failed: {}", e.context());
        }
        ErrorSeverity::Info => {
            debug!("Channel skipped: {}", e.context());
        }
    }
}
```

## Implementation Steps

### Step 1: Add trait to botticelli_interface
- [ ] Create `src/discord_events.rs`
- [ ] Define `EventError` trait
- [ ] Define `DiscordEventProcessor` trait with type aliases
- [ ] Export from `lib.rs`

### Step 2: Implement EventError for DiscordError
- [ ] Add `impl EventError for DiscordError` in `botticelli_error`
- [ ] Map error kinds to severity levels
- [ ] Map error kinds to retry semantics
- [ ] Add context formatting

### Step 3: Implement DiscordEventProcessor
- [ ] Implement trait for `BotticelliHandler`
- [ ] Bind type aliases to Serenity types
- [ ] Move business logic from EventHandler methods
- [ ] Add error collection for batch operations
- [ ] Add structured logging with severity

### Step 4: Update Serenity EventHandler
- [ ] Thin adapter: delegate to `process_*` methods
- [ ] Log errors with full context (severity, retryable)
- [ ] Keep adapter minimal (5-10 lines per method)

### Step 5: Testing
- [ ] Unit tests for `EventError` trait implementation
- [ ] Integration tests with mock types
- [ ] Verify error severity mapping
- [ ] Verify retry logic decisions
- [ ] Verify critical vs warning handling

### Step 6: Documentation
- [ ] Document error severity guidelines
- [ ] Document retry semantics
- [ ] Add examples to trait docs
- [ ] Update HANDLER_TRAIT_ERROR_DESIGN.md with chosen approach

## Success Criteria

- ✅ All event processing logic returns `Result`
- ✅ No concrete types in trait (all type aliases)
- ✅ Error severity drives behavior (abort vs continue)
- ✅ Retry semantics available at adapter layer
- ✅ Tests don't require Serenity types
- ✅ Zero warnings, all tests pass
- ✅ Structured logging includes severity and retryable flags

## Non-Goals

- Don't implement actual retry logic yet (foundation only)
- Don't add metrics yet (can add later)
- Don't change Serenity's EventHandler (impossible)
- Don't over-engineer error types (keep simple)

## Rollout

1. **Phase 1**: Add trait and error impl (foundation)
2. **Phase 2**: Implement for BotticelliHandler (one event at a time)
3. **Phase 3**: Update adapter layer (thin wrappers)
4. **Phase 4**: Add tests (mock-based)
5. **Phase 5**: Document and commit

Each phase should compile and pass tests before proceeding.
