<!-- markdownlint-disable MD046 -->
# Narrative Act Processors Implementation

## Implementation Status

| Step | Component | Status | Files |
|------|-----------|--------|-------|
| 1 | JSON/TOML Extraction | ✅ Complete | `src/narrative/extraction.rs` |
| 2 | ActProcessor Trait | ✅ Complete | `src/narrative/processor.rs` |
| 3 | Enhanced Executor | 🚧 Pending | `src/narrative/executor.rs` (updated) |
| 4 | Discord JSON Models | 🚧 Pending | `src/discord/json_models.rs` |
| 5 | Discord Conversions | 🚧 Pending | `src/discord/conversions.rs` |
| 6 | Discord Processors | 🚧 Pending | `src/discord/processors.rs` |
| 7 | Module Exports | 🚧 Pending | `src/lib.rs`, `src/discord/mod.rs` |
| 8 | Tests | 🚧 Pending | `tests/` directory |

## Overview

This document describes the implementation of a post-processing pipeline for narrative act executions. The system extracts structured data (JSON, TOML) from LLM responses and automatically inserts it into the database.

**Note:** This document serves as both a planning document and implementation guide. Sections marked ✅ have been implemented and tested. Implementation details reflect the actual code in the repository.

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Narrative Execution                       │
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Act 1      │ -> │   Act 2      │ -> │   Act 3      │  │
│  │  "Generate   │    │  "Critique"  │    │  "Output     │  │
│  │   Draft"     │    │              │    │   JSON"      │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│         │                    │                    │         │
└─────────┼────────────────────┼────────────────────┼─────────┘
          │                    │                    │
          v                    v                    v
    ┌─────────────────────────────────────────────────────┐
    │           Act Processor Pipeline                    │
    │                                                     │
    │  1. Extract JSON/TOML from response                │
    │  2. Parse into typed structs                       │
    │  3. Validate data                                  │
    │  4. Insert into database                           │
    │  5. Handle errors gracefully                       │
    └─────────────────────────────────────────────────────┘
          │
          v
    ┌─────────────────┐
    │    Database     │
    │  ┌───────────┐  │
    │  │  Guilds   │  │
    │  │ Channels  │  │
    │  │  Roles    │  │
    │  │  Users    │  │
    │  └───────────┘  │
    └─────────────────┘
```

### Design Principles

1. **Extensible**: Easy to add processors for new data types
2. **Testable**: Each processor is independently testable
3. **Robust**: Handles malformed JSON, extraction failures gracefully
4. **Optional**: Processors are opt-in, don't affect basic narrative execution
5. **Composable**: Multiple processors can handle the same act

## File Structure

```
src/
├── narrative/
│   ├── mod.rs                    # Export processor types
│   ├── core.rs                   # Existing narrative structures
│   ├── executor.rs               # Enhanced with processor support
│   ├── extraction.rs             # NEW: JSON/TOML extraction utilities
│   └── processor.rs              # NEW: ActProcessor trait
│
├── discord/
│   ├── mod.rs
│   ├── processors.rs             # NEW: Discord-specific processors
│   ├── json_models.rs            # NEW: JSON deserialization models
│   └── conversions.rs            # NEW: JSON -> DB model conversions
│
└── database/
    └── discord_repository.rs     # Enhanced with processor helpers

tests/
├── narrative_extraction_test.rs  # NEW: Test extraction logic
├── discord_processors_test.rs    # NEW: Test Discord processors
└── narrative_executor_test.rs    # Updated: Test with processors
```

## Implementation Steps

### Step 1: JSON/TOML Extraction Utilities ✅

**Status:** Complete (commit: 9f2957f)

**Implementation:** `src/narrative/extraction.rs`

**What was built:**

```rust
//! Utilities for extracting structured data from LLM responses.

use crate::{BoticelliError, BoticelliResult};

/// Extract JSON from a response that may contain markdown or extra text.
///
/// This function tries multiple extraction strategies:
/// 1. Markdown code blocks: ```json ... ```
/// 2. Balanced braces: { ... }
/// 3. Balanced brackets: [ ... ]
///
/// # Errors
///
/// Returns an error if no valid JSON is found in the response.
pub fn extract_json(response: &str) -> BoticelliResult<String> {
    // Strategy 1: Extract from markdown code blocks
    if let Some(json) = extract_from_code_block(response, "json") {
        return Ok(json);
    }

    // Strategy 2: Extract balanced braces (objects)
    if let Some(json) = extract_balanced(response, '{', '}') {
        return Ok(json);
    }

    // Strategy 3: Extract balanced brackets (arrays)
    if let Some(json) = extract_balanced(response, '[', ']') {
        return Ok(json);
    }

    Err(BoticelliError::new(format!(
        "No JSON found in response (length: {})",
        response.len()
    )))
}

/// Extract TOML from a response that may contain markdown or extra text.
pub fn extract_toml(response: &str) -> BoticelliResult<String> {
    // Strategy 1: Extract from markdown code blocks
    if let Some(toml) = extract_from_code_block(response, "toml") {
        return Ok(toml);
    }

    // Strategy 2: Look for TOML section headers [...]
    if response.contains('[') && (response.contains(" = ") || response.contains('=')) {
        // Try to find first [ and last meaningful line
        if let Some(start) = response.find('[') {
            return Ok(response[start..].trim().to_string());
        }
    }

    Err(BoticelliError::new(format!(
        "No TOML found in response (length: {})",
        response.len()
    )))
}

/// Extract content from markdown code blocks.
///
/// Looks for patterns like:
/// - ```json ... ```
/// - ```toml ... ```
/// - ``` ... ``` (no language specified)
fn extract_from_code_block(response: &str, language: &str) -> Option<String> {
    // Pattern: ```language\n...\n```
    let pattern = format!("```{}", language);

    if let Some(start) = response.find(&pattern) {
        let content_start = start + pattern.len();
        if let Some(end) = response[content_start..].find("```") {
            let content = &response[content_start..content_start + end];
            return Some(content.trim().to_string());
        }
    }

    // Try without language specifier
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        // Skip to next newline (in case there's a language specifier)
        let skip_to = response[content_start..]
            .find('\n')
            .map(|n| content_start + n + 1)
            .unwrap_or(content_start);

        if let Some(end) = response[skip_to..].find("```") {
            let content = &response[skip_to..skip_to + end];
            return Some(content.trim().to_string());
        }
    }

    None
}

/// Extract content between balanced delimiters.
///
/// Finds the first occurrence of `open` and extracts content up to
/// the matching `close`, handling nesting correctly.
fn extract_balanced(response: &str, open: char, close: char) -> Option<String> {
    let start = response.find(open)?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in response[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => escape_next = true,
            '"' => in_string = !in_string,
            c if c == open && !in_string => depth += 1,
            c if c == close && !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(response[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Parse and validate JSON, returning a specific type.
pub fn parse_json<T>(json_str: &str) -> BoticelliResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(json_str).map_err(|e| {
        BoticelliError::new(format!(
            "Failed to parse JSON: {} (JSON: {}...)",
            e,
            &json_str[..json_str.len().min(100)]
        ))
    })
}

/// Parse and validate TOML, returning a specific type.
pub fn parse_toml<T>(toml_str: &str) -> BoticelliResult<T>
where
    T: serde::de::DeserializeOwned,
{
    toml::from_str(toml_str).map_err(|e| {
        BoticelliError::new(format!(
            "Failed to parse TOML: {} (TOML: {}...)",
            e,
            &toml_str[..toml_str.len().min(100)]
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let response = r#"
Here's the JSON you requested:

```json
{
  "id": 123,
  "name": "Test"
}
```

Hope this helps!
"#;
        let json = extract_json(response).unwrap();
        assert!(json.contains("\"id\": 123"));
    }

    #[test]
    fn test_extract_json_balanced_braces() {
        let response = r#"
Sure! Here it is: {"id": 456, "nested": {"value": "test"}}
"#;
        let json = extract_json(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_array() {
        let response = r#"
Here are the items:
[
  {"id": 1},
  {"id": 2}
]
"#;
        let json = extract_json(response).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn test_no_json_found() {
        let response = "This is just plain text with no JSON";
        assert!(extract_json(response).is_err());
    }
}
```

### Step 2: Act Processor Trait ✅

**Status:** Complete (commit: c448e71)

**Implementation:** `src/narrative/processor.rs`

**What was built:**

```rust
//! Act processing traits and registry.

use crate::{ActExecution, BoticelliResult};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for processing act execution results.
///
/// Processors are invoked after an act completes to extract structured
/// data and perform side effects (database insertion, file writing, etc.).
#[async_trait]
pub trait ActProcessor: Send + Sync {
    /// Process an act execution result.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails. The error should be descriptive
    /// and include context about what went wrong.
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()>;

    /// Check if this processor should handle the given act.
    ///
    /// Implementations can check act name, response content, metadata, etc.
    fn should_process(&self, act_name: &str, response: &str) -> bool;

    /// Return a human-readable name for this processor (for logging).
    fn name(&self) -> &str;
}

/// Registry of act processors with smart routing.
pub struct ProcessorRegistry {
    processors: Vec<Box<dyn ActProcessor>>,
}

impl ProcessorRegistry {
    /// Create a new empty processor registry.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Register a new processor.
    pub fn register(&mut self, processor: Box<dyn ActProcessor>) {
        self.processors.push(processor);
    }

    /// Process an act execution with all matching processors.
    ///
    /// Calls each processor that returns `true` from `should_process`.
    /// Continues processing even if some processors fail, collecting all errors.
    pub async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        let mut errors = Vec::new();

        for processor in &self.processors {
            if processor.should_process(&execution.act_name, &execution.response) {
                if let Err(e) = processor.process(execution).await {
                    tracing::warn!(
                        processor = processor.name(),
                        act = %execution.act_name,
                        error = %e,
                        "Processor failed"
                    );
                    errors.push(format!("{}: {}", processor.name(), e));
                } else {
                    tracing::debug!(
                        processor = processor.name(),
                        act = %execution.act_name,
                        "Processor succeeded"
                    );
                }
            }
        }

        if !errors.is_empty() {
            return Err(crate::BoticelliError::new(format!(
                "Processor errors: {}",
                errors.join("; ")
            )));
        }

        Ok(())
    }

    /// Get the number of registered processors.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Default for ProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### Step 3: Enhanced Narrative Executor

Update `src/narrative/executor.rs`:

```rust
// Add to existing imports
use crate::ProcessorRegistry;

// Add field to NarrativeExecutor
pub struct NarrativeExecutor<D: BoticelliDriver> {
    driver: D,
    processor_registry: Option<ProcessorRegistry>,
}

impl<D: BoticelliDriver> NarrativeExecutor<D> {
    /// Create a new narrative executor with the given LLM driver.
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            processor_registry: None,
        }
    }

    /// Create a new narrative executor with processors.
    pub fn with_processors(driver: D, registry: ProcessorRegistry) -> Self {
        Self {
            driver,
            processor_registry: Some(registry),
        }
    }

    /// Execute a narrative, processing all acts in sequence.
    ///
    /// If processors are registered, they will be invoked after each act
    /// completes to extract and process structured data.
    pub async fn execute<N: NarrativeProvider>(
        &self,
        narrative: &N,
    ) -> BoticelliResult<NarrativeExecution> {
        let mut act_executions = Vec::new();
        let mut conversation_history: Vec<Message> = Vec::new();

        for (sequence_number, act_name) in narrative.act_names().iter().enumerate() {
            let config = narrative
                .get_act_config(act_name)
                .expect("NarrativeProvider should ensure all acts exist");

            conversation_history.push(Message {
                role: Role::User,
                content: config.inputs.clone(),
            });

            let request = GenerateRequest {
                messages: conversation_history.clone(),
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                model: config.model.clone(),
            };

            let response = self.driver.generate(&request).await?;
            let response_text = extract_text_from_outputs(&response.outputs)?;

            let act_execution = ActExecution {
                act_name: act_name.clone(),
                inputs: config.inputs.clone(),
                model: config.model,
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                response: response_text.clone(),
                sequence_number,
            };

            // NEW: Process with registered processors
            if let Some(registry) = &self.processor_registry {
                tracing::info!(
                    act = %act_name,
                    processors = registry.len(),
                    "Processing act with registered processors"
                );

                if let Err(e) = registry.process(&act_execution).await {
                    tracing::error!(
                        act = %act_name,
                        error = %e,
                        "Act processing failed, continuing execution"
                    );
                    // Note: We don't fail the entire narrative on processor errors
                    // The user still gets the execution results
                }
            }

            act_executions.push(act_execution);

            conversation_history.push(Message {
                role: Role::Assistant,
                content: vec![Input::Text(response_text)],
            });
        }

        Ok(NarrativeExecution {
            narrative_name: narrative.name().to_string(),
            act_executions,
        })
    }

    /// Get a reference to the underlying LLM driver.
    pub fn driver(&self) -> &D {
        &self.driver
    }
}
```

### Step 4: Discord JSON Models

Create `src/discord/json_models.rs`:

```rust
//! JSON deserialization models for Discord data.
//!
//! These models match the JSON schemas defined in DISCORD_NARRATIVE.md
//! and are used to parse LLM-generated responses before inserting into
//! the database.

use serde::{Deserialize, Serialize};

/// JSON model for Discord guild data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuildJson {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,

    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub member_count: Option<i32>,
    #[serde(default)]
    pub verification_level: Option<i16>,
    #[serde(default)]
    pub premium_tier: Option<i16>,
    #[serde(default)]
    pub features: Option<Vec<String>>,
}

/// JSON model for Discord channel data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelJson {
    pub id: i64,
    pub channel_type: String,

    #[serde(default)]
    pub guild_id: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<i64>,
    #[serde(default)]
    pub nsfw: Option<bool>,
    #[serde(default)]
    pub rate_limit_per_user: Option<i32>,
    #[serde(default)]
    pub bitrate: Option<i32>,
    #[serde(default)]
    pub user_limit: Option<i32>,
}

/// JSON model for Discord user data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUserJson {
    pub id: i64,
    pub username: String,

    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: Option<bool>,
    #[serde(default)]
    pub premium_type: Option<i16>,
    #[serde(default)]
    pub locale: Option<String>,
}

/// JSON model for Discord role data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRoleJson {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    pub position: i32,
    pub permissions: i64,

    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub hoist: Option<bool>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub unicode_emoji: Option<String>,
    #[serde(default)]
    pub managed: Option<bool>,
    #[serde(default)]
    pub mentionable: Option<bool>,
}

/// JSON model for Discord guild member data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuildMemberJson {
    pub guild_id: i64,
    pub user_id: i64,
    pub joined_at: String, // ISO 8601 timestamp

    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub premium_since: Option<String>,
    #[serde(default)]
    pub deaf: Option<bool>,
    #[serde(default)]
    pub mute: Option<bool>,
    #[serde(default)]
    pub pending: Option<bool>,
}

/// JSON model for Discord member role assignment data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMemberRoleJson {
    pub guild_id: i64,
    pub user_id: i64,
    pub role_id: i64,
    pub assigned_at: String, // ISO 8601 timestamp

    #[serde(default)]
    pub assigned_by: Option<i64>,
}
```

### Step 5: JSON to Database Conversions

Create `src/discord/conversions.rs`:

```rust
//! Conversions between JSON models and database models.

use crate::{
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRoleJson, DiscordUserJson,
};
use chrono::NaiveDateTime;

// Note: You'll need to create these "New*Row" types for insertable models
// They should match your Diesel schema

/// Convert JSON guild to insertable database row.
pub fn guild_json_to_row(json: DiscordGuildJson) -> NewDiscordGuildRow {
    NewDiscordGuildRow {
        id: json.id,
        name: json.name,
        owner_id: json.owner_id,
        icon: json.icon,
        banner: json.banner,
        description: json.description,
        member_count: json.member_count,
        verification_level: json.verification_level,
        premium_tier: json.premium_tier,
        features: json.features.map(|f| f.into_iter().map(Some).collect()),
        // Set defaults for fields not in JSON
        splash: None,
        vanity_url_code: None,
        approximate_member_count: None,
        approximate_presence_count: None,
        afk_channel_id: None,
        afk_timeout: None,
        system_channel_id: None,
        rules_channel_id: None,
        public_updates_channel_id: None,
        explicit_content_filter: None,
        mfa_level: None,
        premium_subscription_count: None,
        max_presences: None,
        max_members: None,
        max_video_channel_users: None,
        large: None,
        unavailable: None,
        joined_at: Some(chrono::Utc::now().naive_utc()),
        bot_permissions: None,
        bot_active: Some(true),
    }
}

/// Convert JSON channel to insertable database row.
pub fn channel_json_to_row(json: DiscordChannelJson) -> Result<NewDiscordChannelRow, String> {
    // Parse channel type enum
    let channel_type = parse_channel_type(&json.channel_type)?;

    Ok(NewDiscordChannelRow {
        id: json.id,
        guild_id: json.guild_id,
        name: json.name,
        channel_type,
        position: json.position,
        topic: json.topic,
        nsfw: json.nsfw,
        rate_limit_per_user: json.rate_limit_per_user,
        bitrate: json.bitrate,
        user_limit: json.user_limit,
        parent_id: json.parent_id,
        // Defaults
        owner_id: None,
        message_count: None,
        member_count: None,
        archived: None,
        auto_archive_duration: None,
        archive_timestamp: None,
        locked: None,
        invitable: None,
        available_tags: None,
        default_reaction_emoji: None,
        default_thread_rate_limit: None,
        default_sort_order: None,
        default_forum_layout: None,
        last_message_at: None,
        last_read_message_id: None,
        bot_has_access: Some(true),
    })
}

/// Parse channel type string to enum value.
fn parse_channel_type(type_str: &str) -> Result<DiscordChannelTypeEnum, String> {
    match type_str {
        "guild_text" => Ok(DiscordChannelTypeEnum::GuildText),
        "guild_voice" => Ok(DiscordChannelTypeEnum::GuildVoice),
        "guild_category" => Ok(DiscordChannelTypeEnum::GuildCategory),
        "guild_announcement" => Ok(DiscordChannelTypeEnum::GuildAnnouncement),
        "public_thread" => Ok(DiscordChannelTypeEnum::PublicThread),
        "private_thread" => Ok(DiscordChannelTypeEnum::PrivateThread),
        "guild_stage_voice" => Ok(DiscordChannelTypeEnum::GuildStageVoice),
        "guild_forum" => Ok(DiscordChannelTypeEnum::GuildForum),
        "guild_media" => Ok(DiscordChannelTypeEnum::GuildMedia),
        _ => Err(format!("Unknown channel type: {}", type_str)),
    }
}

/// Convert JSON user to insertable database row.
pub fn user_json_to_row(json: DiscordUserJson) -> NewDiscordUserRow {
    let now = chrono::Utc::now().naive_utc();

    NewDiscordUserRow {
        id: json.id,
        username: json.username,
        discriminator: json.discriminator,
        global_name: json.global_name,
        avatar: json.avatar,
        bot: json.bot,
        premium_type: json.premium_type,
        locale: json.locale,
        first_seen: now,
        last_seen: now,
        // Defaults
        banner: None,
        accent_color: None,
        system: None,
        mfa_enabled: None,
        verified: None,
        public_flags: None,
    }
}

/// Parse ISO 8601 timestamp string to NaiveDateTime.
pub fn parse_timestamp(timestamp: &str) -> Result<NaiveDateTime, String> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.naive_utc())
        .map_err(|e| format!("Invalid timestamp '{}': {}", timestamp, e))
}

/// Convert JSON guild member to insertable database row.
pub fn guild_member_json_to_row(
    json: DiscordGuildMemberJson,
) -> Result<NewDiscordGuildMemberRow, String> {
    Ok(NewDiscordGuildMemberRow {
        guild_id: json.guild_id,
        user_id: json.user_id,
        nick: json.nick,
        avatar: json.avatar,
        joined_at: parse_timestamp(&json.joined_at)?,
        premium_since: json
            .premium_since
            .as_ref()
            .map(|s| parse_timestamp(s))
            .transpose()?,
        deaf: json.deaf,
        mute: json.mute,
        pending: json.pending,
        // Defaults
        communication_disabled_until: None,
        left_at: None,
    })
}

/// Convert JSON role to insertable database row.
pub fn role_json_to_row(json: DiscordRoleJson) -> NewDiscordRoleRow {
    NewDiscordRoleRow {
        id: json.id,
        guild_id: json.guild_id,
        name: json.name,
        color: json.color.unwrap_or(0),
        hoist: json.hoist,
        icon: json.icon,
        unicode_emoji: json.unicode_emoji,
        position: json.position,
        permissions: json.permissions,
        managed: json.managed,
        mentionable: json.mentionable,
        tags: None,
    }
}

/// Convert JSON member role to insertable database row.
pub fn member_role_json_to_row(
    json: DiscordMemberRoleJson,
) -> Result<NewDiscordMemberRoleRow, String> {
    Ok(NewDiscordMemberRoleRow {
        guild_id: json.guild_id,
        user_id: json.user_id,
        role_id: json.role_id,
        assigned_at: parse_timestamp(&json.assigned_at)?,
        assigned_by: json.assigned_by,
    })
}
```

### Step 6: Discord Processors

Create `src/discord/processors.rs`:

```rust
//! Act processors for Discord data types.

use crate::{
    extract_json, parse_json, ActExecution, ActProcessor, BoticelliResult, DiscordChannelJson,
    DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson, DiscordRoleJson,
    DiscordUserJson,
};
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Processor for Discord guild data.
pub struct DiscordGuildProcessor {
    db_pool: DbPool,
}

impl DiscordGuildProcessor {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildProcessor {
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        // Extract and parse JSON
        let json_str = extract_json(&execution.response)?;
        let guild: DiscordGuildJson = parse_json(&json_str)?;

        tracing::info!(
            guild_id = guild.id,
            guild_name = %guild.name,
            "Inserting Discord guild"
        );

        // Convert to database row
        let row = crate::discord::guild_json_to_row(guild);

        // Insert into database
        let mut conn = self.db_pool.get().map_err(|e| {
            crate::BoticelliError::new(format!("Database connection failed: {}", e))
        })?;

        diesel::insert_into(crate::database::schema::discord_guilds::table)
            .values(&row)
            .on_conflict(crate::database::schema::discord_guilds::id)
            .do_update()
            .set(&row) // Update if already exists
            .execute(&mut conn)
            .map_err(|e| crate::BoticelliError::new(format!("Database insert failed: {}", e)))?;

        tracing::info!("Discord guild inserted successfully");
        Ok(())
    }

    fn should_process(&self, act_name: &str, response: &str) -> bool {
        // Process if act name suggests guild data
        let name_match = act_name.to_lowercase().contains("guild")
            || act_name.to_lowercase().contains("server");

        // Or if response looks like a single guild object
        let content_match = response.contains("\"owner_id\"") && !response.trim().starts_with('[');

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordGuildProcessor"
    }
}

/// Processor for Discord channel data (handles both single and array).
pub struct DiscordChannelProcessor {
    db_pool: DbPool,
}

impl DiscordChannelProcessor {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl ActProcessor for DiscordChannelProcessor {
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        let json_str = extract_json(&execution.response)?;

        // Try parsing as array first, then single object
        let channels: Vec<DiscordChannelJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(count = channels.len(), "Inserting Discord channels");

        let mut conn = self.db_pool.get().map_err(|e| {
            crate::BoticelliError::new(format!("Database connection failed: {}", e))
        })?;

        for channel in channels {
            let row = crate::discord::channel_json_to_row(channel)?;

            diesel::insert_into(crate::database::schema::discord_channels::table)
                .values(&row)
                .on_conflict(crate::database::schema::discord_channels::id)
                .do_update()
                .set(&row)
                .execute(&mut conn)
                .map_err(|e| {
                    crate::BoticelliError::new(format!("Database insert failed: {}", e))
                })?;
        }

        tracing::info!("Discord channels inserted successfully");
        Ok(())
    }

    fn should_process(&self, act_name: &str, response: &str) -> bool {
        let name_match = act_name.to_lowercase().contains("channel");
        let content_match = response.contains("\"channel_type\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordChannelProcessor"
    }
}

// Similar processors for User, Role, GuildMember, MemberRole...
// (Pattern is the same, just different types and tables)
```

### Step 7: Module Exports

Update `src/lib.rs`:

```rust
// Add to existing module declarations
mod narrative;
#[cfg(feature = "discord")]
mod discord;

// Export extraction utilities
pub use narrative::extraction::{extract_json, extract_toml, parse_json, parse_toml};

// Export processor types
pub use narrative::processor::{ActProcessor, ProcessorRegistry};

// Export Discord processors (if feature enabled)
#[cfg(feature = "discord")]
pub use discord::processors::{
    DiscordChannelProcessor, DiscordGuildProcessor, DiscordRoleProcessor,
    DiscordUserProcessor,
};

// Export Discord JSON models
#[cfg(feature = "discord")]
pub use discord::json_models::{
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRoleJson, DiscordUserJson,
};
```

## Testing Strategy

### Unit Tests

Test each component in isolation:

```rust
// tests/narrative_extraction_test.rs
use boticelli::{extract_json, parse_json};

fn test_extract_json_from_markdown() {
    let response = r#"
Here's your data:

{"id": 123, "name": "Test"}
"#;

    let json = extract_json(response).unwrap();
    assert!(json.contains("123"));
}

fn test_extract_json_with_nested_objects() {
    let response = r#"{"outer": {"inner": {"value": "test"}}}"#;
    let json = extract_json(response).unwrap();

    let parsed: serde_json::Value = parse_json(&json).unwrap();
    assert_eq!(parsed["outer"]["inner"]["value"], "test");
}
```

```rust
// tests/discord_processors_test.rs
use boticelli::{ActExecution, DiscordGuildProcessor};

async fn test_guild_processor() {
    let pool = setup_test_db_pool();
    let processor = DiscordGuildProcessor::new(pool);

    let execution = ActExecution {
        act_name: "create_guild".to_string(),
        response: r#"{"id": 123, "name": "Test Guild", "owner_id": 456}"#.to_string(),
        // ... other fields ...
    };

    processor.process(&execution).await.unwrap();

    // Verify insertion
    // ... query database and assert ...
}
```

### Integration Tests

Test the full pipeline:

```rust
// tests/narrative_processor_integration_test.rs
use boticelli::{
    DiscordChannelProcessor, DiscordGuildProcessor, NarrativeExecutor, ProcessorRegistry,
};

async fn test_full_discord_narrative_with_processors() {
    let pool = setup_test_db_pool();
    let driver = MockDriver::new();

    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(pool.clone())));
    registry.register(Box::new(DiscordChannelProcessor::new(pool.clone())));

    let executor = NarrativeExecutor::with_processors(driver, registry);

    let narrative = /* load test narrative */;
    let result = executor.execute(&narrative).await.unwrap();

    // Verify data was inserted
    let mut conn = pool.get().unwrap();
    let guilds: Vec<DiscordGuildRow> = discord_guilds::table.load(&mut conn).unwrap();
    assert!(!guilds.is_empty());
}
```

## CLI Integration

Add CLI commands to test the system:

```rust
// In your CLI handler

/// Execute a narrative with Discord data processors
#[derive(clap::Args)]
pub struct ExecuteDiscordNarrative {
    /// Path to narrative TOML file
    narrative_path: PathBuf,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
}

impl ExecuteDiscordNarrative {
    pub async fn run(&self) -> BoticelliResult<()> {
        // Load narrative
        let narrative = load_narrative(&self.narrative_path)?;

        // Setup database pool
        let pool = create_db_pool(&self.database_url)?;

        // Setup processors
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(DiscordGuildProcessor::new(pool.clone())));
        registry.register(Box::new(DiscordChannelProcessor::new(pool.clone())));
        registry.register(Box::new(DiscordRoleProcessor::new(pool.clone())));
        registry.register(Box::new(DiscordUserProcessor::new(pool.clone())));
        registry.register(Box::new(DiscordGuildMemberProcessor::new(pool.clone())));

        // Setup driver
        let driver = create_driver()?;

        // Execute
        let executor = NarrativeExecutor::with_processors(driver, registry);
        let result = executor.execute(&narrative).await?;

        println!("Narrative executed successfully!");
        println!("Acts completed: {}", result.act_executions.len());

        Ok(())
    }
}
```

## Error Handling

### Graceful Degradation

Processors should not fail the entire narrative:

```rust
// In executor.rs
if let Some(registry) = &self.processor_registry {
    if let Err(e) = registry.process(&act_execution).await {
        // Log error but continue execution
        tracing::error!(
            act = %act_name,
            error = %e,
            "Act processing failed, narrative continues"
        );

        // Optionally: store error in execution metadata
        // act_execution.processing_errors.push(e.to_string());
    }
}
```

### Validation Errors

Provide clear error messages:

```rust
// In processor
let json_str = extract_json(&execution.response).map_err(|e| {
    BoticelliError::new(format!(
        "Failed to extract JSON from act '{}': {}. Response preview: {}",
        execution.act_name,
        e,
        &execution.response[..execution.response.len().min(200)]
    ))
})?;
```

## Performance Considerations

### Batch Inserts

For array responses, use batch inserts:

```rust
// Instead of inserting one by one
for channel in channels {
    diesel::insert_into(discord_channels::table)
        .values(&channel)
        .execute(&mut conn)?;
}

// Use batch insert
diesel::insert_into(discord_channels::table)
    .values(&channel_rows)
    .execute(&mut conn)?;
```

### Connection Pooling

Reuse database connections:

```rust
// Create pool once
let pool = Pool::builder()
    .max_size(10)
    .build(manager)?;

// Pass to all processors
registry.register(Box::new(DiscordGuildProcessor::new(pool.clone())));
```

## Future Enhancements

### 1. Processor Configuration

Allow processors to be configured via narrative metadata:

```toml
[acts.create_guild]
processors = ["discord_guild"]  # Explicit processor list
validate_schema = true           # Enable JSON schema validation
```

### 2. Schema Validation

Validate JSON against schemas before insertion:

```rust
use jsonschema::JSONSchema;

let schema = load_schema("discord_guild_schema.json")?;
let compiled = JSONSchema::compile(&schema)?;

if !compiled.is_valid(&json_value) {
    return Err(/* validation errors */);
}
```

### 3. Dry Run Mode

Test narratives without database insertion:

```rust
pub struct DryRunProcessor {
    inner: Box<dyn ActProcessor>,
}

impl ActProcessor for DryRunProcessor {
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        println!("DRY RUN: Would process with {}", self.inner.name());
        // Parse and validate but don't insert
        Ok(())
    }
}
```

### 4. Metrics and Observability

Track processor performance:

```rust
#[async_trait]
impl ActProcessor for InstrumentedProcessor {
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        let start = std::time::Instant::now();
        let result = self.inner.process(execution).await;
        let duration = start.elapsed();

        tracing::info!(
            processor = self.inner.name(),
            duration_ms = duration.as_millis(),
            success = result.is_ok(),
            "Processor completed"
        );

        result
    }
}
```

## Migration Path

### Phase 1: Foundation (Week 1)
- Implement extraction utilities
- Create ActProcessor trait
- Add processor registry

### Phase 2: Discord Processors (Week 2)
- Implement JSON models
- Create conversion functions
- Build Discord processors

### Phase 3: Integration (Week 3)
- Update executor
- Add CLI commands
- Write tests

### Phase 4: Polish (Week 4)
- Error handling improvements
- Documentation
- Performance optimization

## Conclusion

This architecture provides a clean, extensible way to process narrative outputs. The processor pattern separates concerns, making it easy to:

- Add new data types (Twitter, Reddit, etc.)
- Test components in isolation
- Compose processors for complex workflows
- Handle errors gracefully without breaking narratives

The system is production-ready while remaining flexible for future enhancements.
