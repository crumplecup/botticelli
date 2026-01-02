# Database Test Coverage Tracker

## Current Status

Test file: `tests/database_repository_test.rs`

### Completed ✅

- [x] Basic connection test
- [x] PostgresContentRepository creation
- [x] Test infrastructure with transaction isolation
- [x] Query operations (existing tables)
- [x] Error handling for nonexistent tables

### In Progress 🚧

- [ ] **Dynamic table creation** (blocked - needs implementation)
  - Tests written but `create_content_table` not implemented
  - Required for: insert tests, content CRUD tests

### Repository Operations - Content

### Repository Operations - Content

- [x] **Query operations** (existing tables)
  - [x] Query with limit
  - [x] Query with different limits
  - [x] Handle empty results
  - [x] Handle nonexistent tables (error case)
  
- [ ] **Create operations** (blocked - needs `create_content_table` implementation)
  - [x] Test written for table creation
  - [ ] Implementation needed
  - [ ] Insert new content row
  - [ ] Verify auto-generated fields (id, created_at, updated_at)
  - [ ] Test with various content types
  
- [ ] **Read operations** (blocked - needs dynamic tables)
  - [x] Query existing tables
  - [ ] Get by ID
  - [ ] Get by hash
  - [ ] Filter by content_type
  
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

## Testing Strategy

### Integration Tests (Real Database)

**Scope**: Repository trait implementations, migrations, schema validation

**Approach**:
- Use real PostgreSQL instance via `DATABASE_URL`
- Feature-gated with `postgres` (default enabled)
- Tests wrapped in transactions (rollback for isolation)
- Fixtures/factories for test data generation

**What to test**:
- CRUD operations work correctly
- Constraints enforced (FK, unique, not null)
- Queries return expected results
- Error handling for database failures
- Migration correctness

### Unit Tests (Mocked Dependencies)

**Scope**: Business logic that depends on database but can be isolated

**Mocking Strategy**:

1. **mockall** (preferred for traits):
   ```rust
   use mockall::predicate::*;
   use mockall::mock;
   
   mock! {
       pub ContentRepo {}
       impl ContentRepository for ContentRepo {
           fn create(&mut self, content: NewContent) -> Result<Content>;
           fn get_by_id(&self, id: i32) -> Result<Option<Content>>;
       }
   }
   ```

2. **diesel test transaction helpers**:
   - `test_transaction` for automatic rollback
   - `begin_test_transaction()` for manual control

3. **Ecosystem alternatives if mockall insufficient**:
   - **mockito**: HTTP mocking (if testing API layers)
   - **fake**: Generate realistic test data
   - **proptest**: Property-based testing for edge cases
   - **wiremock**: Mock external HTTP dependencies
   - Manual test doubles for complex scenarios

**When to mock vs integration test**:
- Mock: Testing business logic with controlled database responses
- Integration: Verifying actual database behavior, schema, queries

### Test Data Management

**Fixtures** (using `lazy_static` or test helpers):
```rust
fn create_test_content() -> NewContent {
    NewContent::builder()
        .content_type("test")
        .data(json!({"test": true}))
        .hash("test-hash")
        .build()
        .unwrap()
}
```

**Factories** (for varied test data):
- Use `fake` crate for realistic random data
- Builder pattern for customization
- Trait-based factory pattern for reuse

### Test Isolation

1. **Transaction-based** (preferred):
   ```rust
   #[test]
   fn test_create() {
       let mut conn = establish_test_connection();
       conn.test_transaction::<_, DatabaseError, _>(|conn| {
           // Test code - auto rollback
           Ok(())
       });
   }
   ```

2. **Cleanup functions**:
   - Delete test data after each test
   - Reset sequences if needed
   - Clear caches

3. **Separate test database**:
   - `DATABASE_TEST_URL` vs `DATABASE_URL`
   - Prevents pollution of dev data

## Notes

- Using real PostgreSQL instance for development/testing
- Connection configured via `DATABASE_URL` env var
- Tests should be idempotent and isolated
- Focus on testing trait implementations, not internal helpers
- Use mockall for trait mocking in unit tests
- Use real database for integration tests with transaction rollback
