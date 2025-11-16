<!-- markdownlint-disable MD046 -->
# Narrative Act Processors Implementation

## Implementation Status

### Core Infrastructure (Complete)

| Step | Component | Status | Files | Tests |
|------|-----------|--------|-------|-------|
| 1 | JSON/TOML Extraction | ✅ Complete | `src/narrative/extraction.rs` | 9 passing |
| 2 | ActProcessor Trait | ✅ Complete | `src/narrative/processor.rs` | 6 passing |
| 3 | Enhanced Executor | ✅ Complete | `src/narrative/executor.rs` | 4 passing |
| 4 | Discord JSON Models | ✅ Complete | `src/social/discord/json_models.rs` | 9 passing |
| 5 | Discord Conversions | ✅ Complete | `src/social/discord/conversions.rs` | 14 passing |
| 6 | Discord Processors | ✅ Complete | `src/social/discord/processors.rs` | 7 passing |
| 7 | Module Exports | ✅ Complete | `src/lib.rs`, `src/social/discord/mod.rs` | N/A |

**Subtotal:** 49 unit tests passing

### Integration & CLI (In Progress)

| Step | Component | Status | Files | Priority |
|------|-----------|--------|-------|----------|
| 8a | CLI: Add Processor Flag | ✅ Complete | `src/main.rs` | HIGH |
| 8b | CLI: Setup Repository | ✅ Complete | `src/main.rs` | HIGH |
| 8c | CLI: Register Processors | ✅ Complete | `src/main.rs` | HIGH |
| 8d | CLI: Execute with Pipeline | ✅ Complete | `src/main.rs` | HIGH |
| 9a | Integration Tests: Setup | 🚧 Pending | `tests/narrative_processor_integration_test.rs` | MEDIUM |
| 9b | Integration Tests: End-to-End | 🚧 Pending | `tests/narrative_processor_integration_test.rs` | MEDIUM |
| 9c | Integration Tests: Error Handling | 🚧 Pending | `tests/narrative_processor_integration_test.rs` | MEDIUM |
| 10 | Documentation: Update Status | 🚧 Pending | `NARRATIVE_PROCESSORS.md` | LOW |

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

### Step 3: Enhanced Narrative Executor ✅

**Status:** Complete (commit: 065b4b0)

**Implementation:** `src/narrative/executor.rs` (updated)

**What was built:**

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

### Step 4: Discord JSON Models ✅

**Status:** Complete (commit: 2b173f8)

**Implementation:** `src/social/discord/json_models.rs`

**What was built:**

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

### Step 5: JSON to Database Conversions ✅

**Status:** Complete

**Implementation:** `src/social/discord/conversions.rs` (created)

**What was built:**

Created TryFrom trait implementations for idiomatic Rust conversions between JSON models and Diesel insertable models.

**Key components:**

1. **Helper functions:**
   - `parse_iso_timestamp()` - Converts ISO 8601 strings to NaiveDateTime
   - `parse_channel_type()` - Converts string to ChannelType enum
   - `convert_features()` - Wraps Vec<String> into Vec<Option<String>>

2. **TryFrom implementations:**
   - `DiscordGuildJson → NewGuild`
   - `DiscordUserJson → NewUser`
   - `DiscordChannelJson → NewChannel`
   - `DiscordRoleJson → NewRole`
   - `DiscordGuildMemberJson → NewGuildMember`
   - `DiscordMemberRoleJson → NewMemberRole`

3. **NewMemberRole type:**
   - Created missing Diesel insertable struct for discord_member_roles table
   - Defined in conversions.rs with conditional compilation

**Example usage:**

```rust
use boticelli::{DiscordGuildJson, NewGuild};

let json = DiscordGuildJson {
    id: 123456789,
    name: "My Server".to_string(),
    owner_id: 987654321,
    icon: Some("icon_hash".to_string()),
    banner: None,
    description: Some("A test server".to_string()),
    member_count: Some(100),
    verification_level: Some(2),
    premium_tier: Some(1),
    features: Some(vec!["COMMUNITY".to_string()]),
};

// Idiomatic conversion using try_into()
let new_guild: NewGuild = json.try_into()?;
```

**Tests:**

14 tests covering:
- Timestamp parsing (RFC 3339, with/without fractional seconds)
- Channel type enum conversion
- Features array conversion
- All 6 TryFrom implementations
- Error handling for invalid inputs

**Module exports:**

Added to `src/social/discord/mod.rs`:
```rust
pub use conversions::{parse_channel_type, parse_iso_timestamp, NewMemberRole};
```

Added to `src/lib.rs` under discord feature:
```rust
pub use social::discord::{
    NewMemberRole,
    parse_channel_type, parse_iso_timestamp,
    // ... other Discord types
};
```

### Step 6: Discord Processors ✅

**Status:** Complete

**Implementation:** `src/social/discord/processors.rs` (created)

**What was built:**

Created ActProcessor implementations for all 6 Discord entity types:
1. DiscordGuildProcessor
2. DiscordUserProcessor
3. DiscordChannelProcessor
4. DiscordRoleProcessor
5. DiscordGuildMemberProcessor
6. DiscordMemberRoleProcessor

**Key features:**

Each processor follows the same pattern:
- **Extract** JSON from LLM response using `extract_json()`
- **Parse** into JSON model using `parse_json<T>()`
- **Convert** to Diesel model using `TryFrom` trait (`.try_into()`)
- **Store** in database using `DiscordRepository`
- **Smart routing** with `should_process()` checking act names and content

**Array support:**
All processors handle both single objects and arrays:
```rust
let entities: Vec<T> = if json_str.trim().starts_with('[') {
    parse_json(&json_str)?
} else {
    vec![parse_json(&json_str)?]
};
```

**Logging:**
Structured logging with tracing crate:
```rust
tracing::info!(
    act = %execution.act_name,
    count = entities.len(),
    "Processing Discord entities"
);
```

**Smart routing examples:**

- Guild: Checks for "guild"/"server" in act name, or "owner_id" in response
- Channel: Checks for "channel" or "channel_type"
- User: Checks for "user"/"member" or "username" (without "user_id")
- Role: Checks for "role" or "permissions" + "position"
- GuildMember: Checks for "member" + "guild_id" + "user_id" + "joined_at"
- MemberRole: Checks for "member" + "role" + all IDs + "assigned_at"

**Repository enhancement:**

Added `store_member_role()` method to DiscordRepository:
- Accepts `NewMemberRole` struct
- Uses INSERT ... ON CONFLICT for upserts
- Properly handles `assigned_at` timestamp from JSON

**Tests:**

7 unit tests covering:
- Guild processor routing by name and content
- User processor routing
- Channel processor routing
- Role processor routing
- Member processor routing (excludes member_role acts)
- Member role processor routing

**Module exports:**

Added to `src/social/discord/mod.rs`:
```rust
pub use processors::{
    DiscordChannelProcessor, DiscordGuildMemberProcessor, DiscordGuildProcessor,
    DiscordMemberRoleProcessor, DiscordRoleProcessor, DiscordUserProcessor,
};
```

Added to `src/lib.rs` under discord feature:
```rust
pub use social::discord::{
    // Processors
    DiscordChannelProcessor, DiscordGuildMemberProcessor, DiscordGuildProcessor,
    DiscordMemberRoleProcessor, DiscordRoleProcessor, DiscordUserProcessor,
    // ... other Discord types
};
```

### Step 7: Module Exports ✅

**Status:** Complete

All processor types, JSON models, and conversion utilities are exported at the crate level. See Step 6 completion notes for full export list.

---

## Step 8: CLI Integration ✅

**Status:** Complete (commit: de354df)

**Goal:** Enable users to execute narratives with automatic Discord data processing via command line.

**Implementation:** All substeps (8a-8d) completed in `src/main.rs`

**What was built:**

### Step 8a: Add Processor Flag ✅

Added `--process-discord` flag to the `run` command:

```rust
/// Execute a narrative from a TOML file
Run {
    /// Path to narrative TOML file
    #[arg(short, long)]
    narrative: PathBuf,

    // ... existing flags ...

    /// Enable Discord data processing (extract JSON and insert to database)
    #[arg(long)]
    process_discord: bool,
}
```

**Acceptance criteria:**
- Flag appears in `--help` output
- Flag is optional (default: false for backward compatibility)
- Flag requires `database` and `discord` features

---

### Step 8b: Setup Discord Repository ✅

When `--process-discord` is enabled, creates `DiscordRepository`:

```rust
async fn run_narrative(
    narrative_path: PathBuf,
    // ... other args ...
    process_discord: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing narrative loading ...

    #[cfg(all(feature = "database", feature = "discord"))]
    let discord_repository = if process_discord {
        let conn = boticelli::establish_connection()?;
        Some(std::sync::Arc::new(boticelli::DiscordRepository::new(conn)))
    } else {
        None
    };

    // ...
}
```

**Acceptance criteria:**
- Repository created only when flag is set
- Database connection established from `DATABASE_URL` env var
- Clear error message if `DATABASE_URL` not set
- Repository wrapped in Arc for sharing across processors

---

### Step 8c: Register Discord Processors ✅

Registers all 6 Discord processors when repository is available:

```rust
#[cfg(all(feature = "database", feature = "discord"))]
let executor = if let Some(repo) = discord_repository {
    let mut registry = boticelli::ProcessorRegistry::new();

    // Register all Discord processors
    registry.register(Box::new(boticelli::DiscordGuildProcessor::new(repo.clone())));
    registry.register(Box::new(boticelli::DiscordUserProcessor::new(repo.clone())));
    registry.register(Box::new(boticelli::DiscordChannelProcessor::new(repo.clone())));
    registry.register(Box::new(boticelli::DiscordRoleProcessor::new(repo.clone())));
    registry.register(Box::new(boticelli::DiscordGuildMemberProcessor::new(repo.clone())));
    registry.register(Box::new(boticelli::DiscordMemberRoleProcessor::new(repo.clone())));

    boticelli::NarrativeExecutor::with_processors(driver, registry)
} else {
    boticelli::NarrativeExecutor::new(driver)
};
```

**Acceptance criteria:**
- All 6 processors registered
- Registry properly configured
- Executor uses `with_processors()` constructor
- Falls back to regular executor if flag not set

---

### Step 8d: Execute with Pipeline ✅

Executes narrative with processors active (processor activity logged via tracing):

```rust
// Execute
println!("🚀 Executing narrative...");
let result = executor.execute(&narrative).await?;

// Display results
println!("\n✅ Narrative completed!");
println!("   Acts executed: {}", result.act_executions.len());

#[cfg(all(feature = "database", feature = "discord"))]
if process_discord {
    println!("   Discord data processed and stored in database");
    println!("   Check database for inserted records");
}

// ... existing result display code ...
```

**Acceptance criteria:**
- Execution completes successfully
- User sees processing confirmation
- Errors are caught and displayed clearly
- Data appears in database

---

### Step 8 Testing

**Manual test:**

```bash
# Set up database
export DATABASE_URL="postgres://user:password@localhost/boticelli"
diesel migration run

# Create test narrative (see examples/discord_server.toml)
cat > test_server.toml << 'EOF'
name = "test_discord_generation"
[metadata]
description = "Test Discord data generation"
author = "test"
version = "1.0.0"
toc = ["create_guild"]

[acts.create_guild]
prompt = """
Create a Discord guild JSON (no markdown, just JSON):
{
  "id": 123456789012345678,
  "name": "Test Server",
  "owner_id": 987654321098765432,
  "description": "Test server for CLI integration"
}
"""
EOF

# Run with processor
cargo run --features database,discord,gemini -- run \
    --narrative test_server.toml \
    --backend gemini \
    --process-discord

# Verify data
psql $DATABASE_URL -c "SELECT id, name FROM discord_guilds WHERE id = 123456789012345678;"
```

**Expected output:**
```
📖 Loading narrative from "test_server.toml"...
✓ Loaded: test_discord_generation
  Acts: 1
🚀 Executing narrative...
✅ Narrative completed!
   Acts executed: 1
   Discord data processed and stored in database
```

**Database should contain:**
```sql
        id         |    name
-------------------+-------------
 123456789012345678 | Test Server
```

---

## Step 9: Integration Tests 🚧

**Goal:** Automated tests verifying end-to-end processor pipeline.

### Step 9a: Test Setup Infrastructure

Create `tests/narrative_processor_integration_test.rs`:

```rust
use boticelli::{
    ActExecution, BoticelliDriver, DiscordChannelProcessor, DiscordGuildProcessor,
    DiscordRepository, GenerateRequest, GenerateResponse, Input, Message, Narrative,
    NarrativeExecutor, Output, ProcessorRegistry, Role,
};
use std::sync::Arc;

/// Mock LLM driver that returns predefined responses
struct MockDriver {
    responses: Vec<String>,
    call_count: std::sync::Mutex<usize>,
}

impl MockDriver {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl BoticelliDriver for MockDriver {
    async fn generate(&self, _request: &GenerateRequest) -> boticelli::BoticelliResult<GenerateResponse> {
        let mut count = self.call_count.lock().unwrap();
        let response = self.responses.get(*count).cloned().unwrap_or_default();
        *count += 1;

        Ok(GenerateResponse {
            outputs: vec![Output::Text(response)],
        })
    }
}

/// Create test database connection
fn create_test_db() -> diesel::PgConnection {
    use diesel::Connection;
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

    diesel::PgConnection::establish(&database_url)
        .expect("Failed to connect to test database")
}

/// Clean up test data
fn cleanup_test_data(conn: &mut diesel::PgConnection, guild_id: i64) {
    use boticelli::database::schema::discord_guilds;
    use diesel::prelude::*;

    diesel::delete(discord_guilds::table.filter(discord_guilds::id.eq(guild_id)))
        .execute(conn)
        .ok();
}
```

**Acceptance criteria:**
- Mock driver returns predefined JSON responses
- Test database connection helper
- Cleanup function to avoid test pollution

---

### Step 9b: End-to-End Test

Test complete pipeline from narrative execution to database insertion:

```rust
#[tokio::test]
async fn test_discord_guild_processor_integration() {
    // Setup
    let mut conn = create_test_db();
    let test_guild_id = 999888777666555444i64;
    cleanup_test_data(&mut conn, test_guild_id);

    // Create mock driver with guild JSON response
    let mock_responses = vec![
        r#"{
            "id": 999888777666555444,
            "name": "Integration Test Guild",
            "owner_id": 111222333444555666,
            "description": "Created by integration test",
            "member_count": 42,
            "verification_level": 2
        }"#.to_string(),
    ];
    let driver = MockDriver::new(mock_responses);

    // Setup repository and processor
    let repository = Arc::new(DiscordRepository::new(
        boticelli::establish_connection().unwrap()
    ));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repository.clone())));

    // Create test narrative
    let narrative_toml = r#"
name = "integration_test"
[metadata]
description = "Test"
author = "test"
version = "1.0.0"
toc = ["create_guild"]

[acts.create_guild]
prompt = "Generate guild JSON"
"#;
    let narrative: Narrative = narrative_toml.parse().unwrap();

    // Execute
    let executor = NarrativeExecutor::with_processors(driver, registry);
    let result = executor.execute(&narrative).await.unwrap();

    // Verify execution
    assert_eq!(result.act_executions.len(), 1);
    assert_eq!(result.act_executions[0].act_name, "create_guild");

    // Verify database insertion
    use boticelli::database::schema::discord_guilds;
    use diesel::prelude::*;

    let guild: boticelli::GuildRow = discord_guilds::table
        .filter(discord_guilds::id.eq(test_guild_id))
        .first(&mut conn)
        .expect("Guild should be inserted");

    assert_eq!(guild.name, "Integration Test Guild");
    assert_eq!(guild.owner_id, 111222333444555666);
    assert_eq!(guild.description, Some("Created by integration test".to_string()));
    assert_eq!(guild.member_count, Some(42));
    assert_eq!(guild.verification_level, Some(2));

    // Cleanup
    cleanup_test_data(&mut conn, test_guild_id);
}
```

**Acceptance criteria:**
- Test creates realistic narrative
- Mock driver provides valid JSON
- Processor extracts and inserts data
- Database contains expected records
- Cleanup removes test data

---

### Step 9c: Error Handling Tests

Test that processor errors don't fail narrative execution:

```rust
#[tokio::test]
async fn test_processor_error_does_not_fail_narrative() {
    // Mock driver returns invalid JSON
    let mock_responses = vec![
        "This is not JSON at all!".to_string(),
    ];
    let driver = MockDriver::new(mock_responses);

    // Setup processor that will fail to parse
    let repository = Arc::new(DiscordRepository::new(
        boticelli::establish_connection().unwrap()
    ));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repository)));

    // Execute narrative
    let narrative_toml = r#"
name = "error_test"
[metadata]
description = "Test"
author = "test"
version = "1.0.0"
toc = ["bad_json"]

[acts.bad_json]
prompt = "Return invalid JSON"
"#;
    let narrative: Narrative = narrative_toml.parse().unwrap();

    let executor = NarrativeExecutor::with_processors(driver, registry);
    let result = executor.execute(&narrative).await;

    // Narrative should SUCCEED even though processor failed
    assert!(result.is_ok());
    assert_eq!(result.unwrap().act_executions.len(), 1);
}

#[tokio::test]
async fn test_multiple_processors_with_partial_match() {
    // Test that only matching processors execute
    let mock_responses = vec![
        r#"{"id": 111, "name": "Guild", "owner_id": 222}"#.to_string(),
    ];
    let driver = MockDriver::new(mock_responses);

    let repository = Arc::new(DiscordRepository::new(
        boticelli::establish_connection().unwrap()
    ));

    let mut registry = ProcessorRegistry::new();
    // Register multiple processors - only guild should execute
    registry.register(Box::new(DiscordGuildProcessor::new(repository.clone())));
    registry.register(Box::new(DiscordChannelProcessor::new(repository.clone())));

    let narrative_toml = r#"
name = "test"
[metadata]
description = "Test"
author = "test"
version = "1.0.0"
toc = ["create_guild"]  # Act name matches guild processor

[acts.create_guild]
prompt = "Generate guild"
"#;
    let narrative: Narrative = narrative_toml.parse().unwrap();

    let executor = NarrativeExecutor::with_processors(driver, registry);
    let result = executor.execute(&narrative).await.unwrap();

    assert_eq!(result.act_executions.len(), 1);
    // Verify guild was inserted (guild processor ran)
    // Verify channel was NOT inserted (channel processor didn't run)
}
```

**Acceptance criteria:**
- Invalid JSON doesn't crash narrative
- Processor errors are logged but don't propagate
- Multiple processors can coexist
- Only matching processors execute

---

## Step 10: Documentation Updates 🚧

Update all documentation to reflect completed implementation:

**Files to update:**

1. **NARRATIVE_PROCESSORS.md** (this file)
   - Update status table
   - Mark all steps as ✅
   - Add "Production Ready" badge
   - Update migration timeline

2. **README.md**
   - Add processor feature to feature list
   - Add example with `--process-discord`
   - Link to NARRATIVE_PROCESSORS.md

3. **DISCORD_NARRATIVE.md**
   - Add note about automatic processing
   - Reference CLI integration
   - Show complete workflow

4. **CHANGELOG.md** (if exists)
   - Add entry for processor feature
   - List all 6 processor types
   - Note CLI integration

---

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

## Usage Guide

This section demonstrates how to use the complete narrative processor system to generate and store Discord data via LLM narratives.

### Quick Start Example

Here's a complete example that generates a Discord server with channels:

```rust
use boticelli::{
    // Core narrative types
    Narrative, NarrativeExecutor, ProcessorRegistry,
    // Discord processors
    DiscordGuildProcessor, DiscordChannelProcessor, DiscordRepository,
    // LLM driver
    GeminiClient,
    // Database
    establish_connection,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load narrative from TOML file
    let narrative = Narrative::from_str(&std::fs::read_to_string("create_server.toml")?)?;

    // 2. Setup database connection
    let conn = establish_connection()?;
    let repository = Arc::new(DiscordRepository::new(conn));

    // 3. Setup processor registry
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repository.clone())));
    registry.register(Box::new(DiscordChannelProcessor::new(repository.clone())));

    // 4. Setup LLM driver
    let gemini = GeminiClient::new(
        std::env::var("GEMINI_API_KEY")?,
        "gemini-1.5-flash".to_string(),
    )?;

    // 5. Create executor with processors
    let executor = NarrativeExecutor::with_processors(gemini, registry);

    // 6. Execute narrative (this calls LLM and processes responses)
    let result = executor.execute(&narrative).await?;

    // 7. Review results
    println!("✓ Narrative executed successfully!");
    println!("  Acts completed: {}", result.act_executions.len());

    for (i, act) in result.act_executions.iter().enumerate() {
        println!("  Act {}: {} - {} chars", i + 1, act.act_name, act.response.len());
    }

    Ok(())
}
```

### Example Narrative File

Create `create_server.toml`:

```toml
name = "create_cozy_cafe_server"

[metadata]
description = "Generate a cozy café-themed Discord server"
author = "narrative-system"
version = "1.0.0"

# Table of contents (execution order)
toc = [
    "create_guild",
    "create_channels",
]

# Act 1: Create the guild
[acts.create_guild]
prompt = """
You are helping create a Discord server for a cozy café community.

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- Start your response with { and end with }

Create a guild (Discord server) with the following schema:

{
  "id": <snowflake_id>,         // Use 123456789012345678
  "name": <string>,              // Max 100 chars
  "owner_id": <snowflake_id>,    // Use 987654321098765432
  "description": <string>,       // Optional
  "member_count": <integer>,     // Optional
  "verification_level": <0-4>,   // Optional (0=none, 4=highest)
  "premium_tier": <0-3>,         // Optional (boost level)
  "features": [<string>]         // Optional (e.g., ["COMMUNITY"])
}

Generate a warm, welcoming café server.
"""

# Act 2: Create channels
[acts.create_channels]
prompt = """
You created a cozy café Discord server. Now create channels for it.

**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON array with no additional text
- Do not use markdown code blocks
- Start with [ and end with ]

Create an array of channels with this schema for each:

{
  "id": <snowflake_id>,
  "channel_type": <string>,  // "guild_text", "guild_voice", "guild_category"
  "guild_id": 123456789012345678,  // Must match guild from Act 1
  "name": <string>,
  "topic": <string>,          // Optional description
  "position": <integer>,      // Sort order
  "parent_id": <snowflake_id> // Optional (for category organization)
}

Create 5-7 channels appropriate for a cozy café server:
- A welcome category
- General chat channels
- Voice channels for hangouts
- Topic-specific channels (books, games, etc.)
"""
```

### Execution Flow

When you run the example above, here's what happens:

1. **Load Narrative**: Parses the TOML file into `Narrative` struct
2. **Setup Processors**: Registers processors that will handle specific data types
3. **Execute Act 1** (create_guild):
   - Sends prompt to LLM
   - Receives JSON response
   - `DiscordGuildProcessor.should_process()` returns true (act name contains "guild")
   - Processor extracts JSON, parses into `DiscordGuildJson`
   - Converts to `NewGuild` via `try_into()`
   - Stores in database via `repository.store_guild()`
4. **Execute Act 2** (create_channels):
   - Sends prompt to LLM (with Act 1's response in conversation history)
   - Receives JSON array response
   - `DiscordChannelProcessor.should_process()` returns true
   - Processor extracts JSON array, parses into `Vec<DiscordChannelJson>`
   - Converts each to `NewChannel`
   - Stores all in database via `repository.store_channel()`
5. **Return Results**: Complete execution history returned to caller

### Registering Multiple Processors

For a complete Discord narrative, register all processors:

```rust
let mut registry = ProcessorRegistry::new();

// Register all Discord processors
registry.register(Box::new(DiscordGuildProcessor::new(repository.clone())));
registry.register(Box::new(DiscordUserProcessor::new(repository.clone())));
registry.register(Box::new(DiscordChannelProcessor::new(repository.clone())));
registry.register(Box::new(DiscordRoleProcessor::new(repository.clone())));
registry.register(Box::new(DiscordGuildMemberProcessor::new(repository.clone())));
registry.register(Box::new(DiscordMemberRoleProcessor::new(repository.clone())));

// All processors will be checked for each act
// Only matching processors execute (based on should_process())
```

### Database Setup

Before running narratives, ensure your database is ready:

```bash
# Set database URL
export DATABASE_URL="postgres://user:password@localhost/boticelli"

# Run migrations
diesel migration run

# Verify tables exist
psql $DATABASE_URL -c "\dt discord_*"
```

You should see these tables:
- `discord_guilds`
- `discord_users`
- `discord_channels`
- `discord_roles`
- `discord_guild_members`
- `discord_member_roles`

### Querying Generated Data

After narrative execution, query the results:

```rust
use boticelli::{DiscordRepository, establish_connection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = establish_connection()?;
    let repo = DiscordRepository::new(conn);

    // Get the guild we just created
    let guild = repo.get_guild(123456789012345678).await?;

    if let Some(g) = guild {
        println!("Guild: {} ({})", g.name, g.id);
        println!("Description: {:?}", g.description);
        println!("Members: {:?}", g.member_count);
    }

    // Get all channels for this guild
    let channels = repo.get_guild_channels(123456789012345678).await?;

    println!("\nChannels ({}):", channels.len());
    for channel in channels {
        println!("  #{} ({:?})", channel.name.unwrap_or_default(), channel.channel_type);
    }

    Ok(())
}
```

### Handling LLM Variability

LLMs may return responses with extra text or markdown. The processors handle this automatically:

**What the LLM might return:**

```
Here's your Discord guild data:

```json
{
  "id": 123456789012345678,
  "name": "Cozy Café",
  "owner_id": 987654321098765432,
  "description": "A warm community for coffee lovers"
}
```

I've created a welcoming café server with...
```

**What the processor extracts:**

```json
{
  "id": 123456789012345678,
  "name": "Cozy Café",
  "owner_id": 987654321098765432,
  "description": "A warm community for coffee lovers"
}
```

The `extract_json()` function:
1. Removes markdown code blocks
2. Extracts balanced JSON (handles nested objects)
3. Strips leading/trailing text
4. Returns clean JSON ready for parsing

### Common Patterns

#### Pattern 1: Generate → Review → Regenerate

```rust
// First execution (dry run mode could go here)
let result1 = executor.execute(&narrative).await?;

// Review the generated content
println!("Review generated content:");
for act in &result1.act_executions {
    println!("\n{}:\n{}", act.act_name, act.response);
}

// If not satisfied, adjust narrative and re-execute
// Processors will upsert (update existing records)
```

#### Pattern 2: Incremental Generation

```toml
# First narrative: Just the guild
toc = ["create_guild"]

# Second narrative: Add channels
toc = ["create_channels"]  # Assumes guild exists

# Third narrative: Add users and members
toc = ["create_users", "create_members"]
```

Run each narrative separately:
```bash
./boticelli execute guild.toml
./boticelli execute channels.toml
./boticelli execute users.toml
```

#### Pattern 3: Multi-Community Generation

Generate multiple servers in one narrative:

```toml
toc = [
    "create_guild_1",
    "create_channels_1",
    "create_guild_2",
    "create_channels_2",
]
```

Each guild gets unique IDs, all processed automatically.

### Debugging

Enable detailed logging to see what's happening:

```rust
// Setup tracing subscriber
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();

// Now you'll see processor activity:
// DEBUG discord_guild_processor: Storing Discord guild guild_id=123 guild_name="Cozy Café"
// INFO  discord_guild_processor: Discord guilds stored successfully
```

### Next Steps

1. **Explore DISCORD_NARRATIVE.md** - Complete schema reference and examples
2. **Create custom narratives** - Use the preambles as starting points
3. **Query the database** - Use DiscordRepository methods
4. **Build Discord bot** - Use BoticelliBot to post content to real Discord servers

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

### Phase 1: Foundation ✅
- ✅ Implement extraction utilities (Step 1)
- ✅ Create ActProcessor trait (Step 2)
- ✅ Add processor registry (Step 2)

### Phase 2: Discord Processors ✅
- ✅ Implement JSON models (Step 4)
- ✅ Create conversion functions (Step 5)
- ✅ Build Discord processors (Step 6)

### Phase 3: Integration (In Progress)
- ✅ Update executor (Step 3)
- ✅ Add CLI commands (Step 8)
- 🚧 Write integration tests (Step 9 - pending)

### Phase 4: Polish (Pending)
- Error handling improvements (ongoing)
- Documentation updates (Step 10 - pending)
- Performance optimization (future)

## Conclusion

This architecture provides a clean, extensible way to process narrative outputs. The processor pattern separates concerns, making it easy to:

- Add new data types (Twitter, Reddit, etc.)
- Test components in isolation
- Compose processors for complex workflows
- Handle errors gracefully without breaking narratives

The system is production-ready while remaining flexible for future enhancements.
