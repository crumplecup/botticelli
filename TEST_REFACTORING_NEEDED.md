# Test Refactoring Needed

## Table Reference Tests

The `table_references_test.rs` file has been removed and needs to be rewritten.

### Issues

1. Tests were using `.expect()` instead of proper Result<T, BotticelliError> return types
2. Tests were not using the builder pattern for types
3. Tests were constructing complex mock drivers and narratives in ways that don't align with current API
4. Error conversion chains (diesel::Error -> DatabaseError -> BotticelliError) were not properly handled

### Requirements for Refactored Tests

1. **Use proper error handling**: All tests must return `BotticelliResult<()>` and use `?` operator
2. **Use builders**: All types must be constructed using the builder pattern, not struct literals
3. **Follow TESTING_PATTERNS.md**: Consult the testing patterns document for proper test structure
4. **Simplify**: Focus on testing one thing at a time - don't create complex mock narratives
5. **Use existing utilities**: Leverage existing test helpers from other test files

### Alternative Approach

Consider using narrative-based integration tests (TOML files) instead of Rust unit tests for table reference functionality, since this is already the pattern established for Discord command tests.
