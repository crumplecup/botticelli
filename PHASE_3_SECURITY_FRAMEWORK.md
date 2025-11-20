# Phase 3: Security Framework for Agentic Bot Commands

## Overview

Building on Phase 2's read-only bot command infrastructure, Phase 3 implements a comprehensive security framework that enables **safe write operations** for agentic bots. This framework addresses all security concerns identified in BOT_SECURITY_ANALYSIS.md.

## Goals

1. **Enable write operations safely** - Send messages, create channels, manage roles
2. **Prevent AI-driven attacks** - Block hallucinations, prompt injection, privilege escalation
3. **Ensure reversibility** - Undo/rollback mechanisms for all destructive operations
4. **Maintain audit trail** - Comprehensive logging for compliance and debugging
5. **Gradual rollout** - Start with low-risk operations, expand as confidence grows

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Narrative Executor                          │
│  (existing - orchestrates AI generation + bot commands)          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   BotCommandRegistry                             │
│  (existing - routes commands to platform executors)              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              SecureBotCommandExecutor (NEW)                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  1. Permission Check (NarrativePermissions)             │    │
│  │     - Does narrative have permission for this command?  │    │
│  │     - Is target resource accessible?                    │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │  2. Input Validation (CommandValidator)                 │    │
│  │     - Validate all parameters                           │    │
│  │     - Check rate limits                                 │    │
│  │     - Filter content (if message)                       │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │  3. Approval Check (ApprovalWorkflow)                   │    │
│  │     - Does this require human approval?                 │    │
│  │     - Create pending action if needed                   │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │  4. Execute (Platform-Specific Executor)                │    │
│  │     - Discord, Slack, etc.                              │    │
│  │     - Capture undo data                                 │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │  5. Audit Logging (BotAuditLogger)                      │    │
│  │     - Log operation with full context                   │    │
│  │     - Store undo data                                   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### New Crates

```
crates/
├── botticelli_security/          # Core security framework (NEW)
│   ├── permissions.rs            # Permission model
│   ├── validation.rs             # Input validation
│   ├── rate_limit.rs             # Rate limiting
│   ├── content_filter.rs         # Content filtering
│   ├── approval.rs               # Approval workflows
│   └── audit.rs                  # Audit logging
│
├── botticelli_social/            # Social platform integrations (EXISTING)
│   ├── discord/
│   │   ├── commands.rs           # Discord command executor (EXISTING)
│   │   └── write_commands.rs    # Write operations (NEW)
│   └── trait.rs                  # BotCommandExecutor trait (EXISTING)
│
└── botticelli_database/          # Database operations (EXISTING)
    └── audit_log.rs              # Audit log schema/repository (NEW)
```

## Implementation Plan

### Step 1: Permission Model (botticelli_security)

**Goal**: Define what operations each narrative can perform.

**Types**:

```rust
/// Permission for a specific bot command operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandPermission {
    /// Command name (e.g., "channels.send_message")
    pub command: String,
    
    /// Allowed on these specific resources (None = all resources)
    pub allowed_resources: Option<Vec<ResourceId>>,
    
    /// Forbidden on these specific resources (e.g., admin channels)
    pub forbidden_resources: Vec<ResourceId>,
    
    /// Rate limit for this operation
    pub rate_limit: RateLimit,
    
    /// Requires human approval before execution
    pub requires_approval: bool,
}

/// Resource identifier (channel, role, user, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceId {
    Channel(String),
    Role(String),
    User(String),
    Guild(String),
}

/// Permissions for a narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativePermissions {
    /// Narrative identifier
    pub narrative_id: String,
    
    /// Permissions by command
    pub commands: HashMap<String, CommandPermission>,
    
    /// Protected users (cannot be banned, kicked, etc.)
    pub protected_users: HashSet<String>,
    
    /// Protected roles (cannot be deleted, modified)
    pub protected_roles: HashSet<String>,
    
    /// Maximum operations per time window (global limit)
    pub global_rate_limit: RateLimit,
}

/// Rate limit specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum requests in the time window
    pub max_requests: u32,
    
    /// Time window in seconds
    pub window_secs: u64,
    
    /// Burst allowance (extra requests before throttling)
    pub burst: u32,
}

impl NarrativePermissions {
    /// Check if narrative has permission for a command
    pub fn check_permission(
        &self,
        command: &str,
        target: &ResourceId,
    ) -> Result<(), PermissionError> {
        // Implementation details
    }
    
    /// Load permissions from TOML configuration
    pub fn from_toml(path: &Path) -> Result<Self, PermissionError> {
        // Implementation details
    }
}
```

**TOML Configuration**:

```toml
# permissions/welcome_bot.toml
narrative_id = "welcome_bot"

# Protected resources
protected_users = ["566254598000476160"]  # Server owner
protected_roles = ["admin", "moderator"]

# Global rate limit: 100 operations per hour
[global_rate_limit]
max_requests = 100
window_secs = 3600
burst = 10

# Permission for sending messages
[commands."channels.send_message"]
allowed_resources = [
    { Channel = "welcome" },
    { Channel = "announcements" }
]
forbidden_resources = [
    { Channel = "admin" },
    { Channel = "mod-only" }
]
requires_approval = false

[commands."channels.send_message".rate_limit]
max_requests = 10
window_secs = 60
burst = 2

# Permission for creating channels (requires approval)
[commands."channels.create"]
requires_approval = true

[commands."channels.create".rate_limit]
max_requests = 1
window_secs = 3600
burst = 0
```

**Tests**:
- Unit tests for permission checking logic
- Test loading from TOML
- Test rate limit calculations
- Test resource matching (wildcards, patterns)

---

### Step 2: Input Validation (botticelli_security)

**Goal**: Validate all parameters before execution.

**Validator Trait**:

```rust
/// Validation result
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Command validator
pub trait CommandValidator: Send + Sync {
    /// Validate command parameters
    fn validate(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> ValidationResult<()>;
}

/// Discord-specific validator
pub struct DiscordCommandValidator {
    /// Known guilds (for validation)
    guilds: Arc<RwLock<HashSet<String>>>,
    
    /// Known channels
    channels: Arc<RwLock<HashMap<String, ChannelInfo>>>,
    
    /// Known roles
    roles: Arc<RwLock<HashMap<String, RoleInfo>>>,
    
    /// Content filter
    content_filter: Arc<ContentFilter>,
}

impl CommandValidator for DiscordCommandValidator {
    fn validate(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> ValidationResult<()> {
        match command {
            "channels.send_message" => {
                self.validate_send_message(args)
            }
            "members.ban" => {
                self.validate_ban(args)
            }
            "channels.create" => {
                self.validate_create_channel(args)
            }
            _ => Ok(()),
        }
    }
}

impl DiscordCommandValidator {
    fn validate_send_message(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> ValidationResult<()> {
        // Extract channel_id
        let channel_id = args.get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingParameter("channel_id"))?;
        
        // Check channel exists
        let channels = self.channels.read().unwrap();
        if !channels.contains_key(channel_id) {
            return Err(ValidationError::InvalidChannel(channel_id.to_string()));
        }
        
        // Extract content
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingParameter("content"))?;
        
        // Validate content
        self.content_filter.validate(content)?;
        
        Ok(())
    }
    
    fn validate_ban(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> ValidationResult<()> {
        // Extract user_id
        let user_id = args.get("user_id")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingParameter("user_id"))?;
        
        // Note: We cannot validate user exists without making API call
        // That's intentional - validation is lightweight, execution verifies
        
        // Validate reason (if provided)
        if let Some(reason) = args.get("reason").and_then(|v| v.as_str()) {
            if reason.len() > 512 {
                return Err(ValidationError::ReasonTooLong);
            }
        }
        
        Ok(())
    }
}
```

**Validation Errors**:

```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum ValidationError {
    #[display("Missing required parameter: {}", _0)]
    MissingParameter(&'static str),
    
    #[display("Invalid channel: {}", _0)]
    InvalidChannel(String),
    
    #[display("Invalid user: {}", _0)]
    InvalidUser(String),
    
    #[display("Invalid role: {}", _0)]
    InvalidRole(String),
    
    #[display("Content validation failed: {}", _0)]
    ContentValidation(String),
    
    #[display("Reason too long (max 512 characters)")]
    ReasonTooLong,
    
    #[display("Rate limit exceeded for {}: {} requests in {} seconds", command, count, window)]
    RateLimitExceeded {
        command: String,
        count: u32,
        window: u64,
    },
}
```

---

### Step 3: Content Filtering (botticelli_security)

**Goal**: Filter AI-generated content before sending.

**Content Filter**:

```rust
/// Content filter for message validation
pub struct ContentFilter {
    /// Maximum message length
    max_length: usize,
    
    /// Prohibited patterns (regex)
    prohibited_patterns: Vec<Regex>,
    
    /// Maximum mentions per message
    max_mentions: usize,
    
    /// Maximum links per message
    max_links: usize,
    
    /// URL allowlist (None = all allowed)
    url_allowlist: Option<HashSet<String>>,
}

impl ContentFilter {
    /// Create default filter for Discord
    pub fn discord_default() -> Self {
        let prohibited_patterns = vec![
            // No @everyone/@here spam
            Regex::new(r"@(everyone|here)").unwrap(),
            
            // No excessive caps
            Regex::new(r"[A-Z\s]{50,}").unwrap(),
            
            // No invite links (unless explicitly allowed)
            Regex::new(r"discord\.gg/\w+").unwrap(),
        ];
        
        Self {
            max_length: 2000,  // Discord limit
            prohibited_patterns,
            max_mentions: 5,
            max_links: 3,
            url_allowlist: None,
        }
    }
    
    /// Validate message content
    pub fn validate(&self, content: &str) -> Result<(), ContentError> {
        // Check length
        if content.len() > self.max_length {
            return Err(ContentError::TooLong {
                length: content.len(),
                max: self.max_length,
            });
        }
        
        // Check prohibited patterns
        for pattern in &self.prohibited_patterns {
            if pattern.is_match(content) {
                return Err(ContentError::ProhibitedPattern(
                    pattern.to_string()
                ));
            }
        }
        
        // Count mentions
        let mention_count = content.matches("<@").count();
        if mention_count > self.max_mentions {
            return Err(ContentError::TooManyMentions {
                count: mention_count,
                max: self.max_mentions,
            });
        }
        
        // Extract and validate URLs
        let urls = self.extract_urls(content);
        if urls.len() > self.max_links {
            return Err(ContentError::TooManyLinks {
                count: urls.len(),
                max: self.max_links,
            });
        }
        
        // Validate URLs against allowlist
        if let Some(allowlist) = &self.url_allowlist {
            for url in urls {
                let domain = url.domain()
                    .ok_or_else(|| ContentError::InvalidUrl(url.to_string()))?;
                
                if !allowlist.contains(domain) {
                    return Err(ContentError::UntrustedDomain(domain.to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    fn extract_urls(&self, content: &str) -> Vec<Url> {
        // Simple URL extraction (use proper parser in production)
        content.split_whitespace()
            .filter_map(|word| Url::parse(word).ok())
            .collect()
    }
}
```

**Content Errors**:

```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum ContentError {
    #[display("Content too long: {} characters (max {})", length, max)]
    TooLong { length: usize, max: usize },
    
    #[display("Prohibited pattern detected: {}", _0)]
    ProhibitedPattern(String),
    
    #[display("Too many mentions: {} (max {})", count, max)]
    TooManyMentions { count: usize, max: usize },
    
    #[display("Too many links: {} (max {})", count, max)]
    TooManyLinks { count: usize, max: usize },
    
    #[display("Invalid URL: {}", _0)]
    InvalidUrl(String),
    
    #[display("Untrusted domain: {}", _0)]
    UntrustedDomain(String),
}
```

---

### Step 4: Rate Limiting (botticelli_security)

**Goal**: Prevent abuse through rate limits.

**Rate Limiter**:

```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Rate limiter state
pub struct RateLimiter {
    /// Tracked operations by key
    operations: Arc<Mutex<HashMap<String, OperationHistory>>>,
}

/// Operation history for rate limiting
#[derive(Debug, Clone)]
struct OperationHistory {
    /// Timestamps of recent operations
    timestamps: Vec<Instant>,
    
    /// Last cleanup time
    last_cleanup: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Check if operation is allowed under rate limit
    pub fn check_limit(
        &self,
        key: &str,
        limit: &RateLimit,
    ) -> Result<(), RateLimitError> {
        let mut operations = self.operations.lock().unwrap();
        let now = Instant::now();
        
        // Get or create history
        let history = operations.entry(key.to_string())
            .or_insert_with(|| OperationHistory {
                timestamps: Vec::new(),
                last_cleanup: now,
            });
        
        // Clean up old timestamps
        let window = Duration::from_secs(limit.window_secs);
        history.timestamps.retain(|&ts| now.duration_since(ts) < window);
        
        // Check limit
        let count = history.timestamps.len() as u32;
        if count >= limit.max_requests + limit.burst {
            return Err(RateLimitError::LimitExceeded {
                key: key.to_string(),
                count,
                limit: limit.max_requests,
                window_secs: limit.window_secs,
            });
        }
        
        // Record this operation
        history.timestamps.push(now);
        
        Ok(())
    }
    
    /// Generate rate limit key for command
    pub fn make_key(narrative_id: &str, command: &str) -> String {
        format!("{}:{}", narrative_id, command)
    }
}
```

---

### Step 5: Approval Workflows (botticelli_security)

**Goal**: Require human approval for dangerous operations.

**Approval System**:

```rust
/// Pending action awaiting approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub narrative_id: String,
    pub command: String,
    pub args: HashMap<String, JsonValue>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: ApprovalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

/// Approval workflow manager
pub trait ApprovalWorkflow: Send + Sync {
    /// Create a pending action
    async fn create_pending(
        &self,
        narrative_id: &str,
        command: &str,
        args: HashMap<String, JsonValue>,
    ) -> Result<PendingAction, ApprovalError>;
    
    /// Approve a pending action
    async fn approve(
        &self,
        action_id: &str,
        approver: &str,
    ) -> Result<(), ApprovalError>;
    
    /// Reject a pending action
    async fn reject(
        &self,
        action_id: &str,
        approver: &str,
        reason: &str,
    ) -> Result<(), ApprovalError>;
    
    /// Get pending action by ID
    async fn get_pending(
        &self,
        action_id: &str,
    ) -> Result<PendingAction, ApprovalError>;
    
    /// List pending actions for narrative
    async fn list_pending(
        &self,
        narrative_id: &str,
    ) -> Result<Vec<PendingAction>, ApprovalError>;
}

/// Database-backed approval workflow
pub struct DatabaseApprovalWorkflow {
    database: Arc<PgConnection>,
}

impl DatabaseApprovalWorkflow {
    pub fn new(database: Arc<PgConnection>) -> Self {
        Self { database }
    }
}

#[async_trait::async_trait]
impl ApprovalWorkflow for DatabaseApprovalWorkflow {
    async fn create_pending(
        &self,
        narrative_id: &str,
        command: &str,
        args: HashMap<String, JsonValue>,
    ) -> Result<PendingAction, ApprovalError> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::hours(24);
        
        let action = PendingAction {
            id: id.clone(),
            narrative_id: narrative_id.to_string(),
            command: command.to_string(),
            args,
            created_at,
            expires_at,
            status: ApprovalStatus::Pending,
        };
        
        // Store in database
        // INSERT INTO pending_actions ...
        
        Ok(action)
    }
    
    // ... implement other methods
}
```

---

### Step 6: Audit Logging (botticelli_security + botticelli_database)

**Goal**: Log all write operations with full context.

**Audit Log Entry**:

```rust
/// Audit log entry for bot command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotAuditLogEntry {
    /// Unique ID
    pub id: String,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Narrative that executed the command
    pub narrative_id: String,
    
    /// Step name within narrative
    pub step_name: Option<String>,
    
    /// Command executed
    pub command: String,
    
    /// Command arguments
    pub args: HashMap<String, JsonValue>,
    
    /// Platform (discord, slack, etc.)
    pub platform: String,
    
    /// Target resource (channel, user, role, etc.)
    pub target_type: String,
    pub target_id: String,
    
    /// Guild/workspace ID
    pub guild_id: Option<String>,
    
    /// Who triggered the narrative (if applicable)
    pub executor_user: Option<String>,
    
    /// AI context (prompt + response)
    pub ai_prompt: Option<String>,
    pub ai_response: Option<String>,
    
    /// Result
    pub success: bool,
    pub error: Option<String>,
    
    /// Undo data (if applicable)
    pub undo_data: Option<JsonValue>,
    pub undo_expiry: Option<DateTime<Utc>>,
}

/// Audit logger trait
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log a bot command execution
    async fn log_execution(
        &self,
        entry: BotAuditLogEntry,
    ) -> Result<(), AuditError>;
    
    /// Query audit logs
    async fn query_logs(
        &self,
        filter: AuditLogFilter,
    ) -> Result<Vec<BotAuditLogEntry>, AuditError>;
}

/// Audit log filter
#[derive(Debug, Clone, Default)]
pub struct AuditLogFilter {
    pub narrative_id: Option<String>,
    pub command: Option<String>,
    pub platform: Option<String>,
    pub guild_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub success: Option<bool>,
}
```

**Database Schema**:

```sql
-- migrations/XXXX_create_audit_logs.up.sql
CREATE TABLE bot_audit_logs (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    narrative_id TEXT NOT NULL,
    step_name TEXT,
    command TEXT NOT NULL,
    args JSONB NOT NULL,
    platform TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    guild_id TEXT,
    executor_user TEXT,
    ai_prompt TEXT,
    ai_response TEXT,
    success BOOLEAN NOT NULL,
    error TEXT,
    undo_data JSONB,
    undo_expiry TIMESTAMPTZ,
    
    -- Indexes for querying
    INDEX idx_narrative_id (narrative_id),
    INDEX idx_command (command),
    INDEX idx_platform (platform),
    INDEX idx_guild_id (guild_id),
    INDEX idx_timestamp (timestamp),
    INDEX idx_success (success)
);

CREATE TABLE pending_actions (
    id TEXT PRIMARY KEY,
    narrative_id TEXT NOT NULL,
    command TEXT NOT NULL,
    args JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    approver TEXT,
    approval_time TIMESTAMPTZ,
    rejection_reason TEXT,
    
    INDEX idx_narrative_id (narrative_id),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at)
);
```

---

### Step 7: Secure Command Executor (botticelli_social)

**Goal**: Wrap platform executors with security layer.

**Secure Executor**:

```rust
/// Secure wrapper around platform-specific bot command executor
pub struct SecureBotCommandExecutor<E: BotCommandExecutor> {
    /// Underlying executor
    inner: E,
    
    /// Permission checker
    permissions: Arc<NarrativePermissions>,
    
    /// Input validator
    validator: Arc<dyn CommandValidator>,
    
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    
    /// Content filter
    content_filter: Arc<ContentFilter>,
    
    /// Approval workflow
    approval: Arc<dyn ApprovalWorkflow>,
    
    /// Audit logger
    audit_logger: Arc<dyn AuditLogger>,
}

#[async_trait::async_trait]
impl<E: BotCommandExecutor> BotCommandExecutor for SecureBotCommandExecutor<E> {
    fn platform(&self) -> &str {
        self.inner.platform()
    }
    
    fn supports_command(&self, command: &str) -> bool {
        self.inner.supports_command(command)
    }
    
    fn supported_commands(&self) -> Vec<String> {
        self.inner.supported_commands()
    }
    
    fn command_help(&self, command: &str) -> Option<String> {
        self.inner.command_help(command)
    }
    
    #[instrument(skip(self, args), fields(command))]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, BotCommandError> {
        info!("Executing secure command: {}", command);
        
        // 1. Check permissions
        let target = self.extract_target(command, args)?;
        self.permissions.check_permission(command, &target)
            .map_err(|e| BotCommandError::permission_denied(command, e.to_string()))?;
        
        // 2. Validate input
        self.validator.validate(command, args)
            .map_err(|e| BotCommandError::validation_failed(command, e.to_string()))?;
        
        // 3. Check rate limits
        let rate_key = RateLimiter::make_key(&self.permissions.narrative_id, command);
        let rate_limit = self.permissions.commands.get(command)
            .map(|p| p.rate_limit)
            .unwrap_or(self.permissions.global_rate_limit);
        
        self.rate_limiter.check_limit(&rate_key, &rate_limit)
            .map_err(|e| BotCommandError::rate_limit_exceeded(command, e.to_string()))?;
        
        // 4. Check if approval required
        if let Some(perm) = self.permissions.commands.get(command) {
            if perm.requires_approval {
                let pending = self.approval.create_pending(
                    &self.permissions.narrative_id,
                    command,
                    args.clone(),
                ).await
                    .map_err(|e| BotCommandError::approval_failed(command, e.to_string()))?;
                
                return Ok(json!({
                    "status": "pending_approval",
                    "action_id": pending.id,
                    "expires_at": pending.expires_at,
                }));
            }
        }
        
        // 5. Execute command
        let start = Instant::now();
        let result = self.inner.execute(command, args).await;
        let duration = start.elapsed();
        
        // 6. Audit log
        let entry = BotAuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            narrative_id: self.permissions.narrative_id.clone(),
            step_name: None,  // TODO: Pass from context
            command: command.to_string(),
            args: args.clone(),
            platform: self.platform().to_string(),
            target_type: format!("{:?}", target),
            target_id: self.extract_target_id(&target),
            guild_id: args.get("guild_id").and_then(|v| v.as_str()).map(String::from),
            executor_user: None,  // TODO: Pass from context
            ai_prompt: None,      // TODO: Pass from context
            ai_response: None,    // TODO: Pass from context
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
            undo_data: None,      // TODO: Capture undo data
            undo_expiry: None,
        };
        
        if let Err(e) = self.audit_logger.log_execution(entry).await {
            error!("Failed to log audit entry: {}", e);
            // Don't fail the operation due to logging error
        }
        
        // Log performance
        debug!(
            command,
            duration_ms = duration.as_millis(),
            success = result.is_ok(),
            "Command executed"
        );
        
        result
    }
}
```

---

### Step 8: Write Commands Implementation (botticelli_social)

**Goal**: Implement safe write operations for Discord.

**First Write Command: channels.send_message**

```rust
// crates/botticelli_social/src/discord/write_commands.rs

impl DiscordCommandExecutor {
    /// Send a message to a channel
    #[instrument(skip(self, args), fields(command = "channels.send_message"))]
    async fn channels_send_message(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Executing channels.send_message command");
        
        // Extract channel_id
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::missing_argument("channels.send_message", "channel_id")
            })?;
        
        let channel_id: ChannelId = channel_id_str.parse()
            .map_err(|e| BotCommandError::invalid_argument(
                "channels.send_message",
                "channel_id",
                format!("Invalid channel ID: {}", e),
            ))?;
        
        // Extract content
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::missing_argument("channels.send_message", "content")
            })?;
        
        // Note: Content validation happens in SecureBotCommandExecutor
        // We trust that it's already been validated
        
        // Send message
        let message = self.http.send_message(channel_id, content).await
            .map_err(|e| BotCommandError::api_error(
                "channels.send_message",
                format!("Failed to send message: {}", e),
            ))?;
        
        debug!(
            message_id = %message.id,
            channel_id = %channel_id,
            "Message sent successfully"
        );
        
        // Return message details
        Ok(serde_json::json!({
            "message_id": message.id.to_string(),
            "channel_id": channel_id.to_string(),
            "content": message.content,
            "timestamp": message.timestamp.to_string(),
        }))
    }
}
```

**Add to execute() method**:

```rust
impl BotCommandExecutor for DiscordCommandExecutor {
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        match command {
            // ... existing read commands ...
            
            // Write commands (NEW)
            "channels.send_message" => self.channels_send_message(args).await?,
            
            _ => {
                error!(command, "Unsupported Discord command");
                return Err(BotCommandError::unsupported_command(command));
            }
        }
    }
}
```

---

## Implementation Roadmap

### Week 1: Core Security Framework

**Day 1-2: Permission Model**
- [ ] Implement `NarrativePermissions` type
- [ ] Implement `CommandPermission` type
- [ ] Implement TOML loading
- [ ] Unit tests for permission checking
- [ ] Documentation

**Day 3-4: Validation & Content Filtering**
- [ ] Implement `CommandValidator` trait
- [ ] Implement `DiscordCommandValidator`
- [ ] Implement `ContentFilter`
- [ ] Unit tests for validation
- [ ] Unit tests for content filtering

**Day 5: Rate Limiting**
- [ ] Implement `RateLimiter`
- [ ] Unit tests for rate limiting
- [ ] Integration tests with time simulation

### Week 2: Approval & Audit Infrastructure

**Day 1-2: Approval Workflows**
- [ ] Implement `ApprovalWorkflow` trait
- [ ] Implement `DatabaseApprovalWorkflow`
- [ ] Create database schema migration
- [ ] Unit tests for approval logic
- [ ] Integration tests with PostgreSQL

**Day 3-4: Audit Logging**
- [ ] Implement `AuditLogger` trait
- [ ] Implement `DatabaseAuditLogger`
- [ ] Create database schema migration
- [ ] Unit tests for audit logging
- [ ] Integration tests with PostgreSQL

**Day 5: Secure Executor Wrapper**
- [ ] Implement `SecureBotCommandExecutor`
- [ ] Integration tests with mock executor
- [ ] End-to-end tests with full stack

### Week 3: Write Commands Implementation

**Day 1-2: Message Commands**
- [ ] Implement `channels.send_message`
- [ ] Implement `messages.delete`
- [ ] Integration tests with Discord API
- [ ] Permission configuration examples

**Day 3-4: Moderation Commands**
- [ ] Implement `members.kick` (approval required)
- [ ] Implement `members.ban` (approval required)
- [ ] Integration tests with approval workflow
- [ ] Undo mechanism for kick/ban

**Day 5: Channel Management**
- [ ] Implement `channels.create` (approval required)
- [ ] Implement `channels.update`
- [ ] Integration tests
- [ ] Documentation and examples

### Week 4: Testing, Documentation, Deployment

**Day 1-2: Comprehensive Testing**
- [ ] Security penetration tests
- [ ] Load testing (rate limits)
- [ ] Approval workflow testing
- [ ] Audit log verification

**Day 3-4: Documentation**
- [ ] Security framework guide
- [ ] Permission configuration guide
- [ ] Write command reference
- [ ] Best practices document

**Day 5: Deployment & Monitoring**
- [ ] Deploy to staging environment
- [ ] Set up audit log monitoring
- [ ] Configure alerts for rate limit violations
- [ ] Canary deployment to 1% of narratives

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permission_check_allowed() {
        let perms = NarrativePermissions {
            narrative_id: "test".to_string(),
            commands: hashmap! {
                "channels.send_message".to_string() => CommandPermission {
                    command: "channels.send_message".to_string(),
                    allowed_resources: Some(vec![
                        ResourceId::Channel("123".to_string())
                    ]),
                    forbidden_resources: vec![],
                    rate_limit: RateLimit {
                        max_requests: 10,
                        window_secs: 60,
                        burst: 2,
                    },
                    requires_approval: false,
                },
            },
            protected_users: HashSet::new(),
            protected_roles: HashSet::new(),
            global_rate_limit: RateLimit {
                max_requests: 100,
                window_secs: 3600,
                burst: 10,
            },
        };
        
        let target = ResourceId::Channel("123".to_string());
        assert!(perms.check_permission("channels.send_message", &target).is_ok());
    }
    
    #[test]
    fn test_permission_check_forbidden() {
        // Test forbidden resource
    }
    
    #[test]
    fn test_rate_limit_enforcement() {
        let limiter = RateLimiter::new();
        let limit = RateLimit {
            max_requests: 2,
            window_secs: 60,
            burst: 0,
        };
        
        // First two should succeed
        assert!(limiter.check_limit("test", &limit).is_ok());
        assert!(limiter.check_limit("test", &limit).is_ok());
        
        // Third should fail
        assert!(limiter.check_limit("test", &limit).is_err());
    }
    
    #[test]
    fn test_content_filter_too_long() {
        let filter = ContentFilter::discord_default();
        let long_content = "a".repeat(3000);
        
        let result = filter.validate(&long_content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ContentError::TooLong { .. }));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_secure_send_message_with_permissions() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();
    let channel_id = get_test_channel_id();
    
    // Create permission config
    let permissions = NarrativePermissions {
        narrative_id: "test_narrative".to_string(),
        commands: hashmap! {
            "channels.send_message".to_string() => CommandPermission {
                command: "channels.send_message".to_string(),
                allowed_resources: Some(vec![
                    ResourceId::Channel(channel_id.clone())
                ]),
                forbidden_resources: vec![],
                rate_limit: RateLimit {
                    max_requests: 5,
                    window_secs: 60,
                    burst: 1,
                },
                requires_approval: false,
            },
        },
        protected_users: HashSet::new(),
        protected_roles: HashSet::new(),
        global_rate_limit: RateLimit::default(),
    };
    
    // Create secure executor
    let inner = DiscordCommandExecutor::new(&token);
    let validator = Arc::new(DiscordCommandValidator::new());
    let rate_limiter = Arc::new(RateLimiter::new());
    let content_filter = Arc::new(ContentFilter::discord_default());
    let approval = Arc::new(InMemoryApprovalWorkflow::new());
    let audit_logger = Arc::new(InMemoryAuditLogger::new());
    
    let executor = SecureBotCommandExecutor {
        inner,
        permissions: Arc::new(permissions),
        validator,
        rate_limiter,
        content_filter,
        approval,
        audit_logger: audit_logger.clone(),
    };
    
    // Execute command
    let mut args = HashMap::new();
    args.insert("channel_id".to_string(), json!(channel_id));
    args.insert("content".to_string(), json!("Test message from secure executor"));
    
    let result = executor.execute("channels.send_message", &args).await;
    assert!(result.is_ok());
    
    // Verify audit log
    let logs = audit_logger.get_all_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].command, "channels.send_message");
    assert!(logs[0].success);
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_secure_send_message_rate_limit() {
    // Test that rate limit is enforced
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_secure_ban_requires_approval() {
    // Test that ban command creates pending action
}
```

---

## Security Considerations

### Threat Model

**Threats Addressed**:
1. ✅ AI hallucination → Validation layer + approval workflow
2. ✅ Prompt injection → Content filtering + protected users
3. ✅ Privilege escalation → Permission model + resource restrictions
4. ✅ Data loss → Audit logging + undo mechanisms
5. ✅ Spam/abuse → Rate limiting
6. ✅ Insider threat → Audit trail + approval requirements

**Threats NOT Addressed** (future work):
- Compromised API keys (need key rotation, vault storage)
- DDoS attacks (need infrastructure-level protection)
- Advanced prompt injection (need more sophisticated content analysis)

### Security Audit Checklist

Before deploying to production:
- [ ] All write operations have permission checks
- [ ] All inputs are validated
- [ ] All dangerous operations require approval
- [ ] All operations are rate limited
- [ ] All operations are audit logged
- [ ] Content filtering blocks known attack patterns
- [ ] Protected users cannot be targeted
- [ ] Undo mechanisms are tested and working
- [ ] Database credentials are stored securely
- [ ] Audit logs are backed up regularly

---

## Success Metrics

### Technical Metrics

- **Permission Checks**: 100% coverage on all write operations
- **Audit Logs**: 100% capture rate for all write operations
- **Rate Limit Violations**: < 1% of legitimate operations
- **Approval Turnaround**: < 5 minutes median approval time
- **Undo Success Rate**: > 95% of undo operations succeed

### Security Metrics

- **False Positives**: < 5% of blocked operations are legitimate
- **False Negatives**: 0 malicious operations slip through (aspirational)
- **Incident Response Time**: < 15 minutes from detection to mitigation
- **Audit Query Performance**: < 1 second for 90% of queries

### User Experience Metrics

- **Command Latency**: < 500ms p99 (including security checks)
- **Approval Clarity**: 100% of pending actions have clear explanations
- **Error Messages**: 100% of errors provide actionable guidance

---

## Future Enhancements

### Phase 3.5: Advanced Features

1. **Machine Learning Content Filter**
   - Train model on Discord ToS violations
   - Detect toxic content, hate speech, spam patterns
   - Adaptive learning from moderation feedback

2. **Dynamic Risk Scoring**
   - Assign risk scores to operations based on context
   - Lower trust = more restrictions
   - Build trust over time with successful operations

3. **Multi-Approver Workflows**
   - Require 2+ approvals for high-risk operations
   - Different approval chains for different risk levels
   - Escalation to senior moderators

4. **Undo/Rollback System**
   - 30-day undo window for all operations
   - Snapshot/restore for channels
   - Ban/kick reversal with notification

5. **Federated Permissions**
   - Share permission configs across servers
   - Community-maintained permission templates
   - Permission inheritance hierarchies

### Phase 4: Multi-Platform Security

- Extend security framework to Slack, Telegram, Matrix
- Platform-specific validators and filters
- Unified audit log across all platforms
- Cross-platform threat intelligence sharing

---

## Conclusion

This security framework provides a **solid foundation** for enabling agentic bots to perform write operations safely. By implementing:

1. **Granular permissions** - Control what each narrative can do
2. **Multi-layer validation** - Catch errors before execution
3. **Human-in-the-loop** - Require approval for dangerous operations
4. **Comprehensive auditing** - Track everything for accountability
5. **Rate limiting** - Prevent abuse and loops
6. **Content filtering** - Block harmful content

We can responsibly expand from read-only to read-write bot operations, unlocking powerful new use cases while maintaining security, compliance, and user trust.

**Estimated Total Effort**: 3-4 weeks for core framework + 2-3 weeks per platform for write command implementation.

**Recommended First Deployment**: `channels.send_message` with strict content filtering and rate limiting, monitored closely for 2 weeks before expanding to moderation commands.
