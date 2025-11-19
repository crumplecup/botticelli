# Test Strategy for API-Driven Libraries

## Problem

Currently, tests are mixed together and `--all-features` runs API-consuming tests that:
1. Require environment variables (GEMINI_API_KEY)
2. Consume rate-limited API quotas
3. May fail due to network issues or API changes
4. Should not run on every code change

This makes it impossible to have a reliable, fast test suite for local development.

## Solution: Segregated Test Groups

### Test Categories

**Local Tests (default)**
- Run on every code change
- No external dependencies
- No API keys required
- Fast execution
- Include:
  - Unit tests for business logic
  - Schema inference tests
  - Rate limiter logic tests
  - Protocol serialization/deserialization tests
  - Mock/fake implementations

**API Tests (opt-in)**
- Run only when explicitly requested
- Require API keys
- Consume actual API quotas
- May be slow or fail due to network
- Run:
  1. By human user manually
  2. Before merges to another branch
  3. When explicitly prompted for targeted testing
- Include:
  - Real API integration tests
  - Live streaming tests
  - End-to-end workflows

### Feature Flag Strategy

Instead of using `--all-features`, use specific feature combinations:

**For local development:**
```bash
cargo test                    # Only local tests (no features)
cargo check                   # Quick compilation check
cargo clippy                  # Linting without features
```

**For API testing:**
```bash
cargo test --features gemini,api          # Gemini API tests
cargo test --features anthropic,api       # Anthropic API tests (future)
cargo test --all-features                 # All API tests (expensive!)
```

### Current Feature Flags

Review and potentially add:
- `gemini` - Enables Gemini provider code (already exists)
- `api` - Empty marker flag to gate API-consuming tests (NEW)
- Other provider flags as added

### Implementation Strategy

#### 1. Add `api` Feature Flag

In workspace `Cargo.toml`:
```toml
[features]
default = []
gemini = []
api = []  # NEW: Empty marker for API tests
```

In each crate that has API tests:
```toml
[features]
api = []  # Empty marker for API tests
```

#### 2. Mark API Tests Appropriately

**Current problematic pattern:**
```rust
#[test]
fn test_real_api_call() {
    let client = Client::new(env::var("GEMINI_API_KEY").unwrap());
    // ... uses real API
}
```

**New pattern:**
```rust
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_real_api_call() {
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY required for API tests");
    let client = Client::new(api_key);
    // ... uses real API
}
```

**Better pattern with clear error:**
```rust
#[test]
#[cfg(feature = "api")]
fn test_real_api_call() {
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY required for API tests");
    let client = Client::new(api_key);
    // ... uses real API
}
```

#### 3. Update Test Files

Files to review and update:
- `tests/gemini.rs` - Contains `test_client_creation` and `test_real_api_call`
- `tests/gemini_streaming_test.rs` - Likely has streaming API tests
- `tests/gemini_live_*_test.rs` - Live API integration tests
- Any other test files that make real API calls

#### 4. Update Documentation

Update `CLAUDE.md` Workflow section:
```markdown
### Verification Checklist Before Committing

Run these commands and ensure ALL pass with zero errors/warnings:

```bash
# 1. Check compilation (no features needed)
cargo check

# 2. Run LOCAL tests only (fast, no API keys)
cargo test --lib --tests

# 3. Run doctests
cargo test --doc

# 4. Run clippy
cargo clippy --all-targets
```

### Before Merging to Main

Run API tests to ensure integrations still work:

```bash
# Requires GEMINI_API_KEY in environment
cargo test --features gemini,api
```
```

#### 5. Client Creation Tests

For tests like `test_client_creation` that just verify the client can be created:
- Should NOT require real API keys
- Use mock/fake credentials for construction tests
- Only validate structure, not actual API calls

Example refactor:
```rust
// LOCAL test - no API calls
#[test]
fn test_client_construction() {
    // Test that client can be constructed with a string
    let client = Client::new("fake-api-key-for-testing");
    assert!(client.base_url().contains("generativelanguage.googleapis.com"));
}

// API test - requires real key
#[test]
#[cfg(feature = "api")]
fn test_client_validates_real_key() {
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY required");
    let client = Client::new(api_key);
    // Make minimal API call to verify key works
}
```

## Implementation Steps

### Step 1: Add `api` feature flag
1. Update workspace `Cargo.toml` features
2. Update `botticelli_models/Cargo.toml` features
3. Update any other crates with API tests

### Step 2: Audit and categorize tests
1. Review all test files in `tests/` directory
2. Identify which tests make real API calls
3. Identify which tests require API keys
4. Mark them with `#[cfg(feature = "api")]`

### Step 3: Refactor client creation tests
1. Separate construction tests from validation tests
2. Make construction tests use fake credentials
3. Move real API validation to API-gated tests

### Step 4: Update documentation
1. Update `CLAUDE.md` workflow section
2. Update `TESTING.md` if it exists
3. Add comments in test files explaining the strategy

### Step 5: Verify and commit
1. Run `cargo test` (should pass without API keys)
2. Run `cargo test --features gemini,api` (should work with API key)
3. Commit with message: "refactor(tests): segregate local and API tests"

## Benefits

1. **Fast local development** - No API calls in default test runs
2. **No accidental API usage** - Must explicitly opt-in with feature flag
3. **Clear separation** - Obvious which tests consume resources
4. **CI-friendly** - Can run local tests in CI without API keys
5. **Developer-friendly** - New contributors can test without API access

## Future Considerations

- Mock server for integration testing without real APIs
- Record/replay pattern for deterministic API testing
- Separate test suites by provider (Gemini, Anthropic, etc.)
- Rate limit tracking in test output
