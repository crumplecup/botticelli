# Error Recovery and Retry Strategy

## Problem Statement

Currently, when the Gemini API returns a 503 "model overloaded" error, the entire narrative execution crashes. This is the wrong behavior for transient errors that are expected to resolve within seconds to minutes. We need a sophisticated retry strategy that:

1. **Distinguishes error types** - Some errors should retry (503, 429), others should fail immediately (401, 400)
2. **Uses exponential backoff** - Wait longer between each retry attempt to avoid hammering an overloaded service
3. **Respects rate limits** - Don't retry in ways that violate our configured rate limits
4. **Provides visibility** - Log retry attempts so users understand what's happening
5. **Allows configuration** - Let users control retry behavior through CLI flags or config

## Current Behavior

When a 503 error occurs:

```rust
// src/models/gemini/client.rs:569
let response = builder
    .execute()
    .await
    .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())))?;
```

The error propagates up through:

1. `GeminiClient::generate()` → `GeminiError`
2. `NarrativeExecutor::execute()` → `BoticelliError`
3. `main.rs:execute_with_driver()` → Prints "❌ Execution failed" and exits

**No retry logic exists at any level.**

## Error Classification

We need to categorize errors to determine retry behavior:

### Transient Errors (Should Retry)

- **503 Service Unavailable** - "Model is overloaded. Please try again later."
  - Cause: Temporary capacity issue
  - Strategy: Exponential backoff, 3-5 retries
  - Wait times: 2s, 4s, 8s, 16s, 32s

- **429 Too Many Requests** - Rate limit exceeded
  - Cause: We're sending requests too fast
  - Strategy: Use `x-ratelimit-reset` header if available, otherwise exponential backoff
  - Wait times: Respect the reset time from headers, or use 5s, 10s, 20s, 40s

- **500 Internal Server Error** - Temporary server issue
  - Cause: Backend service failure
  - Strategy: Limited retries with backoff
  - Wait times: 1s, 2s, 4s (max 3 retries)

- **Network errors** - Connection timeouts, DNS failures
  - Cause: Network connectivity issues
  - Strategy: Exponential backoff, shorter than 503
  - Wait times: 1s, 2s, 4s, 8s

### Permanent Errors (Should NOT Retry)

- **401 Unauthorized** - Invalid API key
  - Cause: Authentication failure
  - Action: Fail immediately with helpful message

- **400 Bad Request** - Invalid request format
  - Cause: Bug in our code or invalid user input
  - Action: Fail immediately, log full error details

- **403 Forbidden** - Insufficient permissions
  - Cause: API key doesn't have required permissions
  - Action: Fail immediately with helpful message

- **404 Not Found** - Invalid endpoint or model
  - Cause: Wrong model name or API version
  - Action: Fail immediately, suggest valid models

### Content/Safety Errors (Special Handling)

- **Content filtered** - Gemini `FinishReason::Safety`, `::Recitation`, etc.
  - Cause: Input or output violated content policies
  - Action: Fail the current act, optionally continue to next act with warning
  - Note: Already handled in `convert_response()` at line 801-813

## Proposed Architecture

### 1. Add tokio-retry2 Dependency

```toml
# Cargo.toml
[dependencies]
tokio-retry2 = "0.6"  # Check lib.rs for latest version
```

### 2. Error Classification

Extend error types to work with tokio-retry2's `RetryError`:

```rust
// src/models/gemini/error.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeminiErrorKind {
    // Existing variants...

    /// HTTP error with status code and message
    HttpError {
        status_code: u16,
        message: String,
    },
}

impl GeminiErrorKind {
    /// Check if this error type should be retried
    pub fn is_retryable(&self) -> bool {
        match self {
            GeminiErrorKind::HttpError { status_code, .. } => {
                matches!(
                    *status_code,
                    429 | 500 | 502 | 503 | 504 | 408 // Rate limit, server errors, timeout
                )
            }
            GeminiErrorKind::WebSocketConnection(_) => true,
            GeminiErrorKind::StreamInterrupted(_) => true,
            // Most other errors are permanent
            _ => false,
        }
    }
}

impl GeminiError {
    /// Convert to tokio-retry2's RetryError
    pub fn to_retry_error(self) -> tokio_retry2::RetryError<Self> {
        if self.kind.is_retryable() {
            tokio_retry2::RetryError::Transient {
                err: self,
                retry_after: None, // Could parse x-retry-after header
            }
        } else {
            tokio_retry2::RetryError::Permanent(self)
        }
    }
}
```

**Design rationale:**

- **Works with tokio-retry2 API** - `RetryError` distinguishes transient from permanent
- **Header-aware** - Can specify `retry_after` duration when available
- **Simple integration** - Convert our errors with `.to_retry_error()`

### 3. Integration with GeminiClient

Use tokio-retry2's `Retry::spawn()` to wrap API calls:

```rust
// src/models/gemini/client.rs

use tokio_retry2::{Retry, RetryError};
use tokio_retry2::strategy::{ExponentialBackoff, jitter};

impl GeminiClient {
    /// Generate content with automatic retry on transient errors
    async fn generate(&self, req: GenerateRequest) -> BoticelliResult<GenerateResponse> {
        // Configure retry strategy based on context
        let retry_strategy = self.retry_strategy();

        // Wrap the operation with retry logic
        let result = Retry::spawn(retry_strategy, || {
            self.generate_once(req.clone())
        }).await;

        // Convert RetryError back to our error type
        result.map_err(|e| match e {
            RetryError::Permanent(err) => err.into(),
            RetryError::Transient { err, .. } => err.into(),
        })
    }

    /// Determine retry strategy based on client configuration
    fn retry_strategy(&self) -> impl Iterator<Item = std::time::Duration> {
        let max_retries = self.config.max_retries.unwrap_or(5);

        ExponentialBackoff::from_millis(2000)  // Start at 2 seconds
            .factor(2)                          // Double each time
            .max_delay(std::time::Duration::from_secs(60))  // Cap at 60s
            .map(jitter)                        // Add randomness
            .take(max_retries as usize)
    }

    /// Single attempt at generating content (no retry logic)
    async fn generate_once(&self, req: GenerateRequest) -> Result<GenerateResponse, RetryError<GeminiError>> {
        // ... existing request building logic ...

        let response = builder
            .execute()
            .await
            .map_err(|e| {
                // Parse error to extract status code
                let error = if let Some(status) = extract_status_code(&e.to_string()) {
                    GeminiError::new(GeminiErrorKind::HttpError {
                        status_code: status,
                        message: e.to_string(),
                    })
                } else {
                    GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string()))
                };

                // Convert to RetryError (transient or permanent)
                error.to_retry_error()
            })?;

        // ... rest of existing response parsing ...

        Ok(GenerateResponse {
            outputs: vec![Output::Text(text)],
        })
    }
}

/// Extract HTTP status code from error message
fn extract_status_code(error_msg: &str) -> Option<u16> {
    // Parse "code 503" from "bad response from server; code 503; description: ..."
    if let Some(code_start) = error_msg.find("code ") {
        let code_str = &error_msg[code_start + 5..];
        if let Some(end) = code_str.find(|c: char| !c.is_numeric()) {
            return code_str[..end].parse().ok();
        }
    }
    None
}
```

**Design rationale:**

- **Clean separation** - `generate()` handles retry, `generate_once()` handles API call
- **Configurable strategy** - Can adjust backoff based on client config
- **Automatic logging** - tokio-retry2 emits tracing events automatically
- **Type-safe** - Compiler enforces correct error handling at each layer

### 4. Different Strategies for Different Error Types

Create strategy builders for specific error scenarios:

```rust
impl GeminiClient {
    /// Retry strategy optimized for rate limit errors (429)
    fn rate_limit_strategy() -> impl Iterator<Item = std::time::Duration> {
        ExponentialBackoff::from_millis(5000)  // Start higher
            .factor(2)
            .max_delay(std::time::Duration::from_secs(40))
            .map(jitter)
            .take(3)  // Fewer retries for rate limits
    }

    /// Retry strategy optimized for server overload (503)
    fn overload_strategy() -> impl Iterator<Item = std::time::Duration> {
        ExponentialBackoff::from_millis(2000)
            .factor(2)
            .max_delay(std::time::Duration::from_secs(60))
            .map(jitter)
            .take(5)  // More patient for transient overload
    }

    /// Retry strategy for general server errors (500, 502, 504)
    fn server_error_strategy() -> impl Iterator<Item = std::time::Duration> {
        ExponentialBackoff::from_millis(1000)  // Start fast
            .factor(2)
            .max_delay(std::time::Duration::from_secs(8))
            .map(jitter)
            .take(3)  // Quick retries only
    }
}
```

In Phase 2, we can make `generate_once()` choose the appropriate strategy based on the error type it encounters.

**Design rationale:**

- **Error-specific tuning** - Different failures need different backoff profiles
- **Easy to understand** - Named functions make intent clear
- **Reusable** - Can be used across different clients (Anthropic, etc.)

### 5. CLI Integration

Add retry configuration options to the CLI:

```rust
// src/cli.rs

#[derive(Parser)]
pub struct Cli {
    // Existing fields...

    /// Maximum retry attempts for transient errors (0 to disable)
    #[arg(long, default_value = "5")]
    pub max_retries: u32,

    /// Initial retry backoff delay in milliseconds
    #[arg(long, default_value = "2000")]
    pub retry_backoff_ms: u64,

    /// Disable all retry logic
    #[arg(long)]
    pub no_retry: bool,
}
```

**Design rationale:**

- **Sensible defaults** - Most users get good behavior without configuring anything
- **Escape hatch** - `--no-retry` for debugging or when you want fast failures
- **Tunable** - Power users can adjust based on their network/API tier

## Implementation Plan

### Phase 1: Foundation (Minimal Viable Recovery) ✅ COMPLETE

1. ✅ Add `tokio-retry2` dependency to Cargo.toml
2. ✅ Add `HttpError` variant to `GeminiErrorKind` with status code
3. ✅ Implement `is_retryable()` method on `GeminiErrorKind`
4. ✅ Implement `RetryableError` trait for error classification
5. ✅ Add `RateLimiter::execute()` method with automatic retry
6. ✅ Update error parsing to extract HTTP status codes from error messages
7. ✅ Add jitter to prevent thundering herd

**Goal:** 503 errors automatically retry with exponential backoff using tokio-retry2. ✅

**Testing:**

- ✅ Run `model_options.toml` narrative during peak load
- ✅ Verify 503 errors are retried and succeed
- ✅ Check logs show retry attempts (tokio-retry2 emits tracing events)
- ✅ Verify 401/400 errors fail immediately without retries

### Phase 2: Error-Specific Strategies ✅ COMPLETE

1. ✅ Add `retry_strategy_params()` method to `GeminiErrorKind`
2. ✅ Extend `RetryableError` trait with strategy customization
3. ✅ Update `RateLimiter::execute()` to use error-specific strategies
4. ✅ Add structured logging with error type and strategy parameters
5. ✅ Export all types at crate root per CLAUDE.md guidelines
6. ✅ Fix all doctest imports and examples

**Goal:** Different error types get different retry strategies. ✅

**Implemented Strategies:**
- 429 (rate limit): 5s initial, 3 retries, 40s cap
- 503 (overload): 2s initial, 5 retries, 60s cap
- 500/502/504 (server errors): 1s initial, 3 retries, 8s cap
- 408 (timeout): 2s initial, 4 retries, 30s cap

**Testing:**

- ✅ Comprehensive test suite validates error classification
- ✅ Tests verify correct strategy parameters per error type
- ✅ All doctests pass with proper imports

### Phase 3: Configuration and Visibility ✅ COMPLETE

1. ✅ Add CLI flags for retry configuration (`--max-retries`, `--no-retry`, `--retry-backoff-ms`)
2. ✅ Add retry config to `RateLimitOptions`
3. ✅ Add retry configuration to `RateLimiter` and `GeminiClient`
4. ✅ Apply CLI overrides to error-specific strategies
5. ✅ Enhanced logging with structured fields (attempt number, delay, error type, no_retry status)

**Goal:** Users can control retry behavior and see what's happening. ✅

**Available CLI Options:**
- `--no-retry` - Disable automatic retry completely
- `--max-retries N` - Override maximum retry attempts
- `--retry-backoff-ms MS` - Override initial backoff delay

**Testing:**

- ✅ Test `--no-retry` flag causes immediate failures
- ✅ Test `--max-retries` limits retries
- ✅ Verify logs show clear retry progress with all configuration details

### Phase 4: Live API Retry Support (High Priority)

**Current Issue:**
Live API models (`gemini-2.0-flash-live`, `gemini-2.5-flash-live`) don't benefit from automatic retry because they bypass the `RateLimiter::execute()` code path. WebSocket connections fail immediately on transient errors.

**Architecture Problem:**
```rust
// Current flow - NO RETRY
GeminiClient::generate()
  └─ generate_via_live_api()
      └─ live_client.connect_with_config()  // ❌ Fails immediately on error
          └─ WebSocket handshake

// Desired flow - WITH RETRY  
GeminiClient::generate()
  └─ generate_via_live_api()
      └─ RateLimiter::execute()  // ✅ Automatic retry with backoff
          └─ live_client.connect_with_config()
              └─ WebSocket handshake
```

**Implementation Plan:**

1. **Wrap Live API operations in RateLimiter::execute()**
   - Create a rate-limited wrapper for Live API client
   - Store `RateLimiter<GeminiLiveClient>` in GeminiClient
   - Pass retry config when creating Live API rate limiter
   - Use `execute()` for connection establishment

2. **Make Live API operations compatible with execute()**
   - Connection: `connect_with_config()` should be retryable
   - Message sending: `send_text()` already retryable
   - Error handling: Ensure WebSocket errors surface correctly

3. **Test Live API retry behavior**
   - Test WebSocket handshake failures retry automatically
   - Test WebSocket connection failures retry
   - Test rate limit (429) during Live API session
   - Verify retry respects CLI flags (--no-retry, --max-retries)

**Code Changes Needed:**

```rust
// In GeminiClient struct
pub struct GeminiClient {
    clients: Arc<Mutex<HashMap<String, RateLimiter<TieredGemini<TierConfig>>>>>,
    
    // Change from Option<GeminiLiveClient> to rate-limited wrapper
    live_client: Option<RateLimiter<GeminiLiveClient>>,
    
    // ... existing fields
}

// In new_internal()
let live_client = {
    let rpm = base_tier.rpm();
    super::live_client::GeminiLiveClient::new_with_rate_limit(rpm)
        .map(|client| {
            RateLimiter::new_with_retry(
                client,
                no_retry,
                max_retries,
                retry_backoff_ms,
            )
        })
        .ok()
};

// In generate_via_live_api()
async fn generate_via_live_api(
    &self,
    req: &GenerateRequest,
    model_name: &str,
) -> GeminiResult<GenerateResponse> {
    let live_limiter = self.live_client.as_ref()
        .ok_or_else(|| GeminiError::new(GeminiErrorKind::ClientCreation(
            "Live API client not available".to_string()
        )))?;

    // Build config
    let config = super::live_protocol::GenerationConfig {
        max_output_tokens: req.max_tokens.map(|t| t as i32),
        temperature: req.temperature.map(|t| t as f64),
        ..Default::default()
    };

    // Capture model_name and config for the closure
    let model = model_name.to_string();
    let gen_config = config.clone();
    
    // Use execute() to get automatic retry on connection failures
    let mut session = live_limiter.execute(0, || {
        let m = model.clone();
        let c = gen_config.clone();
        async move {
            live_limiter.inner().connect_with_config(&m, c).await
        }
    }).await?;

    // Rest of implementation...
}
```

**Benefits:**
- ✅ WebSocket handshake failures retry automatically
- ✅ Connection errors get exponential backoff
- ✅ Live API respects --no-retry and --max-retries flags
- ✅ Consistent retry behavior across REST and Live APIs
- ✅ Structured logging for Live API retries

**Testing:**
- Test with model_options.toml (includes live models)
- Verify retry on connection failures
- Test CLI flags work with Live API
- Verify logging shows retry attempts

**Goal:** Live API models work reliably with automatic retry, just like REST API models.

---

### Phase 5: Advanced Features (Future Enhancements)

These features would further improve reliability but are lower priority:

1. ❌ **Respect x-retry-after header** - Use server-provided retry timing
   - Parse `x-retry-after` or `retry-after` headers from 429/503 responses
   - Override calculated backoff with server-specified delay
   - Requires access to response headers in error chain

2. ❌ **Circuit breaker pattern** - Stop retrying if service is persistently down
   - Track failure rate over time window
   - Open circuit after N consecutive failures
   - Half-open state to test recovery
   - Prevent wasting resources on known-bad endpoints

3. ❌ **Retry budget** - Limit total retry time per narrative
   - Set maximum total time for all retries (e.g., 5 minutes)
   - Track cumulative retry time across all acts
   - Fail fast if budget exhausted
   - Prevents narratives from hanging indefinitely

4. ❌ **Per-act retry configuration in TOML** - Fine-grained control
   - Allow acts to specify custom retry behavior
   - Override global settings per act
   - Enable/disable retry for specific acts
   - Example: `[act.retry] max_retries = 10, backoff_ms = 500`

5. ❌ **Metrics/telemetry for retry behavior** - Production observability
   - Track retry success/failure rates
   - Record retry latency distribution
   - Count retries by error type
   - Export to metrics systems (Prometheus, etc.)
   - Add retry summary to execution output

**Goal:** Production-grade retry system with advanced reliability features.

## Recommended Approach: Use tokio-retry2

After researching the ecosystem, **we should use `tokio-retry2`** instead of implementing custom retry logic:

### Why tokio-retry2?

- **Battle-tested:** Fork of widely-used `tokio-retry` (used by 282+ crates)
- **Actively maintained:** Recent updates in 2024, keeps dependencies current
- **Rich features:** ExponentialBackoff, FixedInterval, FibonacciBackoff strategies
- **Jitter support:** Prevents thundering herd via built-in `jitter()` function
- **Tracing integration:** Works with our existing `tracing` setup
- **Transient vs permanent errors:** `RetryError::to_transient()` and `::to_permanent()`
- **Early exit:** Pattern-match errors to bail out of retry loops when appropriate

### Comparison with Alternatives

**tokio-retry (original):**

- Still maintained but improvements happening in tokio-retry2
- 282 crates depend on it (highly trusted)
- tokio-retry2 is backward-compatible improvement

**backoff crate:**

- More general-purpose (sync + async)
- Less tight integration with Tokio
- Good if we need non-Tokio retry, but tokio-retry2 is more focused

**Custom implementation:**

- ❌ Reinventing the wheel (~200 lines)
- ❌ Need to test edge cases ourselves
- ❌ Need to maintain over time
- ✅ More control (but we don't need it)

**Decision:** Use `tokio-retry2`. It's the right tool for the job.

## Alternative Approaches Considered

### Option A: Retry at Executor Level

Instead of retrying in the client, retry at the `NarrativeExecutor` level when executing acts.

**Pros:**

- Works for all backends (Gemini, Anthropic, etc.) without per-client changes
- Can implement narrative-specific retry policies
- Can save partial progress and resume from failed act

**Cons:**

- Less granular - retries entire act instead of just the API call
- Harder to distinguish client errors from executor errors
- Loses access to HTTP-specific info (status codes, headers)

**Decision:** Start with client-level retries (Phase 1-3), consider executor-level for Phase 4.

### Option B: Let Rate Limiter Handle It

Extend the existing `RateLimiter` to automatically retry on 429/503.

**Pros:**

- Centralized rate limit logic
- Already has token bucket and backoff concepts

**Cons:**

- Rate limiter is about _preventing_ errors, retry is about _recovering_ from them
- Mixing concerns makes both components more complex
- Harder to configure separately

**Decision:** Keep retry logic separate. Rate limiter prevents errors proactively, retry handles unexpected failures.

## Success Metrics

How will we know this is working?

1. **Reliability:** Narratives that failed with 503 now succeed after retries
2. **Visibility:** Logs clearly show retry attempts and outcomes
3. **Performance:** Retry delays are reasonable (not too fast, not too slow)
4. **User Control:** CLI flags allow users to tune retry behavior
5. **Code Quality:** Error handling is consistent across all backends

## Future Enhancements

Beyond the scope of this initial implementation, but worth considering:

1. **Persistent retry state** - Save narrative progress to database, resume after crashes
2. **Smart retry scheduling** - Use Gemini's reported `x-ratelimit-reset` time instead of guessing
3. **Cost-aware retries** - Don't retry if user's daily budget is exhausted
4. **Partial success handling** - Save successful acts even if later ones fail
5. **Adaptive backoff** - Learn optimal retry delays based on success rates
6. **Multi-region fallback** - Retry with different API endpoints if primary is overloaded

## References

- [Google Cloud Retry Guidelines](https://cloud.google.com/apis/design/errors#error_retries)
- [AWS Exponential Backoff](https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/)
- [RFC on HTTP Status Codes](https://httpwg.org/specs/rfc9110.html#status.codes)
- [Polly (C# Resilience Library)](https://github.com/App-vNext/Polly) - Good patterns reference
