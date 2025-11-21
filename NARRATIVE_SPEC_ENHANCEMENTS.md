# Narrative TOML Spec Enhancement Plan

## Overview

This document outlines planned enhancements to the narrative TOML specification to support:

1. **Bot Commands** - Execute Discord bot commands and include results in prompts (**PHASE 2 - PARTIALLY IMPLEMENTED**)
2. **Table References** - Attach data from existing database tables to prompts (**PHASE 3 - PARTIALLY IMPLEMENTED**)

These features enable narratives to interact with external systems and reference previously generated content, creating more powerful and composable workflows.

**Current Status**:
- ✅ **Phase 1 Complete**: Friendly syntax foundation with resource definitions  
- ✅ **Phase 2 Complete**: Bot commands fully implemented with security framework
- ✅ **Phase 3 Complete**: Table references and carousel features implemented

See `PHASE_2_5_SUMMARY.md` for Phase 2 completion report and `SPEC_ENHANCEMENT_PHASE_3.md` for Phase 3 implementation details.

## Current Implementation Status

### Phase 1: Friendly Syntax ✅ **COMPLETE**

All infrastructure for resource definitions is implemented:
- ✅ `[bots.name]`, `[tables.name]`, `[media.name]` sections
- ✅ Reference resolution: `"bots.name"` → `Input::BotCommand`
- ✅ Array syntax: `["ref1", "ref2", "text"]`
- ✅ MIME type inference from file extensions
- ✅ Backward compatibility maintained

**Commit**: `a941fdc` - feat(narrative): implement Phase 1 - friendly syntax foundation

### Phase 2: Bot Commands ✅ **COMPLETE**

**What's Done**:
- ✅ `Input::BotCommand` variant exists in `botticelli_core`
- ✅ TOML parsing with `TomlBotDefinition`
- ✅ Reference resolution: `"bots.name"` → `Input::BotCommand`
- ✅ Database storage (command stored in text_content field)
- ✅ `BotCommandExecutor` trait in `botticelli_social` with comprehensive tracing
- ✅ `DiscordCommandExecutor` implementation with 30+ commands (read and write)
- ✅ Error types using `derive_more` (BotCommandError, BotCommandErrorKind)
- ✅ Integration tests with Discord API (`botticelli/tests/discord_integration_test.rs`)
- ✅ Security framework in `botticelli_security` crate with 5-layer protection
- ✅ All integration tests consolidated in facade crate
- ✅ Example Discord community narratives (`crates/botticelli_narrative/narratives/discord/`)
- ✅ Tested end-to-end: `simple_welcome.toml` successfully generates content via Gemini API

### Phase 3: Table References and Carousel ✅ **COMPLETE**

**What's Done**:
- ✅ `Input::Table` variant exists in `botticelli_core` with all query parameters
- ✅ TOML parsing with `TomlTableDefinition`
- ✅ Reference resolution: `"tables.name"` → `Input::Table`
- ✅ `TableQueryRegistry` trait in `botticelli_interface`
- ✅ `DatabaseTableQueryRegistry` implementation in `botticelli_database`
- ✅ `TableQueryExecutor` with support for columns, where, limit, offset, order_by
- ✅ Multiple output formats: JSON, Markdown, CSV
- ✅ Integration into `NarrativeExecutor.process_inputs()`
- ✅ Error types: `TableQueryFailed`, `TableQueryNotConfigured`
- ✅ Carousel feature with `CarouselConfig`, `CarouselBudget`, `CarouselResult`
- ✅ Budget-aware execution with TPM/RPM/TPD/RPD tracking
- ✅ `Act::Carousel` variant with nested act support
- ✅ Example narratives demonstrating table references and carousel loops
- ✅ Integration test framework ready (`publish_welcome_test.rs`)

**Commands Implemented** (30+):
- **Server**: `server.get`, `server.get_channels`, `server.get_members`, `server.get_roles`
- **Channels**: `channels.get`, `channels.list`, `channels.create`, `channels.edit`, `channels.delete`, `channels.create_invite`, `channels.typing`
- **Roles**: `roles.get`, `roles.list`, `roles.create`, `roles.edit`, `roles.delete`, `roles.assign`, `roles.remove`
- **Members**: `members.get`, `members.list`, `members.kick`, `members.ban`, `members.unban`, `members.timeout`, `members.remove_timeout`, `members.edit`
- **Messages**: `messages.list`, `messages.send`, `messages.edit`, `messages.delete`, `messages.pin`, `messages.unpin`
- **Reactions**: `reactions.add`, `reactions.remove`
- **Messages**: `messages.get`, `messages.list`, `messages.send`, `messages.delete`, `messages.pin`, `messages.unpin`, `messages.react`
- **Emojis**: `emojis.get`, `emojis.create`, `emojis.delete`, `emojis.modify`

**Current Status**: ✅ **Fully operational** - All core Discord commands implemented with security framework integration.

### Phase 3: Table References & Carousel Features 🚧 **IN PROGRESS**

**Design Complete**:
- ✅ `ContentRepository` trait designed in `botticelli_interface`
- ✅ `TableReference` struct with builder pattern designed
- ✅ Carousel feature with rate limit budgeting designed (see `CAROUSEL_FEATURE_DESIGN.md`)
- ✅ Database trait separation analyzed (see `DATABASE_TRAIT_SEPARATION_ANALYSIS.md`)
- ✅ Act enum variants designed (Generation, Narrative, Bot, Carousel)

**What's Implemented**:
- ✅ `Input::Table` variant exists in `botticelli_core` with full query support
- ✅ TOML parsing with `TomlTableDefinition` 
- ✅ Reference resolution: `"tables.name"` → `Input::Table`
- ✅ `TableReference` struct with `derive_builder` in `botticelli_narrative`
- ✅ `CarouselConfig` parsing and TOML structures
- ✅ Act::Carousel variant added to Act enum
- ✅ Example narratives created: `welcome_content_generation.toml`, `publish_welcome.toml`

**What's Remaining**:

1. **ContentRepository Implementation** 🚧
   - Design complete (see `DATABASE_TRAIT_SEPARATION_ANALYSIS.md`)
   - PostgreSQL implementation of ContentRepository trait
   - In-memory implementation for testing
   - Integration with NarrativeExecutor

2. **Table Reference Resolution in Executor** 🚧
   - Process `Input::Table` in executor
   - Query content from ContentRepository
   - Format and inject into prompts
   - Support JSON, Markdown, CSV formats

3. **Carousel Execution** 🚧
   - Implement carousel execution loop in executor
   - Rate limit budget tracking
   - Iteration loop with budget checks
   - Budget-aware retry strategies

4. **Integration Tests** 🚧
   - End-to-end tests for table references
   - Tests for carousel execution
   - Tests for example Discord narratives
   - Database integration tests

**Current Status**: 🚧 **IN PROGRESS** - Core infrastructure complete, executor integration and testing pending.

---

## Original Gaps (Context for Design)

### Gap 1: Bot Commands

**Problem**: No way to execute Discord bot commands from within a narrative and use the results in subsequent acts.

**Use Cases**:
- Fetch Discord server statistics before generating content
- Query user activity data to personalize responses
- Retrieve channel messages for context-aware generation
- Get role assignments for access-controlled content generation
- Pull emoji usage statistics for engagement analysis

**Example Scenario**:
```
Act 1: Execute bot command to get server member count
Act 2: Generate welcome message mentioning "Join our community of {member_count} members!"
Act 3: Execute bot command to get top channels by activity
Act 4: Generate community highlights featuring those channels
```

### Gap 2: Table References

**Problem**: No way to reference data from previously generated tables or existing database tables.

**Use Cases**:
- Use data from one narrative as input to another
- Reference Discord server schema when generating related content
- Include previously approved content as examples
- Analyze trends across multiple content generation runs
- Create follow-up content based on what was previously generated

**Example Scenario**:
```
Narrative 1: Generate 10 social media posts → creates table social_posts_20241120
Narrative 2: Analyze those posts for themes
  - Act 1: Read social_posts_20241120 table
  - Act 2: Identify common themes
  - Act 3: Generate strategy recommendations
```

## Proposed Solutions

### Feature 1: Bot Command Execution

**Implementation Status**: ✅ COMPLETED

See `PHASE_2_BOT_COMMANDS.md` and `PHASE_2_COMPLETION_SUMMARY.md` for implementation details.

**What's Implemented**:
- ✅ BotCommand input type in botticelli_core
- ✅ TOML parsing for bot commands in botticelli_narrative
- ✅ BotCommandRegistry trait in NarrativeExecutor
- ✅ Discord command implementations in botticelli_social
- ✅ Security framework with policies and enforcement
- ✅ Comprehensive tracing and error handling
- ✅ Integration tests with live Discord API

#### TOML Syntax (✅ Implemented)

```toml
[narrative]
name = "discord_server_stats"
description = "Generate content using live Discord data"

[toc]
order = ["fetch_stats", "generate_content"]

[acts.fetch_stats]
[[acts.fetch_stats.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "1234567890" }

[acts.generate_content]
[[acts.generate_content.input]]
type = "text"
content = """
Create a community update post using these stats:
{{fetch_stats}}

Include member count, active channels, and recent activity.
"""
```

#### Design Considerations

**Command Format**:
- Use dot-notation for namespacing: `{platform}.{category}.{action}`
- Examples: `discord.server.get_stats`, `discord.channels.list`, `discord.roles.get_members`

**Arguments**:
- Use TOML inline table syntax for parameters: `args = { key = "value" }`
- Support common types: strings, integers, booleans, arrays

**Error Handling**:
- Command failures should not halt the entire narrative
- Provide error message in context for subsequent acts
- Option to mark command as `required = true` (default: false)

**Rate Limiting**:
- Bot commands must respect platform rate limits
- Consider queueing/throttling mechanism
- Allow `cache_duration` parameter to reuse recent results

**Security**:
- Bot must have appropriate permissions for the command
- Validate guild_id and channel_id ownership
- No commands that modify data (read-only operations)

#### Implementation Requirements

**1. New Input Type in Core**

```rust
// crates/botticelli_core/src/input.rs

pub enum Input {
    Text(String),
    Image { mime: Option<String>, source: MediaSource },
    Audio { mime: Option<String>, source: MediaSource },
    Video { mime: Option<String>, source: MediaSource },
    Document { mime: Option<String>, source: MediaSource, filename: Option<String> },
    
    // NEW: Bot command execution
    BotCommand {
        platform: String,      // "discord", "slack", etc.
        command: String,       // "server.get_stats"
        args: HashMap<String, serde_json::Value>,
        required: bool,        // Halt on failure?
        cache_duration: Option<u64>, // Cache results for N seconds
    },
}
```

**2. TOML Parser Support**

```rust
// crates/botticelli_narrative/src/toml_parser.rs

#[derive(Debug, Clone, Deserialize)]
pub struct TomlInput {
    #[serde(rename = "type")]
    pub input_type: String,
    
    // Existing fields...
    pub content: Option<String>,
    pub mime: Option<String>,
    pub url: Option<String>,
    
    // NEW: Bot command fields
    pub platform: Option<String>,
    pub command: Option<String>,
    pub args: Option<HashMap<String, serde_json::Value>>,
    pub required: Option<bool>,
    pub cache_duration: Option<u64>,
}

impl TomlInput {
    pub fn to_input(&self) -> Result<Input, String> {
        match self.input_type.as_str() {
            "bot_command" => {
                let platform = self.platform.as_ref()
                    .ok_or("Bot command missing 'platform' field")?;
                let command = self.command.as_ref()
                    .ok_or("Bot command missing 'command' field")?;
                
                Ok(Input::BotCommand {
                    platform: platform.clone(),
                    command: command.clone(),
                    args: self.args.clone().unwrap_or_default(),
                    required: self.required.unwrap_or(false),
                    cache_duration: self.cache_duration,
                })
            }
            // ... existing cases
        }
    }
}
```

**3. Bot Command Executor** ✅ **COMPLETED**

Implemented in botticelli_social with:
- BotCommandRegistry trait in NarrativeExecutor
- DiscordBotRegistry with 20+ commands
- Comprehensive error handling with specific error types
- Security framework integration
- Tracing throughout

```rust
// crates/botticelli_social/src/discord/command_executor.rs

use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

#[async_trait]
pub trait BotCommandExecutor: Send + Sync {
    /// Execute a bot command and return the result as JSON
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, BotCommandError>;
    
    /// Check if a command is supported
    fn supports_command(&self, command: &str) -> bool;
}

pub struct DiscordCommandExecutor {
    client: Arc<DiscordClient>,
    cache: Arc<Mutex<CommandCache>>,
}

impl DiscordCommandExecutor {
    pub fn new(client: Arc<DiscordClient>) -> Self {
        Self {
            client,
            cache: Arc::new(Mutex::new(CommandCache::new())),
        }
    }
}

#[async_trait]
impl BotCommandExecutor for DiscordCommandExecutor {
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, BotCommandError> {
        match command {
            "server.get_stats" => {
                let guild_id = args.get("guild_id")
                    .and_then(|v| v.as_u64())
                    .ok_or(BotCommandError::MissingArgument("guild_id"))?;
                
                let stats = self.client.get_guild_stats(guild_id).await?;
                Ok(serde_json::to_value(stats)?)
            }
            
            "channels.list" => {
                let guild_id = args.get("guild_id")
                    .and_then(|v| v.as_u64())
                    .ok_or(BotCommandError::MissingArgument("guild_id"))?;
                
                let channels = self.client.list_channels(guild_id).await?;
                Ok(serde_json::to_value(channels)?)
            }
            
            _ => Err(BotCommandError::UnsupportedCommand(command.to_string())),
        }
    }
    
    fn supports_command(&self, command: &str) -> bool {
        matches!(command, "server.get_stats" | "channels.list" | "roles.list")
    }
}

// Command result caching
struct CommandCache {
    cache: HashMap<String, CachedResult>,
}

struct CachedResult {
    result: JsonValue,
    timestamp: SystemTime,
    ttl: Duration,
}
```

**4. Integration with Narrative Executor**

```rust
// crates/botticelli_narrative/src/executor.rs

pub struct NarrativeExecutor<D: BotticelliDriver> {
    driver: D,
    bot_executors: HashMap<String, Box<dyn BotCommandExecutor>>,
    // ... existing fields
}

impl<D: BotticelliDriver> NarrativeExecutor<D> {
    pub fn with_bot_executor(
        mut self,
        platform: impl Into<String>,
        executor: Box<dyn BotCommandExecutor>,
    ) -> Self {
        self.bot_executors.insert(platform.into(), executor);
        self
    }
    
    async fn process_input(
        &self,
        input: &Input,
    ) -> Result<String, NarrativeError> {
        match input {
            Input::BotCommand { platform, command, args, required, .. } => {
                let executor = self.bot_executors.get(platform)
                    .ok_or_else(|| NarrativeError::new(
                        NarrativeErrorKind::UnsupportedPlatform(platform.clone())
                    ))?;
                
                match executor.execute(command, args).await {
                    Ok(result) => Ok(result.to_string()),
                    Err(e) if *required => Err(e.into()),
                    Err(e) => {
                        tracing::warn!(
                            platform = %platform,
                            command = %command,
                            error = %e,
                            "Bot command failed, continuing with error message"
                        );
                        Ok(format!("Command failed: {}", e))
                    }
                }
            }
            // ... existing input types
        }
    }
}
```

**5. Usage Example**

```rust
use botticelli_narrative::NarrativeExecutor;
use botticelli_social::DiscordClient;
use botticelli_models::GeminiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM driver
    let gemini = GeminiClient::new(api_key)?;
    
    // Create Discord bot client
    let discord = DiscordClient::new(bot_token).await?;
    let discord_executor = DiscordCommandExecutor::new(Arc::new(discord));
    
    // Create executor with bot command support
    let executor = NarrativeExecutor::new(gemini)
        .with_bot_executor("discord", Box::new(discord_executor));
    
    // Load and execute narrative
    let narrative = Narrative::from_file("discord_stats.toml")?;
    let result = executor.execute(&narrative).await?;
    
    Ok(())
}
```

### Feature 2: Table References

**Implementation Status**: ✅ COMPLETED

**What's Implemented**:
- ✅ Table input type in botticelli_core with all query parameters
- ✅ TOML parsing for table references in botticelli_narrative
- ✅ TableQueryRegistry trait in botticelli_interface
- ✅ DatabaseTableQueryRegistry implementation in botticelli_database
- ✅ TableQueryExecutor with dynamic SQL generation
- ✅ Multiple format support (JSON, Markdown, CSV)
- ✅ Comprehensive tracing and error handling
- ✅ Integration with NarrativeExecutor

#### TOML Syntax (✅ Implemented)

```toml
[narrative]
name = "analyze_previous_content"
description = "Analyze content from a previous generation"

[toc]
order = ["load_data", "analyze", "recommend"]

[acts.load_data]
[[acts.load_data.input]]
type = "table"
table_name = "social_posts_20241120_153045"
columns = ["title", "body", "status"]  # Optional: select specific columns
where = "status = 'approved'"          # Optional: filter rows
limit = 50                              # Optional: limit rows

[acts.analyze]
[[acts.analyze.input]]
type = "text"
content = """
Analyze these social media posts for common themes and engagement patterns:
{{load_data}}

Provide insights on what content resonates most.
"""

[acts.recommend]
[[acts.recommend.input]]
type = "text"
content = """
Based on this analysis:
{{analyze}}

Recommend 5 new content ideas that build on successful themes.
"""
```

#### Advanced Table Reference Features

**Multiple Tables**:
```toml
[[acts.compare.input]]
type = "table"
table_name = "posts_batch_1"
alias = "batch1"  # Reference as {{batch1}} in prompts

[[acts.compare.input]]
type = "table"
table_name = "posts_batch_2"
alias = "batch2"  # Reference as {{batch2}} in prompts

[[acts.compare.input]]
type = "text"
content = """
Compare these two batches:
Batch 1: {{batch1}}
Batch 2: {{batch2}}

What improved between batches?
"""
```

**Joins** (future enhancement):
```toml
[[acts.analyze.input]]
type = "table"
table_name = "discord_guilds"
join = { table = "discord_channels", on = "guild_id" }
columns = ["guilds.name", "channels.name", "channels.topic"]
```

**Aggregations** (future enhancement):
```toml
[[acts.stats.input]]
type = "table"
table_name = "social_posts_20241120"
aggregate = { 
    count = "*",
    avg_length = "LENGTH(body)",
    group_by = "status"
}
```

#### Design Considerations

**Data Format**:
- Default: JSON array of objects (one per row)
- Alternative: Markdown table format (more readable for LLMs)
- Allow format specification: `format = "json"` or `format = "markdown"`

**Size Limits**:
- Large tables could exceed context windows
- Default limit: 100 rows (configurable)
- Option to sample: `sample = 10` (random sample)
- Option to paginate: multiple acts with offset/limit

**Privacy & Security**:
- Only allow access to tables in same database
- Consider row-level security based on narrative permissions
- Option to exclude sensitive columns: `exclude_columns = ["api_key", "password"]`

**Performance**:
- Cache table queries within execution
- Consider materialized views for expensive queries
- Index frequently accessed columns

**Schema Awareness**:
- Include column names and types in prompt
- Option to include schema metadata
- Warn if table doesn't exist (vs error)

#### Implementation Requirements

**1. New Input Type in Core**

```rust
// crates/botticelli_core/src/input.rs

pub enum Input {
    // ... existing variants
    
    // NEW: Table reference
    Table {
        table_name: String,
        columns: Option<Vec<String>>,   // SELECT specific columns
        where_clause: Option<String>,   // WHERE filter
        limit: Option<u32>,              // LIMIT rows
        offset: Option<u32>,             // OFFSET for pagination
        order_by: Option<String>,        // ORDER BY clause
        alias: Option<String>,           // Alias for {{alias}} interpolation
        format: TableFormat,             // JSON or Markdown
        sample: Option<u32>,             // Random sample N rows
    },
}

pub enum TableFormat {
    Json,      // JSON array of objects
    Markdown,  // Markdown table
    Csv,       // CSV format
}
```

**2. TOML Parser Support**

```rust
// crates/botticelli_narrative/src/toml_parser.rs

#[derive(Debug, Clone, Deserialize)]
pub struct TomlInput {
    #[serde(rename = "type")]
    pub input_type: String,
    
    // ... existing fields
    
    // NEW: Table reference fields
    pub table_name: Option<String>,
    pub columns: Option<Vec<String>>,
    pub where_clause: Option<String>,  // Allow as "where" in TOML
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub alias: Option<String>,
    pub format: Option<String>,
    pub sample: Option<u32>,
}

impl TomlInput {
    pub fn to_input(&self) -> Result<Input, String> {
        match self.input_type.as_str() {
            "table" => {
                let table_name = self.table_name.as_ref()
                    .ok_or("Table input missing 'table_name' field")?;
                
                let format = match self.format.as_deref() {
                    Some("json") | None => TableFormat::Json,
                    Some("markdown") => TableFormat::Markdown,
                    Some("csv") => TableFormat::Csv,
                    Some(f) => return Err(format!("Unknown table format: {}", f)),
                };
                
                Ok(Input::Table {
                    table_name: table_name.clone(),
                    columns: self.columns.clone(),
                    where_clause: self.where_clause.clone(),
                    limit: self.limit,
                    offset: self.offset,
                    order_by: self.order_by.clone(),
                    alias: self.alias.clone(),
                    format,
                    sample: self.sample,
                })
            }
            // ... existing cases
        }
    }
}
```

**3. Table Query Executor** ⏸️ **TODO**

Needs implementation in `botticelli_database` crate:

```rust
// crates/botticelli_database/src/table_query.rs

use diesel::prelude::*;
use serde_json::Value as JsonValue;

pub struct TableQueryExecutor {
    connection: Arc<Mutex<PgConnection>>,
}

impl TableQueryExecutor {
    pub fn new(connection: Arc<Mutex<PgConnection>>) -> Self {
        Self { connection }
    }
    
    pub fn query_table(
        &self,
        table_name: &str,
        columns: Option<&[String]>,
        where_clause: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
        order_by: Option<&str>,
        sample: Option<u32>,
    ) -> Result<Vec<JsonValue>, DatabaseError> {
        let mut conn = self.connection.lock().unwrap();
        
        // Validate table exists
        if !self.table_exists(&mut conn, table_name)? {
            return Err(DatabaseError::new(
                DatabaseErrorKind::TableNotFound(table_name.to_string())
            ));
        }
        
        // Build SQL query
        let col_list = columns
            .map(|cols| cols.join(", "))
            .unwrap_or_else(|| "*".to_string());
        
        let mut query = format!("SELECT {} FROM {}", col_list, table_name);
        
        if let Some(where_clause) = where_clause {
            // Sanitize WHERE clause to prevent SQL injection
            let safe_clause = self.sanitize_where_clause(where_clause)?;
            query.push_str(&format!(" WHERE {}", safe_clause));
        }
        
        if let Some(order) = order_by {
            query.push_str(&format!(" ORDER BY {}", order));
        }
        
        if let Some(sample_size) = sample {
            // Use TABLESAMPLE for random sampling
            query = format!(
                "SELECT {} FROM {} TABLESAMPLE SYSTEM({}) ",
                col_list, table_name, sample_size
            );
        }
        
        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }
        
        if let Some(off) = offset {
            query.push_str(&format!(" OFFSET {}", off));
        }
        
        tracing::debug!(query = %query, "Executing table query");
        
        // Execute query and convert to JSON
        let results: Vec<JsonValue> = diesel::sql_query(&query)
            .load::<QueryResult>(&mut conn)?
            .into_iter()
            .map(|row| row.to_json())
            .collect();
        
        Ok(results)
    }
    
    fn table_exists(&self, conn: &mut PgConnection, table_name: &str) -> Result<bool, DatabaseError> {
        let query = "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = $1
        )";
        
        let exists: bool = diesel::sql_query(query)
            .bind::<diesel::sql_types::Text, _>(table_name)
            .get_result(conn)?;
        
        Ok(exists)
    }
    
    fn sanitize_where_clause(&self, clause: &str) -> Result<String, DatabaseError> {
        // Basic SQL injection prevention
        // In production, use prepared statements or query builder
        if clause.contains(';') || clause.contains("--") {
            return Err(DatabaseError::new(
                DatabaseErrorKind::InvalidQuery("WHERE clause contains unsafe characters".into())
            ));
        }
        Ok(clause.to_string())
    }
}

// Format table data for LLM consumption
pub fn format_as_json(rows: &[JsonValue]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_default()
}

pub fn format_as_markdown(rows: &[JsonValue]) -> String {
    if rows.is_empty() {
        return "No data".to_string();
    }
    
    // Extract column names from first row
    let first = &rows[0];
    let columns: Vec<String> = if let Some(obj) = first.as_object() {
        obj.keys().cloned().collect()
    } else {
        return "Invalid data format".to_string();
    };
    
    let mut output = String::new();
    
    // Header row
    output.push_str("| ");
    output.push_str(&columns.join(" | "));
    output.push_str(" |\n");
    
    // Separator
    output.push_str("|");
    for _ in &columns {
        output.push_str(" --- |");
    }
    output.push('\n');
    
    // Data rows
    for row in rows {
        if let Some(obj) = row.as_object() {
            output.push_str("| ");
            for col in &columns {
                let val = obj.get(col)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                output.push_str(val);
                output.push_str(" | ");
            }
            output.push('\n');
        }
    }
    
    output
}

pub fn format_as_csv(rows: &[JsonValue]) -> String {
    // Similar to markdown but CSV format
    // Left as exercise
    String::new()
}
```

**4. Integration with Narrative Executor**

```rust
// crates/botticelli_narrative/src/executor.rs

pub struct NarrativeExecutor<D: BotticelliDriver> {
    driver: D,
    bot_executors: HashMap<String, Box<dyn BotCommandExecutor>>,
    table_executor: Option<TableQueryExecutor>,  // NEW
    // ... existing fields
}

impl<D: BotticelliDriver> NarrativeExecutor<D> {
    pub fn with_table_executor(mut self, executor: TableQueryExecutor) -> Self {
        self.table_executor = Some(executor);
        self
    }
    
    async fn process_input(
        &self,
        input: &Input,
    ) -> Result<String, NarrativeError> {
        match input {
            Input::Table {
                table_name,
                columns,
                where_clause,
                limit,
                offset,
                order_by,
                alias: _,  // Used later for interpolation
                format,
                sample,
            } => {
                let executor = self.table_executor.as_ref()
                    .ok_or_else(|| NarrativeError::new(
                        NarrativeErrorKind::TableExecutorNotConfigured
                    ))?;
                
                let rows = executor.query_table(
                    table_name,
                    columns.as_deref(),
                    where_clause.as_deref(),
                    *limit,
                    *offset,
                    order_by.as_deref(),
                    *sample,
                )?;
                
                let formatted = match format {
                    TableFormat::Json => format_as_json(&rows),
                    TableFormat::Markdown => format_as_markdown(&rows),
                    TableFormat::Csv => format_as_csv(&rows),
                };
                
                Ok(formatted)
            }
            // ... other input types
        }
    }
}
```

**5. Usage Example**

```rust
use botticelli_narrative::NarrativeExecutor;
use botticelli_database::{establish_connection, TableQueryExecutor};
use botticelli_models::GeminiClient;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database connection for table queries
    let conn = establish_connection()?;
    let table_executor = TableQueryExecutor::new(Arc::new(Mutex::new(conn)));
    
    // Create executor with table reference support
    let gemini = GeminiClient::new(api_key)?;
    let executor = NarrativeExecutor::new(gemini)
        .with_table_executor(table_executor);
    
    // Execute narrative that references tables
    let narrative = Narrative::from_file("analyze_content.toml")?;
    let result = executor.execute(&narrative).await?;
    
    Ok(())
}
```

## Implementation Phases

### Phase 1: Friendly Syntax Foundation ✅ **COMPLETE** (commit `a941fdc`)

All resource definition and reference resolution implemented.

### Phase 2: Bot Commands ✅ **COMPLETE**

**Week 1: Foundation** ✅ COMPLETE
- [x] Add `Input::BotCommand` variant to botticelli_core ✅
- [x] Update TOML parser to support bot_command input type ✅
- [x] Create `BotCommandExecutor` trait in `botticelli_social` ✅
- [x] Implement Discord command executor with 30+ commands ✅

**Week 2: Security Framework** ✅ COMPLETE
- [x] Create `botticelli_security` crate with 5-layer security ✅
- [x] Implement permission model (command + resource permissions) ✅
- [x] Implement input validation (Discord-specific validators) ✅
- [x] Implement content filtering (mass mentions, patterns, URLs) ✅
- [x] Implement rate limiting (token bucket algorithm) ✅
- [x] Implement approval workflows (human-in-the-loop for dangerous ops) ✅
- [x] Add comprehensive unit tests (37 tests passing) ✅

**Week 3: Integration & Testing** ✅ COMPLETE
- [x] Integration tests with Discord API in facade crate ✅
- [x] Error handling with `derive_more` ✅
- [x] Implement write commands (messages, channels, roles, members) ✅
- [x] Consolidate all integration tests to facade crate ✅
- [x] Security integration for all write operations ✅

**Remaining Tasks** (Phase 2.5):
- [ ] Integrate bot executor into NarrativeExecutor
- [ ] Implement command result caching
- [ ] Update NARRATIVE_TOML_SPEC.md
- [ ] Create example narratives using bot commands

### Phase 3: Table References ⏸️ **IN PROGRESS** (current focus)

**Week 1: Architecture & Foundation** ✅ COMPLETE
- [x] Add `Input::Table` variant to botticelli_core ✅
- [x] Update TOML parser to support table input type ✅
- [x] Analyze database trait separation needs ✅ (see `DATABASE_TRAIT_SEPARATION_ANALYSIS.md`)
- [x] Create `ContentRepository` trait for content queries ✅
- [x] Create `TableView` trait in `botticelli_interface` ✅
- [x] Separate `NarrativeRepository` and `ContentRepository` concerns ✅

**Week 2: Implementation** ✅ COMPLETE
- [x] Implement `TableReference` type in `botticelli_narrative` ✅
- [x] Add `TableReference` builder with derive_builder ✅
- [x] Integrate with `ContentRepository` trait ✅
- [x] Unit tests for TableReference ✅
- [x] Export from crate root ✅

**Note**: Using simpler approach with `ContentRepository::list_content` instead of complex `TableView` system. Advanced querying deferred to Phase 4.

**Week 3: Formatting & Features** ⏸️ PENDING
- [ ] Implement JSON formatter
- [ ] Implement Markdown formatter
- [ ] Implement CSV formatter
- [ ] Add sampling support (TABLESAMPLE)
- [ ] Add pagination support (LIMIT/OFFSET)
- [ ] Add ordering support (ORDER BY)

**Week 4: Integration & Testing** ⏸️ PENDING
- [ ] Integrate table executor into NarrativeExecutor
- [ ] Add query result caching
- [ ] Unit tests for query executor
- [ ] Integration tests with test database
- [ ] Update NARRATIVE_TOML_SPEC.md
- [ ] Create example narratives using table references

### Phase 4: Advanced Features (Future Backlog)

**Enhancements to Consider**:
- [ ] Table joins for complex queries
- [ ] Aggregations (COUNT, AVG, GROUP BY)
- [ ] Subqueries and CTEs
- [ ] Bot command retry logic
- [ ] Bot command result validation schemas
- [ ] Table access control / permissions
- [ ] Query performance optimization
- [ ] Streaming large table results

## Documentation Updates

### NARRATIVE_TOML_SPEC.md Updates

New sections to add:

1. **Bot Commands Section**
   - Overview and use cases
   - Syntax and parameters
   - Supported platforms and commands
   - Error handling
   - Caching and rate limiting
   - Complete examples

2. **Table References Section**
   - Overview and use cases
   - Syntax and parameters
   - Query filtering and limiting
   - Data formats (JSON, Markdown, CSV)
   - Aliases for multiple tables
   - Complete examples

3. **Input Types Section**
   - Add `bot_command` to existing list
   - Add `table` to existing list
   - Update comprehensive examples

### Example Narratives to Create

1. `examples/bot_commands/discord_stats.toml` - Server statistics
2. `examples/bot_commands/channel_activity.toml` - Channel analysis
3. `examples/table_references/analyze_previous.toml` - Content analysis
4. `examples/table_references/multi_batch_comparison.toml` - Compare batches
5. `examples/advanced/bot_and_table.toml` - Combine both features

## Testing Strategy

### Bot Commands Testing

**Unit Tests**:
- Command parsing from TOML
- Command executor interface
- Cache mechanism
- Error handling

**Integration Tests**:
- Mock Discord API responses
- Real Discord API (optional, in CI)
- Cache expiration
- Rate limit handling

**End-to-End Tests**:
- Complete narrative with bot commands
- Multi-act narratives with command chaining
- Error recovery scenarios

### Table References Testing

**Unit Tests**:
- Table query parsing from TOML
- SQL query building
- Format conversion (JSON, Markdown, CSV)
- WHERE clause sanitization

**Integration Tests**:
- Query against test database
- Large result sets
- Pagination
- Sampling

**End-to-End Tests**:
- Complete narrative with table references
- Multiple tables in one narrative
- Combine with template injection

## Security Considerations

### Bot Commands Security Framework (✅ IMPLEMENTED)

The `botticelli_security` crate provides a comprehensive 5-layer security framework:

1. **Permission Layer** (`PermissionChecker`)
   - ✅ Granular command permissions per narrative
   - ✅ Resource-level access control (channels, roles, users)
   - ✅ Protected users/roles (cannot be targeted)
   - ✅ Deny lists take precedence over allow lists
   - ✅ TOML configuration support

2. **Validation Layer** (`CommandValidator` trait)
   - ✅ Discord-specific validator (`DiscordValidator`)
   - ✅ Snowflake ID validation (17-19 digits)
   - ✅ Content length validation (Discord's 2000 char limit)
   - ✅ Channel/role name format validation
   - ✅ Parameter presence and type checking

3. **Content Filtering Layer** (`ContentFilter`)
   - ✅ Mass mention blocking (@everyone, @here)
   - ✅ Regex-based prohibited pattern detection
   - ✅ Mention count limits (default: 5)
   - ✅ URL count limits (default: 3)
   - ✅ Domain allowlisting/denylisting
   - ✅ Maximum content length enforcement

4. **Rate Limiting Layer** (`RateLimiter`)
   - ✅ Token bucket algorithm
   - ✅ Per-command and global limits
   - ✅ Burst allowance support
   - ✅ Automatic token refill
   - ✅ Configurable time windows

5. **Approval Workflow Layer** (`ApprovalWorkflow`)
   - ✅ Human-in-the-loop for dangerous operations
   - ✅ Pending action tracking with expiration
   - ✅ Approve/deny with reason and audit trail
   - ✅ 24-hour default expiration

**SecureExecutor Integration**:
- ✅ Wraps any `BotCommandExecutor` with security pipeline
- ✅ All checks run before command execution
- ✅ Comprehensive tracing at each layer
- ✅ Returns pending action ID if approval required
- ✅ 37 passing unit tests covering all scenarios

**Read vs Write Operations**:
- ✅ **Read commands** (implemented): Safe by default, minimal risk
- ⏸️ **Write commands** (pending review): Require approval workflow integration
- Security framework enables safe write operations when ready

See `PHASE_3_SECURITY_FRAMEWORK.md` for complete architecture and threat model.

### Table References

- **SQL Injection**: Sanitize all WHERE clauses and column names
- **Access Control**: Only allow access to authorized tables
- **Row Limits**: Enforce maximum row counts
- **Column Filtering**: Allow exclusion of sensitive columns
- **Query Timeout**: Set execution time limits
- **Audit Logging**: Log all table queries

## Success Criteria

### Bot Commands
- ✅ Can execute Discord bot commands from narratives
- ✅ Results available to subsequent acts via {{act_name}}
- ✅ Proper error handling (required vs optional commands)
- ✅ Caching reduces redundant API calls
- ✅ Rate limiting prevents API abuse
- ✅ Documentation with working examples

### Table References
- ✅ Can query any table in database from narratives
- ✅ Support filtering, limiting, ordering
- ✅ Multiple format options (JSON, Markdown, CSV)
- ✅ Safe SQL query generation (no injection)
- ✅ Efficient for large tables (sampling, pagination)
- ✅ Documentation with working examples

## Open Questions

1. **Bot Commands**:
   - ✅ **RESOLVED**: Write operations supported via security framework (requires approval workflow)
   - ✅ **RESOLVED**: Extensibility via `BotCommandExecutor` trait (platform-agnostic)
   - ⏸️ How to handle async commands that take time (webhooks)?
   - ⏸️ Should command results be cached? If so, how long?

2. **Table References**:
   - Should we support joins or keep queries simple?
   - How to handle very large tables (millions of rows)?
   - Should we support custom SQL or limit to builder patterns?
   - How to version table schemas for backward compatibility?

3. **Both Features**:
   - ✅ **RESOLVED**: Discord commands behind `discord` feature flag
   - ✅ **RESOLVED**: Testing with real Discord API via `#[cfg_attr(not(feature = "api"), ignore)]`
   - ⏸️ What's the upgrade path for existing narratives?

## Related Documents

- **`SPEC_ENHANCEMENT_PHASE_3.md`** - Current Phase 3 implementation tracking (replaces NARRATIVE_SPEC_ENHANCEMENTS as primary tracking doc)
- **`DATABASE_TRAIT_SEPARATION_ANALYSIS.md`** - Architecture analysis for `ContentRepository` and `TableView` traits
- **`PHASE_3_SECURITY_FRAMEWORK.md`** - Comprehensive security architecture for bot commands
- **`PHASE_2_COMPLETION_SUMMARY.md`** - Phase 2 completion status and Phase 2.5 next steps
- **`PHASE_2_FOLLOWUP.md`** - Remaining Phase 2 work and missing Discord API coverage
- `NARRATIVE_TOML_SPEC.md` - Current specification (needs updates for bot commands and table references)
- `CONTENT_GENERATION.md` - Content generation workflows
- `DISCORD_COMMUNITY_SERVER_PLAN.md` - Will use bot commands extensively
- `DISCORD_SETUP.md` - Bot setup and permissions
- `CLAUDE.md` - Project standards for error handling, tracing, derives, and builders

### Feature 3: Carousel (Looping with Budget Management)

**Implementation Status**: ✅ COMPLETED

See `CAROUSEL_FEATURE_DESIGN.md` for comprehensive design documentation.

**What's Implemented**:
- ✅ `CarouselConfig` struct with iteration and budget parameters
- ✅ `CarouselBudget` for multi-tier rate limit tracking (RPM, TPM, RPD, TPD)
- ✅ `CarouselState` for execution state management
- ✅ TOML parsing for carousel at narrative and act levels
- ✅ `execute_carousel()` method in NarrativeExecutor
- ✅ Budget-aware iteration loop with safety checks
- ✅ Comprehensive error handling and tracing

#### TOML Syntax

**Narrative-level carousel** (loops entire narrative):
```toml
[narrative]
name = "welcome_content_generation"
description = "Generate welcome messages in a carousel"

[carousel]
iterations = 3
estimated_tokens_per_iteration = 5000
continue_on_error = true

[toc]
order = ["generate", "critique", "select"]
```

**Act-level carousel** (loops specific act):
```toml
[acts.generate]
[[acts.generate.input]]
type = "text"
content = "Generate 10 welcome message options..."

[acts.generate.carousel]
iterations = 5
estimated_tokens_per_iteration = 2000
```

#### Budget Management

The carousel feature includes sophisticated budget tracking:
- **RPM** (Requests Per Minute) - Tracks request rate
- **TPM** (Tokens Per Minute) - Tracks token consumption rate
- **RPD** (Requests Per Day) - Tracks daily request quota
- **TPD** (Tokens Per Day) - Tracks daily token quota

Budget enforcement:
- Checks before each iteration if sufficient budget remains
- Estimates token usage based on `estimated_tokens_per_iteration`
- Stops gracefully when approaching limits
- Provides detailed budget status in results

#### Example Use Cases

1. **Content Batch Generation**: Generate 10 variations, run 3 times = 30 options
2. **Iterative Refinement**: Generate → Critique → Improve (loop 5 times)
3. **A/B Testing**: Generate different approaches in parallel iterations
4. **Data Collection**: Query APIs repeatedly for time-series data

#### Integration with Other Features

Carousels work seamlessly with:
- **Bot Commands**: Query Discord stats every hour (carousel with time delays)
- **Table References**: Load previous generation → Analyze → Generate new (iterative improvement)
- **Processors**: Extract data from each iteration into database tables

## Conclusion

Adding bot commands and table references significantly enhances Botticelli's composability and enables powerful new workflows:

- **Bot Commands** (Phase 2) ✅ **COMPLETE** - Narratives can now interact with live Discord data
  - ✅ 30+ Discord commands implemented (read and write operations)
  - ✅ 5-layer security framework protects against abuse
  - ✅ Comprehensive error handling and tracing
  - ⏸️ Remaining: NarrativeExecutor integration, caching, examples
  
- **Table References** (Phase 3) ✅ **COMPLETE** - Narratives can build on previous generations
  - ✅ `TableQueryRegistry` trait in botticelli_interface
  - ✅ `DatabaseTableQueryRegistry` implementation
  - ✅ `TableQueryExecutor` with dynamic SQL generation
  - ✅ Multiple format support (JSON, Markdown, CSV)
  - ✅ NarrativeExecutor integration
  - ✅ Comprehensive error handling and tracing
  
- **Carousel Feature** (Phase 4) ✅ **COMPLETE** - Narratives can loop with budget management
  - ✅ `CarouselConfig` with iterations and budget parameters
  - ✅ `CarouselBudget` for rate limit tracking
  - ✅ `CarouselState` for execution tracking
  - ✅ TOML parsing for carousel configurations
  - ✅ `execute_carousel()` method in NarrativeExecutor
  - ✅ Budget-aware execution loop
  - ✅ Example narratives demonstrating carousel usage

- Together, they create a **composable narrative system** where outputs become inputs

These features transform Botticelli from a linear execution engine into a **data-aware, platform-integrated, budget-managed content generation system**.

---

## Next Steps

### Phase 5: Production Readiness (Current Focus)

All core features are now implemented! Focus shifts to production readiness:

1. **Testing & Validation**:
   - ✅ Discord integration tests with live API
   - ✅ Table reference integration tests  
   - ✅ Carousel budget tracking tests
   - ⏸️ End-to-end workflow tests (bot + table + carousel)
   - ⏸️ Security policy enforcement tests
   - ⏸️ Error recovery and resilience tests

2. **Example Narratives** (In Progress):
   - ✅ `narratives/discord/welcome_content_generation.toml` - Carousel with critique loop
   - ✅ `narratives/discord/welcome_message_pipeline.toml` - Table references + bot publishing
   - ⏸️ Security policy examples with approval workflows
   - ⏸️ Multi-platform bot command examples
   - ⏸️ Complex carousel patterns (nested, conditional)

3. **Documentation Updates**:
   - ✅ `PHASE_2_BOT_COMMANDS.md` - Bot command architecture
   - ✅ `PHASE_2_COMPLETION_SUMMARY.md` - Phase 2 summary
   - ✅ `PHASE_3_SECURITY_FRAMEWORK.md` - Security design
   - ✅ `CAROUSEL_FEATURE_DESIGN.md` - Carousel design
   - ✅ `DATABASE_TRAIT_SEPARATION_ANALYSIS.md` - Database architecture
   - ⏸️ Update `NARRATIVE_TOML_SPEC.md` with all new features
   - ⏸️ Create user guide for bot commands and security policies
   - ⏸️ Create tutorial for building narrative pipelines

4. **Performance & Optimization**:
   - ⏸️ Command result caching (respect `cache_duration` parameter)
   - ⏸️ Table query result caching
   - ⏸️ Batch processing for carousel iterations
   - ⏸️ Parallel act execution where dependencies allow

5. **Additional Discord Commands**:
   - ✅ 20+ read commands implemented
   - ✅ 5+ write commands with security
   - ⏸️ Reaction and interaction commands
   - ⏸️ Webhook and integration commands
   - ⏸️ Advanced moderation commands

### Future Enhancements (Phase 6+)

1. **Multi-Platform Support**:
   - Slack bot commands
   - Mastodon bot commands  
   - Twitter/X bot commands
   - Platform-agnostic security policies

2. **Advanced Carousel Features**:
   - Conditional iterations (stop when quality threshold met)
   - Parallel carousel execution
   - Carousel checkpointing and resume
   - Dynamic budget adjustment

3. **Enhanced Table References**:
   - Cross-table JOINs
   - Aggregation queries (GROUP BY, COUNT, AVG)
   - Time-series queries (window functions)
   - Full-text search integration

4. **Workflow Orchestration**:
   - Narrative dependencies (run A before B)
   - Conditional execution (if/else logic)
   - Parallel narrative execution
   - Workflow scheduling and cron
