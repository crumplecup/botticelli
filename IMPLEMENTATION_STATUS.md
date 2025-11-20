# Implementation Status: Friendly Syntax & Extensions

## Phase 1: Friendly Syntax Foundation ✅ COMPLETE

**Status**: Implemented and tested

**What's Working**:
- ✅ Resource definition sections: `[bots]`, `[tables]`, `[media]`
- ✅ Resource reference resolution: `"bots.name"`, `"tables.name"`, `"media.name"`
- ✅ Array syntax for multi-input acts: `["ref1", "ref2", "text"]`
- ✅ MIME type inference from file extensions
- ✅ Media type detection (image/audio/video/document)
- ✅ New Input variants: `BotCommand`, `Table`, `TableFormat`
- ✅ TOML parser updates with full reference resolution
- ✅ 100% backward compatibility maintained

**Example Working**:
```toml
[media.logo]
file = "./logo.png"

[acts]
analyze = ["media.logo", "Describe this logo"]
```

**Commit**: `a941fdc` - feat(narrative): implement Phase 1 - friendly syntax foundation

---

## Phase 2: Bot Command Execution ⏸️ READY FOR IMPLEMENTATION

**Status**: Input type exists, needs executor integration

**What's Already Done**:
- ✅ `Input::BotCommand` variant in botticelli_core
- ✅ `TomlBotDefinition` for parsing `[bots.name]` sections
- ✅ Reference resolution from `"bots.name"` → `Input::BotCommand`
- ✅ TOML syntax fully designed and documented

**What's Needed**:
1. **BotCommandExecutor Trait** (`botticelli_social` or new crate)
   ```rust
   pub trait BotCommandExecutor: Send + Sync {
       async fn execute(
           &self,
           command: &str,
           args: &HashMap<String, JsonValue>,
       ) -> Result<JsonValue, BotCommandError>;
       
       fn supports_command(&self, command: &str) -> bool;
   }
   ```

2. **DiscordCommandExecutor Implementation**
   - Requires Discord client from `botticelli_social`
   - Implement commands: `server.get_stats`, `channels.list`, etc.
   - Add command result caching
   - Rate limiting support

3. **NarrativeExecutor Integration**
   - Add `bot_executors: HashMap<String, Box<dyn BotCommandExecutor>>`
   - Add `with_bot_executor()` builder method
   - Handle `Input::BotCommand` in `process_input()`
   - Convert bot command result (JSON) to text for LLM

4. **Error Handling**
   - Create `BotCommandError` in botticelli_error
   - Handle `required` flag (halt vs continue on failure)
   - Provide helpful error messages in context

**Prerequisites**:
- Discord bot client must be functional
- Bot permissions properly configured
- Discord API integration working

**Estimated Time**: 2-3 weeks (depends on Discord client maturity)

---

## Phase 3: Table References ⏸️ READY FOR IMPLEMENTATION

**Status**: Input type exists, needs executor integration

**What's Already Done**:
- ✅ `Input::Table` variant in botticelli_core
- ✅ `TableFormat` enum (Json, Markdown, Csv)
- ✅ `TomlTableDefinition` for parsing `[tables.name]` sections
- ✅ Reference resolution from `"tables.name"` → `Input::Table`
- ✅ TOML syntax fully designed and documented

**What's Needed**:
1. **TableQueryExecutor** (`botticelli_database`)
   ```rust
   pub struct TableQueryExecutor {
       connection: Arc<Mutex<PgConnection>>,
   }
   
   impl TableQueryExecutor {
       pub fn query_table(
           &self,
           table_name: &str,
           columns: Option<&[String]>,
           where_clause: Option<&str>,
           limit: Option<u32>,
           // ...
       ) -> Result<Vec<JsonValue>, DatabaseError>;
   }
   ```

2. **SQL Query Building**
   - Build SELECT queries from Input::Table parameters
   - WHERE clause sanitization (SQL injection prevention)
   - Support for LIMIT, OFFSET, ORDER BY
   - Random sampling with TABLESAMPLE

3. **Data Formatting Functions**
   - `format_as_json()` - Pretty JSON array of objects
   - `format_as_markdown()` - Markdown table for LLM readability
   - `format_as_csv()` - CSV format

4. **NarrativeExecutor Integration**
   - Add `table_executor: Option<TableQueryExecutor>`
   - Add `with_table_executor()` builder method
   - Handle `Input::Table` in `process_input()`
   - Convert table data to formatted string for LLM

5. **Security & Performance**
   - SQL injection protection
   - Row count limits (prevent context overflow)
   - Query result caching
   - Table existence validation

**Prerequisites**:
- PostgreSQL database connection working
- Diesel ORM setup complete
- Database schema reflection working

**Estimated Time**: 2-3 weeks (depends on database infrastructure)

---

## Testing Strategy

### Phase 1 (Current - Friendly Syntax)
- ✅ Parse narratives with resource definitions
- ✅ Resolve references to resources
- ✅ MIME type inference
- ✅ Array syntax for multi-input acts
- ✅ Backward compatibility with existing narratives

### Phase 2 (Bot Commands)
- Parse bot command definitions from TOML
- Execute mock bot commands
- Handle bot command failures gracefully
- Cache bot command results
- Integrate with narrative executor end-to-end

### Phase 3 (Table References)
- Parse table definitions from TOML
- Query test database tables
- Format results as JSON, Markdown, CSV
- Handle large result sets (sampling, pagination)
- SQL injection protection tests

---

## Example: Full Friendly Syntax (When Complete)

```toml
[narrative]
name = "comprehensive_analysis"
description = "Analyze Discord server and historical data"

[toc]
order = ["fetch_stats", "load_data", "analyze", "recommend"]

# Bot commands
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

# Table queries
[tables.recent_posts]
table_name = "approved_posts_2024"
where = "status = 'approved'"
limit = 50
format = "markdown"

# Media files
[media.logo]
file = "./logo.png"

# Acts using friendly syntax
[acts]
fetch_stats = "bots.get_stats"
load_data = "tables.recent_posts"

analyze = [
    "media.logo",
    "bots.get_stats",
    "tables.recent_posts",
    "Comprehensive analysis of branding, server activity, and content"
]

recommend = """
Based on {{analyze}}, create a strategic content plan.
"""
```

---

## Next Steps

### Immediate (Phase 1 complete):
1. ✅ Update NARRATIVE_TOML_SPEC.md with implementation notes
2. ✅ Create example narratives using friendly syntax
3. Push to GitHub

### Short-term (Phases 2 & 3):
- Implement bot command executor when Discord client ready
- Implement table query executor when database infrastructure ready
- Create integration tests for end-to-end workflows

### Long-term (Future enhancements):
- Support for joins in table queries
- Aggregations (COUNT, AVG, GROUP BY)
- Bot command retry logic
- Query performance optimization
- Streaming large table results

---

## Documentation Updates Needed

- [x] NARRATIVE_TOML_SPEC.md - Updated with friendly syntax
- [x] FRIENDLY_SYNTAX_DESIGN.md - Complete design document
- [x] NARRATIVE_SPEC_ENHANCEMENTS.md - Bot commands and table references
- [ ] Add implementation guide for bot executors
- [ ] Add implementation guide for table executors
- [ ] Update examples with working friendly syntax

---

## Breaking Changes

None! Friendly syntax is 100% opt-in and backward compatible.

---

## Performance Considerations

### Bot Commands:
- Cache results with configurable TTL
- Respect platform rate limits
- Batch commands where possible

### Table References:
- Enforce row limits to prevent context overflow
- Use database indexes for filtered queries
- Consider materialized views for expensive queries
- Cache query results within execution

---

## Security Considerations

### Bot Commands:
- Read-only operations only
- Validate guild_id/channel_id ownership
- Run with minimal bot permissions
- Audit log all command executions

### Table References:
- SQL injection protection (parameterized queries)
- Row-level access control
- Exclude sensitive columns
- Query timeout limits
- Audit log all table queries
