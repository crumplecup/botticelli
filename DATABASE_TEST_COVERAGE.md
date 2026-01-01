# Database Test Coverage Tracker

## Current Status

Test file: `tests/database_repository_test.rs`

### Completed ✅

- [x] Basic connection test
- [x] PostgresContentRepository creation

### Repository Operations - Content

- [ ] **Create operations**
  - [ ] Insert new content row
  - [ ] Verify auto-generated fields (id, created_at, updated_at)
  - [ ] Test with various content types
  
- [ ] **Read operations**
  - [ ] Get by ID
  - [ ] Get by hash
  - [ ] List with pagination
  - [ ] Filter by content_type
  - [ ] Handle not found cases
  
- [ ] **Update operations**
  - [ ] Update metadata
  - [ ] Verify updated_at changes
  - [ ] Handle non-existent ID
  
- [ ] **Delete operations**
  - [ ] Delete by ID
  - [ ] Verify cascade behavior
  - [ ] Handle non-existent ID

### Repository Operations - Content Generation

- [ ] **ContentGenerationRepository trait**
  - [ ] Create generation record
  - [ ] Link to content
  - [ ] Store prompt/parameters
  - [ ] Track generation metadata
  - [ ] Query by content_id
  - [ ] Query by generation parameters

### Repository Operations - Narrative

- [ ] **NarrativeRepository trait**
  - [ ] Create narrative
  - [ ] Read narrative by ID
  - [ ] Update narrative state
  - [ ] List narratives
  - [ ] Delete narrative
  - [ ] Associate with content

### Schema and Migrations

- [ ] **Migration testing**
  - [ ] Verify all tables exist
  - [ ] Check column types
  - [ ] Verify constraints (PK, FK, UNIQUE, NOT NULL)
  - [ ] Test indexes exist
  
- [ ] **Schema validation**
  - [ ] Diesel schema matches database
  - [ ] All relationships mapped correctly

### Error Handling

- [ ] **Database errors**
  - [ ] Connection failures
  - [ ] Query syntax errors
  - [ ] Constraint violations (unique, FK, etc.)
  - [ ] Transaction rollback
  - [ ] Serialization errors (JSON columns)

### Edge Cases

- [ ] **Data boundaries**
  - [ ] Empty strings
  - [ ] NULL values in nullable columns
  - [ ] Very long text fields
  - [ ] Special characters in text
  - [ ] Large JSON payloads
  
- [ ] **Concurrency**
  - [ ] Simultaneous inserts
  - [ ] Update conflicts
  - [ ] Transaction isolation

### Performance

- [ ] **Query optimization**
  - [ ] Index usage verification
  - [ ] Pagination performance
  - [ ] Bulk operations
  - [ ] Join performance

## Test Setup Requirements

- [x] Test database connection via env vars
- [x] Feature flag: `postgres` (default enabled)
- [ ] Test fixtures/factories for common objects
- [ ] Transaction rollback for test isolation
- [ ] Cleanup between tests

## Notes

- Using real PostgreSQL instance for development/testing
- Connection configured via `DATABASE_URL` env var
- Tests should be idempotent and isolated
- Focus on testing trait implementations, not internal helpers
