# Tracing Instrumentation - botticelli_database

## Overview

This document describes the tracing instrumentation added to botticelli_database to provide comprehensive observability of database operations.

## Instrumentation Strategy

All public functions are instrumented with appropriate tracing spans and events:

### Span Naming Convention

Spans follow the pattern: `module_name.function_name`

Examples:
- `database.establish_connection`
- `content_management.list_content`
- `schema_reflection.reflect_table_schema`

### Field Selection

Spans include relevant fields while skipping large or sensitive data:

- **Always include**: identifiers (table names, IDs), counts, status values
- **Skip**: database connections (`conn`), large data structures (JSON values, schemas)
- **Format with %**: strings and display types
- **Format with ?**: debug types

## Instrumented Modules

### connection.rs

**Functions:**
- `establish_connection()` - Database connection establishment
  - Span: `database.establish_connection`
  - Events: `debug` on success, `error` on failure

### content_management.rs

**Functions:**
- `list_content()` - Query content tables
  - Span: `content_management.list_content`
  - Fields: `table`, `limit`
  - Events: `warn` for missing columns

- `get_content_by_id()` - Fetch specific content item
  - Span: `content_management.get_content_by_id`
  - Fields: `table`, `id`

- `update_content_metadata()` - Update tags and rating
  - Span: `content_management.update_content_metadata`
  - Fields: `table`, `id`
  - Skips: `tags` (potentially large array)

- `update_review_status()` - Change review status
  - Span: `content_management.update_review_status`
  - Fields: `table`, `id`, `status`
  - Events: `debug` for SQL queries

- `delete_content()` - Remove content item
  - Span: `content_management.delete_content`
  - Fields: `table`, `id`

- `promote_content()` - Copy to production table
  - Span: `content_management.promote_content`
  - Fields: `source`, `target`, `id`
  - Events: `info` on promotion start

### content_generation_repository.rs

**Functions:**
- `start_generation()` - Begin generation tracking
  - Events: `debug` on start, `error` on failure
  - Fields: `table`, `narrative`

- `complete_generation()` - Finish generation tracking
  - Events: `debug` on completion, `error` on failure
  - Fields: `table`, `status`

### narrative_conversions.rs

**Functions:**
- `rows_to_act_execution()` - Reconstruct ActExecution from DB rows
  - Span: `narrative_conversions.rows_to_act_execution`
  - Fields: `act_name`, `input_count`

- `rows_to_narrative_execution()` - Reconstruct NarrativeExecution
  - Span: `narrative_conversions.rows_to_narrative_execution`
  - Fields: `narrative`, `act_count`

### schema_docs.rs

**Functions:**
- `generate_schema_prompt()` - Create LLM prompt from schema
  - Span: `schema_docs.generate_schema_prompt`
  - Fields: `table`, `column_count`

- `assemble_prompt()` - Complete prompt assembly
  - Span: `schema_docs.assemble_prompt`
  - Fields: `template`
  - Skips: `user_content_focus` (potentially large text)

### schema_inference.rs

**Functions:**
- `infer_schema()` - Infer schema from JSON
  - Span: `schema_inference.infer_schema`
  - Events: `debug` for JSON type, `error` for invalid input, `info` for completion
  - Skips: `json` parameter (large data)

- `create_inferred_table()` - Create table from inferred schema
  - Span: `schema_inference.create_inferred_table`
  - Fields: `table`, `field_count`
  - Events: `info` with comprehensive details

### schema_reflection.rs

**Functions:**
- `reflect_table_schema()` - Query table structure
  - Span: `schema_reflection.reflect_table_schema`
  - Fields: `table`
  - Events: `error` for missing tables

- `table_exists()` - Check table existence
  - Span: `schema_reflection.table_exists`
  - Fields: `table`

- `create_content_table()` - Create generation table
  - Span: `schema_reflection.create_content_table`
  - Fields: `table`, `template`
  - Events: `info` for creation, `info` for existing tables

## Log Level Guidelines

### `trace!`
- Not currently used (reserved for very detailed debugging)

### `debug!`
- Function entry/exit (via `#[instrument]`)
- SQL query details
- Connection establishment
- Schema reflection queries

### `info!`
- Table creation
- Content promotion
- Generation tracking milestones
- Schema inference completion

### `warn!`
- Missing optional columns
- Feature availability issues
- Recoverable errors

### `error!`
- Connection failures
- Query execution failures
- Schema inference errors
- Invalid input validation

## Usage Examples

### Viewing Traces

With `tracing-subscriber` configured:

```rust
// In your application initialization:
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

tracing_subscriber::registry()
    .with(fmt::layer())
    .with(EnvFilter::from_default_env())
    .init();

// Set environment variable:
// RUST_LOG=botticelli_database=debug
```

### Example Output

```
2025-11-19T18:30:00.123Z DEBUG database.establish_connection: Connecting to PostgreSQL database
2025-11-19T18:30:00.234Z INFO content_management.list_content{table="discord_guilds" limit=10}: Listed 5 content items
2025-11-19T18:30:01.123Z INFO schema_inference.infer_schema: Schema inference complete field_count=8
```

## Benefits

1. **Debugging**: Trace request flow through database operations
2. **Performance**: Identify slow queries and operations
3. **Monitoring**: Track content generation and table creation
4. **Error Context**: Rich error information with operation context
5. **Audit Trail**: Complete record of database modifications

## Best Practices

1. **Always skip `conn` parameter** - Database connections are not Debug/Display
2. **Skip large data structures** - Use counts instead (e.g., `input_count` not `inputs`)
3. **Use structured fields** - `table = %table_name` not concatenated strings
4. **Log errors with context** - Include operation details in error events
5. **Keep spans focused** - One span per logical operation

## Future Enhancements

Potential improvements for future iterations:

1. Add `trace!` level events for loop iterations and detailed state changes
2. Add metrics collection (operation counts, duration histograms)
3. Add correlation IDs for tracking related operations
4. Add sampling for high-volume operations
5. Add custom span extensions for query analysis

---

**Last Updated:** 2025-11-19
**Status:** ✅ Complete
