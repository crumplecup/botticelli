# Gemini Client Architecture Guide

## Overview

The `GeminiClient` implementation enables per-request model selection for Google's Gemini API, allowing multi-model narratives where different acts can use different Gemini models (e.g., `gemini-2.5-flash`, `gemini-2.5-flash-lite`, `gemini-2.5-pro`).

**Status**: Complete. All phases finished. Per-request model selection fully functional and tested with live API.

## Architecture

### The Problem We Solved

**Original Issue**: The `gemini-rust` crate requires model selection at client creation time via `Gemini::with_model(api_key, model_name)`, but Boticelli's API design expects per-request model selection via `GenerateRequest.model`. This architectural mismatch meant all requests used the same model regardless of what was specified in the request.

**Impact**:
- Multi-model narratives broken
- Cost unpredictability (may use expensive models when cheap ones requested)
- Violated `BoticelliDriver` trait contract

### The Solution: Three-Layer Architecture

We implemented a three-layer architecture that couples clients with rate limiting:

```
┌─────────────────────────────────────────────────────────────┐
│ GeminiClient                                                │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ HashMap<String, RateLimiter<TieredGemini<GeminiTier>>>  │ │
│ │                                                           │ │
│ │  "gemini-2.0-flash" ──► RateLimiter ──► TieredGemini    │ │
│ │                                          ├─ Gemini       │ │
│ │                                          └─ GeminiTier   │ │
│ │                                                           │ │
│ │  "gemini-2.5-flash" ──► RateLimiter ──► TieredGemini    │ │
│ │                                          ├─ Gemini       │ │
│ │                                          └─ GeminiTier   │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

#### Layer 1: `TieredGemini<T: Tier>`

**File**: `src/models/gemini.rs` (lines 96-161)

Couples a Gemini API client with its rate limit tier:

```rust
#[derive(Clone)]
pub struct TieredGemini<T: Tier> {
    pub client: Gemini,
    pub tier: T,
}

impl<T: Tier> Tier for TieredGemini<T> {
    // Delegates all Tier methods to self.tier
    fn rpm(&self) -> Option<u32> { self.tier.rpm() }
    fn tpm(&self) -> Option<u64> { self.tier.tpm() }
    // ... etc
}
```

**Purpose**:
- Couples the client with its rate limit configuration
- Implements `Tier` so it can be used with `RateLimiter<T>`
- Enables type-safe access control

#### Layer 2: `RateLimiter<T: Tier>`

**File**: `src/rate_limit/limiter.rs` (lines 21-219)

Generic rate limiter that takes ownership of any type implementing `Tier`:

```rust
#[derive(Clone)]
pub struct RateLimiter<T: Tier> {
    inner: T,
    rpm_limiter: Option<Arc<DirectRateLimiter>>,
    tpm_limiter: Option<Arc<DirectRateLimiter>>,
    rpd_limiter: Option<Arc<DirectRateLimiter>>,
    concurrent_semaphore: Arc<Semaphore>,
}

impl<T: Tier> RateLimiter<T> {
    pub fn new(tier: T) -> Self { /* ... */ }
    pub fn inner(&self) -> &T { &self.inner }
    pub async fn acquire(&self, estimated_tokens: u64) -> RateLimiterGuard { /* ... */ }
}
```

**Purpose**:
- Enforces rate limits (RPM, TPM, RPD, concurrent requests)
- Owns the inner value (TieredGemini)
- Provides controlled access via `inner()` method
- Generic over `T: Tier` - no dynamic dispatch overhead

**Key Property**: Cheap to clone (all internal state is Arc-wrapped)

#### Layer 3: `GeminiClient`

**File**: `src/models/gemini.rs` (lines 169-189)

Client pool that manages model-specific rate-limited clients:

```rust
pub struct GeminiClient {
    clients: Arc<Mutex<HashMap<String, RateLimiter<TieredGemini<GeminiTier>>>>>,
    api_key: String,
    model_name: String,      // Default model
    default_tier: GeminiTier,
}
```

**Purpose**:
- Lazy client creation (only create clients for models actually used)
- One client per model, each with independent rate limiting
- Thread-safe access via `Arc<Mutex<HashMap>>`
- Minimal lock contention (held only during get-or-create)

### Request Flow

When `generate_internal()` is called:

1. **Extract model name**:
   ```rust
   let model_name = req.model.as_ref().unwrap_or(&self.model_name);
   ```

2. **Get or create rate-limited client**:
   ```rust
   let rate_limited_client = {
       let mut clients = self.clients.lock().unwrap();
       clients.entry(model_name.clone())
           .or_insert_with(|| {
               let client = Gemini::with_model(&self.api_key, model_name.clone())
                   .expect("Failed to create client");
               let tiered = TieredGemini { client, tier: self.default_tier };
               RateLimiter::new(tiered)
           })
           .clone()  // Cheap - all Arc internals
   };
   ```

3. **Acquire rate limit**:
   ```rust
   let _guard = rate_limited_client.acquire(estimated_tokens).await;
   ```

4. **Access client through rate limiter**:
   ```rust
   let client = &rate_limited_client.inner().client;
   let mut builder = client.generate_content();
   ```

5. **Execute request** (existing message processing logic)

## Usage

### Basic Usage

```rust
use boticelli::{GeminiClient, GenerateRequest, Input, Message, Role};

// Create client (uses Free tier by default)
let client = GeminiClient::new()?;

// Request using default model (gemini-2.5-flash)
let request = GenerateRequest {
    messages: vec![Message {
        role: Role::User,
        content: vec![Input::Text("Hello".to_string())],
    }],
    model: None,  // Uses default
    ..Default::default()
};

let response = client.generate(&request).await?;
```

### Per-Request Model Selection

```rust
// Override model for this request
let request = GenerateRequest {
    messages: vec![Message {
        role: Role::User,
        content: vec![Input::Text("Complex task".to_string())],
    }],
    model: Some("gemini-2.5-pro".to_string()),  // Use pro model for complex tasks
    ..Default::default()
};

let response = client.generate(&request).await?;
```

### Multi-Model Narratives

```toml
# narrations/text_models.toml
[acts.act1]
model = "gemini-2.5-flash-lite"  # Lite model for drafting
[[acts.act1.input]]
type = "text"
content = "Generate draft content"

[acts.act2]
model = "gemini-2.5-flash"  # Standard model for critique
[[acts.act2.input]]
type = "text"
content = "Critique the draft"

[acts.act3]
model = "gemini-2.5-pro"  # Pro model for final polished version
[[acts.act3.input]]
type = "text"
content = "Create final polished version"
```

```rust
use boticelli::{GeminiClient, Narrative, NarrativeExecutor};

let client = GeminiClient::new()?;
let executor = NarrativeExecutor::new(client);
let narrative = Narrative::from_file("narrations/text_models.toml")?;

let execution = executor.execute(&narrative).await?;
// Each act uses its specified model
```

## Implementation Details

### Model Name Conversion

The gemini-rust crate requires `Model` enum variants, not string model names. We convert string model names to the appropriate enum variants:

**File**: `src/models/gemini.rs` (lines 254-281)

```rust
fn model_name_to_enum(name: &str) -> Model {
    match name {
        "gemini-2.5-flash" => Model::Gemini25Flash,
        "gemini-2.5-flash-lite" => Model::Gemini25FlashLite,
        "gemini-2.5-pro" => Model::Gemini25Pro,
        "text-embedding-004" => Model::TextEmbedding004,
        // For other model names, use Custom variant with "models/" prefix
        other => {
            if other.starts_with("models/") {
                Model::Custom(other.to_string())
            } else {
                Model::Custom(format!("models/{}", other))
            }
        }
    }
}
```

**Key Detail**: The Gemini API requires model names in `Model::Custom()` to be prefixed with `"models/"`. Our conversion automatically adds this prefix if not already present.

**Supported Models**:
- **Gemini 2.5 models** (use enum variants):
  - `gemini-2.5-flash` (default)
  - `gemini-2.5-flash-lite`
  - `gemini-2.5-pro`
- **Gemini 2.0 models** (use Custom with "models/" prefix):
  - `gemini-2.0-flash` → `Model::Custom("models/gemini-2.0-flash")`
  - `gemini-2.0-flash-lite` → `Model::Custom("models/gemini-2.0-flash-lite")`
- **Any other model** (use Custom with "models/" prefix):
  - `models/gemini-experimental` → preserved as-is
  - `custom-model-name` → `Model::Custom("models/custom-model-name")`

**Live API Models** (use WebSocket protocol):
- **Experimental models** - Automatically routed to Live API (WebSocket):
  - `models/gemini-2.0-flash-exp` - Experimental model via WebSocket
  - `models/gemini-2.0-flash-live` - Live model (if available)
  - Any model containing `-live` or `-exp` in the name

  These models use WebSocket connections for bidirectional streaming. The `GeminiClient` automatically detects and routes them to the Live API. See [Live API Support](#live-api-support) below for details.

### Tier Conversion

Since `new_with_tier()` accepts `Option<Box<dyn Tier>>` (for API compatibility) but we need concrete `GeminiTier`, we use name matching:

**File**: `src/models/gemini.rs` (lines 350-361)

```rust
let default_tier = if let Some(tier) = tier {
    match tier.name() {
        "Free" => GeminiTier::Free,
        "Pay-as-you-go" => GeminiTier::PayAsYouGo,
        _ => GeminiTier::Free,  // Default for unknown
    }
} else {
    GeminiTier::Free
};
```

This pragmatic approach handles the Box<dyn Tier> → GeminiTier conversion without breaking the existing API.

### Client Lifecycle

- **Creation**: Lazy - only created when first requested
- **Caching**: Stored in HashMap for reuse
- **Cleanup**: Never - clients live for program lifetime
- **Memory**: Minimal - only creates clients for models actually used

### Thread Safety

- **HashMap access**: Protected by `Mutex`
- **Lock duration**: Minimal - held only during get-or-create
- **Cloning**: Cheap - `RateLimiter` clone is O(1) (Arc internals)
- **Contention**: Low - typical usage is serial (one request at a time)

### Error Handling

**Current**: Uses `.expect()` in `or_insert_with` for client creation failures

**Rationale**: Client creation failures are initialization errors (bad API key, network issues), not recoverable at this point. Panicking is acceptable.

**Future**: Could improve with two-phase creation or better error propagation (see Phase 6 in implementation history).

## Testing

Test suites validate both Gemini 2.5 and 2.0 model support:

### Gemini 2.5 Model Tests
**File**: `tests/gemini_model_test.rs`

1. **Default model usage**: Verify default model when `req.model` is None
2. **Model override**: Verify correct model used when `req.model` is Some
3. **Multiple model requests**: Verify client pool handles different models
4. **Narrative integration**: Verify multi-model narrative execution

### Gemini 2.0 Model Tests
**File**: `tests/gemini_2_0_models_test.rs`

1. **Gemini 2.0 Flash**: Verify gemini-2.0-flash works via Model::Custom
2. **Gemini 2.0 Flash Lite**: Verify gemini-2.0-flash-lite works
3. **Mixed 2.0 and 2.5 models**: Verify client pool handles both generations
4. **Explicit "models/" prefix**: Verify user-provided prefix is preserved

**Run tests** (requires `GEMINI_API_KEY`):
```bash
# Run all Gemini tests
cargo test --features gemini -- --ignored

# Run only 2.5 model tests
cargo test --features gemini --test gemini_model_test -- --ignored

# Run only 2.0 model tests
cargo test --features gemini --test gemini_2_0_models_test -- --ignored
```

## Benefits of This Architecture

1. **Type Safety**: Cannot access client without going through rate limiter
2. **Performance**: No dynamic dispatch (`Box<dyn Trait>` eliminated)
3. **Correctness**: Each model has independent rate limiting
4. **Efficiency**: Clients reused, cheap cloning
5. **Simplicity**: Single HashMap instead of separate structures
6. **Extensibility**: Generic pattern works for other providers

## Future Work

### Model-Specific Rate Limits (Phase 10)

**Current**: All models in a tier share the same rate limits

**Future**: Allow per-model rate limits in `boticelli.toml`:

```toml
[providers.gemini.tiers.free.models."gemini-2.0-flash"]
rpm = 10
tpm = 250_000

[providers.gemini.tiers.free.models."gemini-2.0-flash-lite"]
rpm = 15  # Lite model has higher RPM
tpm = 250_000
```

**Implementation**:
1. Extend `BoticelliConfig` to parse nested model configuration
2. Modify client creation to look up model-specific tier
3. Fall back to tier-level defaults if model config not found

### Better Error Handling (Phase 6)

**Current**: Panics on client creation failure in `or_insert_with`

**Improvement Options**:
1. Two-phase creation: validate outside lock, insert inside
2. Pre-validate model names against known models
3. Return `Result` from `generate_internal` and handle gracefully

### Configurable Default Model

**Current**: Hard-coded `"gemini-2.0-flash"`

**Future**: Allow configuration in `boticelli.toml`:

```toml
[providers.gemini]
default_model = "gemini-2.0-flash-lite"
default_tier = "free"
```

## Implementation History

### Phase 1: Investigation and Tests ✓
- Identified the bug (model field ignored)
- Created test suite (`tests/gemini_model_test.rs`)
- Documented the problem in GEMINI.md

### Phase 2: Generic RateLimiter ✓
- Changed `RateLimiter` from `RateLimiter { _tier: Box<dyn Tier> }` to `RateLimiter<T: Tier> { inner: T }`
- Added `inner()` method for accessing wrapped value
- Updated tests to remove `Box<dyn Tier>` wrappers

### Phase 3: TieredGemini Type ✓
- Created `TieredGemini<T: Tier>` struct
- Implemented `Tier` trait by delegation
- Exported from crate root

### Phase 4: GeminiClient Refactoring ✓
- Replaced single client with HashMap of rate-limited clients
- Added `api_key`, `default_tier` fields
- Updated constructor to do lazy initialization

### Phase 5: Per-Request Model Selection ✓
- Implemented get-or-create pattern in `generate_internal()`
- Added model name extraction from request
- Integrated rate limiting per client

### Phase 6: Backward Compatibility Verification ✓
- Verified `src/main.rs` still works with unchanged API
- All tests compile successfully
- No breaking changes to public API

### Phase 7: Supporting Documentation ✓
- Updated module-level documentation
- Enhanced `model_name()` and `Metadata` impl docs
- Added usage examples to module docs

### Phase 8: API Integration Testing ✓
- Fixed Model enum conversion (string names → enum variants)
- Changed default model from gemini-2.0-flash to gemini-2.5-flash
- Updated all tests to use Gemini 2.5 models
- All 5 API integration tests pass with live API calls

### Phase 9: Documentation and Examples ✓
- Updated GEMINI.md with Model enum conversion details
- Added model compatibility warnings
- Updated all examples to use Gemini 2.5 models
- Documented recommended model selection

## Live API Support

**Status**: Complete. WebSocket-based Live API fully integrated with automatic routing.

### Overview

The Gemini Live API provides WebSocket-based bidirectional streaming for real-time interactions. Models that require the Live API (experimental and live models) are automatically detected and routed through WebSocket connections.

### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│ GeminiClient                                                 │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ REST API Clients (gemini-rust)                           │ │
│ │   "gemini-2.5-flash" ──► RateLimiter ──► TieredGemini   │ │
│ │   "gemini-2.5-pro"   ──► RateLimiter ──► TieredGemini   │ │
│ └──────────────────────────────────────────────────────────┘ │
│                                                               │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ Live API Client (WebSocket)                              │ │
│ │   GeminiLiveClient + LiveRateLimiter                     │ │
│ │     └─► Creates LiveSession per request                  │ │
│ └──────────────────────────────────────────────────────────┘ │
│                                                               │
│  Automatic Routing: is_live_model() checks for -live/-exp   │
└──────────────────────────────────────────────────────────────┘
```

**Key Components**:
- **`GeminiLiveClient`**: Factory for creating WebSocket sessions
- **`LiveSession`**: Active WebSocket connection for bidirectional communication
- **`LiveRateLimiter`**: Message-based rate limiting for persistent WebSocket connections
- **Automatic routing**: `GeminiClient` detects live models and uses WebSocket automatically

### Model Detection

Models are automatically routed to Live API if their name contains:
- `-live` (e.g., `gemini-2.0-flash-live`)
- `-exp` (e.g., `gemini-2.0-flash-exp`)

```rust
fn is_live_model(model_name: &str) -> bool {
    model_name.contains("-live") || model_name.contains("-exp")
}
```

### Usage

#### Unified Client (Automatic Routing)

The simplest approach - just specify a live model and `GeminiClient` handles routing:

```rust
use boticelli::{BoticelliDriver, GeminiClient, GenerateRequest, Message, Role, Input};

let client = GeminiClient::new()?;

// Automatically uses Live API (WebSocket)
let request = GenerateRequest {
    messages: vec![Message {
        role: Role::User,
        content: vec![Input::Text("Hello from Live API!".to_string())],
    }],
    model: Some("models/gemini-2.0-flash-exp".to_string()),
    max_tokens: Some(100),
    ..Default::default()
};

// Automatically routed to WebSocket Live API
let response = client.generate(&request).await?;
```

#### Streaming with Automatic Routing

Streaming also works automatically:

```rust
use boticelli::{BoticelliDriver, GeminiClient, Streaming};
use futures_util::StreamExt;

let client = GeminiClient::new()?;

let request = GenerateRequest {
    messages: vec![Message {
        role: Role::User,
        content: vec![Input::Text("Count to 5".to_string())],
    }],
    model: Some("models/gemini-2.0-flash-exp".to_string()),
    ..Default::default()
};

// Automatically uses WebSocket streaming
let mut stream = client.generate_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);
    if chunk.is_final {
        break;
    }
}
```

#### Direct Live API Client

For more control, use `GeminiLiveClient` directly:

```rust
use boticelli::{GeminiLiveClient, GenerationConfig};

// Create Live API client with rate limiting
let client = GeminiLiveClient::new_with_rate_limit(Some(10))?; // 10 RPM

// Configure generation parameters
let config = GenerationConfig {
    max_output_tokens: Some(100),
    temperature: Some(0.7),
    response_modalities: Some(vec!["TEXT".to_string()]),
    ..Default::default()
};

// Connect and perform handshake
let mut session = client
    .connect_with_config("models/gemini-2.0-flash-exp", config)
    .await?;

// Send messages
let response = session.send_text("Hello!").await?;
println!("Response: {}", response);

// Close session
session.close().await?;
```

#### Streaming with Live API Client

```rust
use futures_util::StreamExt;

let client = GeminiLiveClient::new()?;
let config = GenerationConfig::default();

let mut session = client
    .connect_with_config("models/gemini-2.0-flash-exp", config)
    .await?;

// send_text_stream consumes the session (stream owns the WebSocket)
let mut stream = session.send_text_stream("Tell me a story").await?;

while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result?;
    print!("{}", chunk.content);

    if chunk.is_final {
        println!("\n[Stream complete]");
        break;
    }
}

// Session automatically closed when stream is dropped
```

### Rate Limiting

Live API uses message-based rate limiting (not connection-based):

**Key Differences from REST API**:
- **REST API**: Tracks individual HTTP requests
- **Live API**: Tracks WebSocket messages over persistent connections

**Implementation**:
```rust
pub struct LiveRateLimiter {
    messages_sent: AtomicU32,
    window_start: Arc<Mutex<Instant>>,
    max_messages_per_minute: u32,
}
```

**Behavior**:
- Tracks messages sent in a 60-second rolling window
- Blocks proactively when limit would be exceeded
- Automatically resets window after 60 seconds
- Thread-safe via `Arc` for sharing across sessions

**Default Configuration**:
- Free tier: 10 messages per minute (matches REST API RPM)
- Rate limits are read from `TierConfig` automatically

**Custom Rate Limiting**:
```rust
// Create client with custom rate limit
let client = GeminiLiveClient::new_with_rate_limit(Some(5))?; // 5 messages/min

// Or create a standalone rate limiter
let limiter = LiveRateLimiter::new(10);

// Before sending each message
limiter.acquire().await;
// ... send message ...
limiter.record();
```

### WebSocket Protocol

The Live API uses a specific WebSocket message protocol:

**Connection Handshake**:
1. Connect to WebSocket endpoint with API key in query param
2. Send `setup` message with model and configuration
3. Wait for `setupComplete` response
4. Connection ready for bidirectional messages

**Message Exchange**:
- **Client → Server**: `clientContent` messages with conversation turns
- **Server → Client**: `serverContent` messages with model responses
- **Completion**: `turnComplete: true` indicates response is finished

**Message Types**:
- `SetupMessage`: Initial configuration
- `ClientContentMessage`: User messages
- `ServerMessage`: Model responses, errors, metadata
- See `src/models/gemini/live_protocol.rs` for complete definitions

### Error Handling

**Live API Specific Errors**:
```rust
pub enum GeminiErrorKind {
    // WebSocket-specific
    WebSocketConnection(String),    // Connection failed
    WebSocketHandshake(String),     // Setup failed
    InvalidServerMessage(String),   // Parse error
    ServerDisconnect(String),       // Server sent goAway
    StreamInterrupted(String),      // Stream closed unexpectedly

    // General errors (shared with REST API)
    MissingApiKey,
    ApiRequest(String),
    // ...
}
```

**Error Recovery**:
- Connection failures: Retry with exponential backoff (user responsibility)
- Invalid messages: Logged and propagated as errors
- Stream interruptions: Clean up resources automatically
- Rate limiting: Proactive (waits before sending, doesn't error)

### Testing

Live API has comprehensive test coverage:

**Basic Functionality** (`tests/gemini_live_basic_test.rs`):
- `test_live_api_connection`: WebSocket connection and handshake
- `test_live_api_basic_generation`: Simple text generation
- `test_live_api_streaming`: Streaming with multiple chunks
- `test_live_api_multiple_turns`: Multiple messages in one session

**Integration** (`tests/gemini_live_integration_test.rs`):
- `test_gemini_client_routes_to_live_api`: Automatic routing from `GeminiClient`
- `test_gemini_client_streaming_routes_to_live_api`: Streaming routing
- `test_gemini_client_detects_live_models`: Model detection logic

**Error Handling** (`tests/gemini_live_error_test.rs`):
- `test_live_api_invalid_model`: Invalid model handling
- `test_live_api_rate_limiting`: Rate limiter blocking behavior
- `test_live_api_empty_message`: Edge case handling
- `test_live_api_very_long_message`: Large message handling
- `test_streaming_error_recovery`: Stream interruption recovery

**Run Live API tests** (requires `GEMINI_API_KEY`):
```bash
# All Live API tests
cargo test --features gemini,api live

# Specific test files
cargo test --features gemini,api --test gemini_live_basic_test
cargo test --features gemini,api --test gemini_live_integration_test
cargo test --features gemini,api --test gemini_live_error_test
```

### Rate Limit Comparison

**Free Tier Observations** (based on empirical testing):

| Metric | REST API | Live API | Notes |
|--------|----------|----------|-------|
| RPM | 10 | 10 | Messages per minute |
| TPM | 250,000 | ? | Tokens per minute |
| RPD | 250 | ? | Requests per day |
| Connection | Stateless | Persistent | WebSocket stays open |

**Live API Benefits**:
- Lower latency for multiple messages (persistent connection)
- True bidirectional streaming
- Better for real-time interactions
- Potentially higher effective throughput (no connection overhead)

**When to Use Live API**:
- ✅ Real-time streaming responses
- ✅ Multiple rapid exchanges
- ✅ Low-latency requirements
- ✅ Bidirectional communication

**When to Use REST API**:
- ✅ Single request/response
- ✅ Batch processing
- ✅ Simpler deployment (no WebSocket support needed)
- ✅ Better for serverless environments

### Implementation Files

**Core Implementation**:
- `src/models/gemini/live_client.rs`: WebSocket client and session management
- `src/models/gemini/live_protocol.rs`: Message type definitions
- `src/models/gemini/live_rate_limit.rs`: Message-based rate limiting
- `src/models/gemini/client.rs`: Unified client with automatic routing

**Tests**:
- `tests/gemini_live_basic_test.rs`: Basic Live API functionality
- `tests/gemini_live_integration_test.rs`: Integration with `GeminiClient`
- `tests/gemini_live_error_test.rs`: Error handling and edge cases

**Documentation**:
- `GEMINI_STREAMING.md`: Implementation plan and WebSocket protocol details
- `GEMINI.md`: This file - complete architecture and usage guide

### Future Enhancements

**Potential Improvements**:
1. **Audio/Video Support**: Add support for realtime audio/video streaming
2. **Tool Calling**: Implement function calling over WebSocket
3. **Session Persistence**: Add support for session resumption
4. **Connection Pooling**: Reuse WebSocket connections across requests
5. **Adaptive Rate Limiting**: Learn actual rate limits from API responses
6. **Metrics**: Track message counts, latency, error rates

## References

- **gemini-rust crate**: <https://docs.rs/gemini-rust>
- **Gemini API docs**: <https://ai.google.dev/gemini-api/docs/models/gemini>
- **Gemini Live API docs**: <https://ai.google.dev/gemini-api/docs/live>
- **Governor (rate limiting)**: <https://docs.rs/governor>
- **tokio-tungstenite (WebSocket)**: <https://docs.rs/tokio-tungstenite>
- **CLAUDE.md**: Project coding standards and patterns
