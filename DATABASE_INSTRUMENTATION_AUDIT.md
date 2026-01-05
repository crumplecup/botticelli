# Database Crate Instrumentation Audit

## Summary

The _database crate has significant gaps in instrumentation coverage. Many public functions lack `#[instrument]` attributes, making debugging and performance monitoring difficult.

## Missing Instrumentation by Module

### connection.rs (Critical - 3 functions)
- [ ] `establish_connection()` - Database connection creation
- [ ] `create_pool()` - Connection pool creation
- [ ] `create_pool_from_url(database_url)` - Pool with custom URL

### content_management.rs (High Priority - 9 functions)
- [ ] `list_content(conn, table_name, limit)` - Query operations
- [ ] `get_content_by_id(conn, table_name, id)` - Single record fetch
- [ ] `update_content_metadata(conn, table_name, id, metadata)` - Metadata updates
- [ ] `update_review_status(conn, table_name, id, status)` - Status changes
- [ ] `delete_content(conn, table_name, id)` - Delete operations
- [ ] `pull_and_delete(conn, table_name, id)` - Fetch then delete
- [ ] `promote_content(conn, from_table, to_table, id)` - Cross-table moves
- [ ] `insert_content(conn, table_name, row)` - Insert operations
- [ ] `query_content(conn, table_name, query)` - Query operations

### narrative_conversions.rs (Medium Priority - 7 functions)
- [ ] `status_to_string(status)` - Enum conversion
- [ ] `string_to_status(s)` - String parsing
- [ ] `execution_to_new_row(execution)` - Model to DB conversion
- [ ] `act_execution_to_new_row(act, execution_id)` - Act conversion
- [ ] `input_to_new_row(input, execution_id, position)` - Input conversion
- [ ] `rows_to_act_execution(rows)` - DB to model conversion
- [ ] `rows_to_narrative_execution(execution_row, acts, inputs)` - Full reconstruction

### content_generation_repository.rs (Low Priority - Constructor)
- [ ] `new(conn)` - Simple constructor (skip per standards)

### content_repository.rs (Low Priority - Constructors/Getters)
- [ ] `new(pool)` - Constructor (skip)
- [ ] `pool()` - Getter (skip)

### db_operations.rs (Low Priority - Constructor)
- [ ] `new(pool)` - Constructor (skip)

### models.rs (Low Priority - Constructors/Converters)
- [ ] `new(...)` - Constructor (skip)
- [ ] `error(...)` - Constructor (skip)
- [ ] `to_serializable(&self)` - Simple conversion (consider adding)

### narrative_repository.rs (Low Priority - Constructors)
- [ ] `new(...)` - Constructor (skip)
- [ ] `from_arc(...)` - Constructor (skip)

### registry_impl.rs (Low Priority - Constructors)
- [ ] `new(...)` - Constructors (skip)

## Priority Actions

### Phase 1: Critical Infrastructure (connection.rs)
These functions are called at startup and need instrumentation for deployment debugging:
1. Add `#[instrument]` to all connection/pool creation functions
2. Add `skip(database_url)` for URL parameters (sensitive data)
3. Log connection success/failure events

### Phase 2: High Traffic Operations (content_management.rs)
These are called frequently and need performance tracking:
1. Add `#[instrument(skip(conn), fields(table_name, id, limit))]` pattern
2. Skip `conn` (large struct), include context fields
3. Add debug events for SQL operations
4. Log row counts in results

### Phase 3: Data Conversions (narrative_conversions.rs)
Useful for debugging serialization issues:
1. Add `#[instrument]` to conversion functions
2. Skip large data structures, include IDs
3. Log conversion failures with context

## Instrumentation Patterns

### Database Query Pattern
```rust
#[instrument(skip(conn), fields(table_name, id))]
pub fn get_content_by_id(
    conn: &mut PgConnection,
    table_name: &str,
    id: i64,
) -> DatabaseResult<ContentRow> {
    debug!("Fetching content");
    // ... query ...
    debug!(found = result.is_ok(), "Query complete");
    result
}
```

### Pool/Connection Pattern
```rust
#[instrument(skip(database_url))]
pub fn create_pool_from_url(database_url: &str) -> DatabaseResult<DbPool> {
    info!("Creating connection pool");
    // ... create pool ...
    info!("Pool created successfully");
    Ok(pool)
}
```

### Conversion Pattern
```rust
#[instrument(skip(execution), fields(execution_id = execution.id()))]
pub fn execution_to_new_row(
    execution: &NarrativeExecution,
) -> DatabaseResult<NewExecutionRow> {
    debug!("Converting execution to row");
    // ... conversion ...
    Ok(row)
}
```

## Estimated Impact

- **Without instrumentation**: 500-line logs to find DB issue, 10+ minutes
- **With instrumentation**: 5-line trace shows exact query/table, instant fix

## Notes

- Constructors and simple getters typically skip instrumentation (see CLAUDE.md)
- Skip sensitive data (URLs, credentials) in span fields
- Include context IDs and table names for debugging
- Log SQL at debug level for query troubleshooting
