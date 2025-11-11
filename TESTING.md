# Testing Guide

## Overview

Boticelli has a comprehensive test suite designed to maximize coverage while minimizing API usage and costs. Most tests use mock drivers to avoid hitting actual LLM APIs.

## Test Categories

### 1. Unit Tests (No API Calls)

These tests run automatically with `cargo test` and don't require API keys:

- **Rate Limiter Logic** - `tests/rate_limit_limiter_test.rs`
  - Tests RPM, TPM, RPD, and concurrent limiting
  - Uses low limits to see blocking behavior quickly
  - All tests complete in <100ms

- **Configuration** - `tests/rate_limit_config_test.rs`
  - TOML parsing and merging
  - Tier config loading
  - Bundled defaults

- **Header Detection** - `tests/rate_limit_detector_test.rs`
  - Provider-specific header parsing
  - Gemini, Anthropic, OpenAI formats
  - Cache functionality

- **Narrative Executor** - `tests/narrative_executor_test.rs`
  - Uses mock drivers (no API calls)
  - Tests context passing, multimodal inputs
  - 6 tests, all mocked

- **Narrative Parsing** - `tests/narrative_test.rs`
  - TOML validation
  - Multimodal content parsing

### 2. Integration Tests (Optional API Calls)

These tests are gated behind `BOTICELLI_RUN_API_TESTS` environment variable:

```bash
# Run only unit tests (default, no API)
cargo test

# Run with API tests (requires GEMINI_API_KEY)
BOTICELLI_RUN_API_TESTS=1 cargo test
```

**API Test Constraints:**
- **Maximum 2 requests total** across all API tests
- **~7 tokens per request** (minimal prompts: "Say 'hi'", "Say 'ok'")
- **Total: ~14 tokens** consumed when running API tests
- **Rate Limiting: Free tier safe** (10 RPM, 250 RPD)

API tests in `tests/rate_limit_integration_test.rs`:
1. `test_gemini_client_without_rate_limiting` - 1 request, ~7 tokens
2. `test_gemini_client_with_rate_limiting` - 1 request, ~7 tokens

### 3. Manual CLI Testing

For end-to-end CLI testing, use the minimal narrative:

```bash
# Test with rate limiting (uses config defaults)
cargo run -- run -n narratives/test_minimal.toml --backend gemini

# Test with explicit tier
cargo run -- run -n narratives/test_minimal.toml --backend gemini --tier free

# Test with custom limits
cargo run -- run -n narratives/test_minimal.toml --backend gemini --rpm 5

# Test without rate limiting
cargo run -- run -n narratives/test_minimal.toml --backend gemini --no-rate-limit
```

**Minimal narrative tokens:** ~5 tokens total (2 input, 3 output)

## Free Tier Budget Management

### Gemini Free Tier Limits
- **RPM:** 10 requests per minute
- **TPM:** 250,000 tokens per minute
- **RPD:** 250 requests per day

### Testing Budget

| Test Type | Requests | Tokens | % of Daily |
|-----------|----------|--------|------------|
| Unit tests (cargo test) | 0 | 0 | 0% |
| Integration tests (API) | 2 | ~14 | 0.8% |
| Manual CLI test (1x) | 1 | ~5 | 0.4% |
| **Total per full test run** | **3** | **~19** | **1.2%** |

You can run the full test suite (unit + API) **~83 times per day** before hitting the 250 RPD limit.

### Conservative Testing Strategy

1. **Default workflow** (multiple times per day):
   ```bash
   cargo test  # Unit tests only, no API
   ```

2. **Before commits** (1-2 times per feature):
   ```bash
   BOTICELLI_RUN_API_TESTS=1 cargo test  # +2 requests
   ```

3. **Manual CLI verification** (1 time per major change):
   ```bash
   cargo run -- run -n narratives/test_minimal.toml --backend gemini
   ```

4. **Full narrative testing** (sparingly, for real workflows):
   ```bash
   cargo run -- run -n narratives/mint.toml --backend gemini
   # Use longer narratives only when necessary
   ```

## Running Tests

### Quick Tests (No API, < 1 second)
```bash
cargo test
```

### With API Tests (Requires Key, ~2 requests)
```bash
# Set API key
export GEMINI_API_KEY="your-key-here"

# Run all tests including API
BOTICELLI_RUN_API_TESTS=1 cargo test

# Run only integration tests
BOTICELLI_RUN_API_TESTS=1 cargo test --test rate_limit_integration_test
```

### Specific Test Suites
```bash
# Rate limiter logic (no API)
cargo test --test rate_limit_limiter_test

# Configuration (no API)
cargo test --test rate_limit_config_test

# Header detection (no API)
cargo test --test rate_limit_detector_test

# Narrative executor with mocks (no API)
cargo test --test narrative_executor_test

# Integration with optional API
BOTICELLI_RUN_API_TESTS=1 cargo test --test rate_limit_integration_test
```

### Watch Mode
```bash
# Run tests on file changes (unit tests only, no API)
cargo watch -x test

# With API tests
BOTICELLI_RUN_API_TESTS=1 cargo watch -x test
```

## Debugging Rate Limiting

### Verify Rate Limiter is Working

Run with verbose output to see rate limiting in action:

```bash
# Should show rate limiting info on startup
cargo run -- run -n narratives/test_minimal.toml --backend gemini --verbose

# Expected output:
# Rate Limiting: Free (RPM: Some(10), TPM: Some(250000), RPD: Some(250))
```

### Test Rate Limit Blocking

Use aggressive limits to see blocking quickly:

```bash
# Set very low RPM to see rate limiting
cargo run -- run -n narratives/mint.toml --backend gemini --rpm 1

# This will pause between requests (1 per minute)
```

### Monitor API Usage

Check your Gemini API dashboard to verify rate limiting is working:
- https://aistudio.google.com/app/apikey

Look for:
- Request rate matches your tier
- No 429 errors (rate limit exceeded)
- Token usage is as expected

## Continuous Integration

For CI/CD, run only unit tests by default:

```yaml
# .github/workflows/test.yml
- name: Run unit tests
  run: cargo test
  # No API key needed, no API calls made
```

To test with API in CI (optional):

```yaml
- name: Run integration tests
  run: BOTICELLI_RUN_API_TESTS=1 cargo test
  env:
    GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
  # Only run on main branch or releases to conserve quota
  if: github.ref == 'refs/heads/main'
```

## Tips for Economical Testing

1. **Use mocks by default** - All new tests should use mock drivers unless specifically testing API integration

2. **Keep prompts minimal** - Test prompts should be 1-5 words: "Say 'hi'", "Count to 3", etc.

3. **Set low max_tokens** - Use `max_tokens: Some(5)` in test requests

4. **Use temperature: 0.0** - Makes responses deterministic and predictable

5. **Test rate limiting logic separately** - Use unit tests with low limits (RPM: 2) to see behavior quickly

6. **Gate API tests** - Always use `skip_unless_api_tests_enabled!()` macro for API tests

7. **Reuse narratives** - Create shared minimal test narratives instead of generating new ones

8. **Monitor usage** - Check API dashboard regularly to ensure tests aren't consuming too much quota

## Example: Adding a New Test

```rust
#[tokio::test]
async fn test_new_feature() {
    // ✅ GOOD: Use mock driver
    struct MockDriver;

    #[async_trait]
    impl BoticelliDriver for MockDriver {
        async fn generate(&self, _req: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
            Ok(GenerateResponse {
                outputs: vec![Output::Text("mock response".to_string())],
            })
        }
        // ... other trait methods
    }

    let driver = MockDriver;
    // Test your feature with the mock
}

#[tokio::test]
async fn test_new_feature_with_real_api() {
    // ✅ GOOD: Gate API test
    skip_unless_api_tests_enabled!();

    let client = GeminiClient::new().expect("Failed to create client");

    // ✅ GOOD: Minimal request
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Hi".to_string())], // 1 token
        }],
        temperature: Some(0.0),  // ✅ Deterministic
        max_tokens: Some(3),     // ✅ Minimal output
    };

    let response = client.generate(&request).await.expect("API failed");

    // Verify response
    assert!(!response.outputs.is_empty());
}
```

## Troubleshooting

### "Test failed: API key not found"
- Set `GEMINI_API_KEY` environment variable
- Or skip API tests: run `cargo test` without `BOTICELLI_RUN_API_TESTS=1`

### "Rate limit exceeded (429)"
- Wait 1 minute before retrying
- Check your daily quota usage
- Use `--rpm` flag to slow down requests
- Run fewer tests with API calls

### "Tests are slow"
- Check if you accidentally enabled API tests: `echo $BOTICELLI_RUN_API_TESTS`
- Unit tests should complete in < 1 second
- API tests may take 2-5 seconds total

## Summary

- **Default: `cargo test`** - Fast, free, no API
- **Before commit: `BOTICELLI_RUN_API_TESTS=1 cargo test`** - 2 requests, ~14 tokens
- **Manual verification: CLI with test_minimal.toml** - 1 request, ~5 tokens
- **Budget: ~19 tokens per full test cycle** - Can run 83x per day safely
