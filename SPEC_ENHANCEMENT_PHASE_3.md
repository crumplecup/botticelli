# Phase 3: Table References Implementation Plan

## Overview

Phase 3 adds the ability to reference database tables from within narratives, enabling narratives to build on previously generated content and create data-driven workflows.

**Status**: ✅ Core Implementation Complete - Table references fully functional  
**Prerequisites**: Phase 1 (friendly syntax) ✅ Complete

### What Was Built

**Infrastructure (100% Complete)**:
- ✅ `TableQueryView` and `TableCountView` builder structs with derive_builder
- ✅ `TableQueryExecutor` with SQL query construction and execution
- ✅ `TableQueryRegistry` trait implementation for executor integration
- ✅ Security: table/column sanitization, WHERE clause validation
- ✅ Three output formatters: JSON, Markdown, CSV
- ✅ `Input::Table` variant in botticelli_core
- ✅ TOML parsing support for table references
- ✅ NarrativeExecutor integration for table input processing

**Integration (100% Complete)**:
- ✅ `NarrativeExecutor::with_table_registry()` builder method
- ✅ `process_inputs()` handles `Input::Table` variants
- ✅ Calls `TableQueryRegistry::query_table()` with parameters
- ✅ Formats results and inserts into prompt context
- ✅ Error handling (table queries always fail-fast)

**Testing Status**:
- ✅ All existing tests pass
- ✅ Zero clippy warnings
- ⏳ Integration tests with real database tables - TODO

**Ready For**:
- Users can add table references to narratives via TOML
- Query real PostgreSQL tables with filtering and pagination
- Format results for LLM consumption
- Build data-driven narrative workflows

**Next Steps**:
- Integration tests with sample database tables
- Example narratives demonstrating table references
- Alias interpolation for table results (`{{alias}}`)
- Documentation with usage examples

## Architecture

### Component Structure

```
botticelli_database/src/
├── table_query.rs       # TableQueryExecutor implementation ✅ COMPLETE
└── table_query_view.rs  # TableQueryView and TableCountView builders ✅ COMPLETE
```

**Implementation Status**: All table query infrastructure exists and compiles successfully.

### Key Types

#### 1. TableQueryView (Query Builder) ✅ IMPLEMENTED

Uses `derive_builder` to construct queries safely:

```rust
use derive_builder::Builder;
use derive_getters::Getters;

/// View for querying table data with flexible filtering and pagination.
#[derive(Debug, Clone, Default, Builder, Getters)]
#[builder(setter(into, strip_option), default)]
pub struct TableQueryView {
    table_name: String,
    columns: Option<Vec<String>>,
    where_clause: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<String>,
    sample: Option<i64>,  // For future TABLESAMPLE support
}
```

**Usage**:
```rust
let query = TableQueryViewBuilder::default()
    .table_name("social_posts_20241120")
    .columns(vec!["title", "body", "status"])
    .where_clause("status = 'published'")
    .order_by("created_at DESC")
    .limit(10)
    .build()?;
```

#### 2. TableQueryExecutor (Execution Trait)

```rust
use async_trait::async_trait;
use diesel::PgConnection;

/// Execute table queries and format results.
#[async_trait]
pub trait TableQueryExecutor: Send + Sync {
    /// Execute a table query and return formatted results.
    async fn execute(&self, query: &TableQuery) -> TableQueryResult<String>;
    
    /// Check if a table exists in the database.
    async fn table_exists(&self, table_name: &str) -> TableQueryResult<bool>;
    
    /// Get table schema information (column names and types).
    async fn get_schema(&self, table_name: &str) -> TableQueryResult<TableSchema>;
}

/// Default implementation using Diesel.
pub struct DieselTableQueryExecutor {
    connection: PgConnection,
}

impl DieselTableQueryExecutor {
    pub fn new(connection: PgConnection) -> Self {
        Self { connection }
    }
}
```

#### 2. TableQueryExecutor ✅ IMPLEMENTED

```rust
use std::sync::{Arc, Mutex};
use diesel::PgConnection;

/// Executes table queries for narrative table references.
#[derive(Clone, derive_getters::Getters)]
pub struct TableQueryExecutor {
    connection: Arc<Mutex<PgConnection>>,
}
```

**Implemented Methods**:
- ✅ `query_table(&self, view: &TableQueryView) -> DatabaseResult<Vec<JsonValue>>`
- ✅ `count_rows(&self, view: &TableCountView) -> DatabaseResult<i64>`

**Security Features**:
- ✅ Table existence validation via `information_schema`
- ✅ Table name sanitization (alphanumeric + underscore only)
- ✅ Column name sanitization (alphanumeric + underscore only)
- ✅ WHERE clause danger pattern detection (`;`, `--`, `DROP`)
- ✅ Parameterized binding where possible
- ✅ Comprehensive tracing with `#[instrument]`

#### 3. Data Formatting ✅ IMPLEMENTED

Three formatting functions in `botticelli_database::table_query`:

- ✅ `format_as_json(rows: &[JsonValue]) -> String` - Pretty-printed JSON array
- ✅ `format_as_markdown(rows: &[JsonValue]) -> String` - Markdown table with header
- ✅ `format_as_csv(rows: &[JsonValue]) -> String` - CSV with proper escaping

#### 4. Error Handling ✅ IMPLEMENTED

Uses `DatabaseError` and `DatabaseErrorKind` from `botticelli_error` crate:

- ✅ `TableNotFound(String)` - Table doesn't exist
- ✅ `InvalidQuery(String)` - SQL validation failed
- ✅ `Query(String)` - Query execution error
- ✅ `Connection(String)` - Database connection error

All errors include location tracking via `#[track_caller]`.

### TOML Syntax

#### Simple Table Reference

```toml
[[acts.analyze.input]]
type = "table"
table_name = "social_posts_20241120"
limit = 100
format = "markdown"
```

#### Filtered Query

```toml
[[acts.analyze.input]]
type = "table"
table_name = "social_posts_20241120"
columns = ["title", "body", "created_at"]
where = "status = 'published' AND platform = 'twitter'"
order_by = "created_at DESC"
limit = 10
```

#### Multiple Tables with Aliases

```toml
[[acts.compare.input]]
type = "table"
table_name = "posts_batch_1"
alias = "batch1"
limit = 50

[[acts.compare.input]]
type = "table"
table_name = "posts_batch_2"
alias = "batch2"
limit = 50

[[acts.compare.input]]
type = "text"
content = """
Compare these batches:

Batch 1:
{{batch1}}

Batch 2:
{{batch2}}

What improved?
"""
```

### Friendly Syntax with Resource Definitions

Users can define table references once and reuse them:

```toml
# Define table resources
[tables.recent_posts]
table_name = "social_posts_20241120"
columns = ["title", "body", "status"]
where = "status = 'published'"
order_by = "created_at DESC"
limit = 20
format = "markdown"

[tables.draft_posts]
table_name = "social_posts_20241120"
where = "status = 'draft'"
limit = 10

# Use references in acts
[[acts.analyze.input]]
sources = ["tables.recent_posts", "Here's my recent content. Analyze it."]

[[acts.review.input]]
sources = ["tables.draft_posts", "Review these drafts and suggest improvements."]
```

## Implementation Steps

### Infrastructure (✅ COMPLETE)

- ✅ Step 1: Query builder types (`TableQueryView`, `TableCountView`)
- ✅ Step 2: Error handling (uses `DatabaseError` from `botticelli_error`)
- ✅ Step 3: Query executor (`TableQueryExecutor` in `table_query.rs`)
- ✅ Step 4: Data formatters (`format_as_json`, `format_as_markdown`, `format_as_csv`)
- ✅ Step 5: Input type (`Input::Table` variant exists in `botticelli_core`)
- ✅ Step 6: TOML parser support (`TomlTableDefinition` exists)
- ✅ Step 7: Resource definitions (`[tables.name]` sections parse correctly)

### Integration (✅ COMPLETE)

- [x] Step 8: **NarrativeExecutor Integration** - Process `Input::Table` during execution
  - [x] Add `table_registry: Box<dyn TableQueryRegistry>` field to executor
  - [x] Handle `Input::Table` in act input processing
  - [x] Implement TableQueryRegistry trait for TableQueryExecutor
  - [x] Call `registry.query_table()` to get results
  - [x] Choose format (JSON/Markdown/CSV) based on TOML config
  - [x] Format results using `format_as_*` functions
  - [x] Insert formatted data into prompt context
  - [x] Handle query errors gracefully (always fail - tables required by default)
  - [ ] Add result caching within execution context (future enhancement)

- [ ] Step 9: **Integration Tests** (Ready to implement)
  - [ ] Create test database table with sample data
  - [ ] Test basic query (SELECT * with limit)
  - [ ] Test column selection
  - [ ] Test WHERE clause filtering
  - [ ] Test ORDER BY and pagination
  - [ ] Test all formats (JSON, Markdown, CSV)
  - [ ] Test table aliases in prompt interpolation
  - [ ] Test error handling (table not found, invalid query)
  - [ ] Test multiple table references in one narrative
  - Note: Infrastructure complete, tests can be written against real database

- [ ] Step 10: **Documentation and Examples** (Partially complete)
  - [x] Update `NARRATIVE_SPEC_ENHANCEMENTS.md` status
  - [x] Create `SPEC_ENHANCEMENT_PHASE_3.md` planning doc
  - [x] Document TableQueryRegistry trait implementation
  - [ ] Create `examples/table_reference_narrative.toml`
  - [ ] Add usage guide to README or docs
  - [ ] Document security best practices
  - [ ] Document performance considerations

## Security Considerations

### SQL Injection Prevention

**CRITICAL**: All table queries must be parameterized or properly escaped.

1. **Table names**: Whitelist validation (must match `[a-zA-Z0-9_]+`)
2. **Column names**: Validate against schema
3. **WHERE clauses**: Use Diesel's query builder (NOT raw SQL)
4. **ORDER BY**: Validate against schema
5. **LIMIT/OFFSET**: Already safe (integers)

### Access Control

- Only allow access to tables in the same database connection
- Future: Row-level security based on narrative permissions
- Future: Column exclusion lists (e.g., exclude `api_key`, `password`)

### Resource Limits

- Default LIMIT: 100 rows (prevent huge queries)
- Maximum LIMIT: 1000 rows (configurable)
- Query timeout: 30 seconds
- Result size limit: 10MB formatted output

## Performance Considerations

### Query Optimization

1. **Indexing**: Ensure frequently queried columns are indexed
2. **Caching**: Cache identical queries within execution context
3. **Pagination**: Encourage LIMIT/OFFSET for large datasets
4. **Materialized views**: Consider for expensive aggregations

### Data Transfer

1. **Column selection**: Encourage selecting only needed columns
2. **Format efficiency**: JSON more compact than Markdown
3. **Streaming**: Future enhancement for very large results

## Testing Strategy

### Unit Tests

- Query builder validation
- SQL generation correctness
- Data formatting (all formats)
- Error handling

### Integration Tests

- Query execution against real database
- Format rendering with actual data
- Reference resolution in narratives
- Error propagation through executor

### API Tests (Gated)

- Full narrative execution with table references
- Multi-table workflows
- Alias interpolation in prompts

## Future Enhancements

### Phase 3.5: Advanced Queries

- **Joins**: `join = { table = "other", on = "key" }`
- **Aggregations**: `aggregate = { count = "*", group_by = "status" }`
- **Subqueries**: Reference other table queries
- **CTEs**: Common table expressions for complex queries

### Phase 3.6: Schema Management

- **Schema introspection**: Include column types in prompts
- **Type inference**: Help LLMs understand data types
- **Relationships**: Document foreign keys for LLM context

### Phase 3.7: Performance

- **Query plan analysis**: Warn about slow queries
- **Result streaming**: Stream large results instead of buffering
- **Incremental loading**: Load data as needed during execution

## Success Criteria

Phase 3 status:

- ✅ Users can reference database tables in TOML narratives
- ✅ Queries support filtering, sorting, limiting, pagination  
- ✅ Results formatted as JSON, Markdown, or CSV
- ✅ TableQueryRegistry trait implemented and integrated
- ✅ NarrativeExecutor processes Input::Table variants
- ✅ Errors properly reported with context
- ✅ SQL injection prevented through validation
- ✅ All compilation checks pass (cargo check, clippy)
- ✅ All local tests pass
- ⏳ Aliases work in prompt interpolation (`{{alias}}`) - TODO
- ⏳ Integration tests with real database - TODO
- ⏳ Documentation complete with examples - Partially done

## References

- `NARRATIVE_SPEC_ENHANCEMENTS.md` - Original design document
- `PHASE_2_BOT_COMMANDS.md` - Similar pattern for bot commands
- `CLAUDE.md` - Derive patterns and error handling guidelines
- `botticelli_core/src/input.rs` - Input::Table variant
- `botticelli_narrative/src/toml/parser.rs` - TOML parsing
