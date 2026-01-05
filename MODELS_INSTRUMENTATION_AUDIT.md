# Models Crate Instrumentation Audit

## Overview

All public functions must have `#[instrument]` attributes for observability. This audit identifies missing instrumentation in the `botticelli_models` crate.

## Critical: Missing Instrumentation

### Gemini Module

#### `gemini/client/core.rs`
- [ ] `pub fn capabilities()` - missing #[instrument]
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub fn new_with_tier()` - missing #[instrument]
- [ ] `pub fn new_with_retry()` - missing #[instrument]
- [ ] `pub fn new_with_config()` - missing #[instrument]
- [ ] `pub fn set_default_model()` - missing #[instrument]

#### `gemini/client/tiered.rs`
- [ ] `pub fn new()` - missing #[instrument]

#### `gemini/live_protocol.rs`
- [ ] `pub fn text()` - missing #[instrument]
- [ ] `pub fn as_text()` - missing #[instrument]
- [ ] `pub fn is_setup_complete()` - missing #[instrument]
- [ ] `pub fn is_server_content()` - missing #[instrument]
- [ ] `pub fn is_tool_call()` - missing #[instrument]
- [ ] `pub fn is_go_away()` - missing #[instrument]
- [ ] `pub fn extract_text()` - missing #[instrument]
- [ ] `pub fn is_turn_complete()` - missing #[instrument]

#### `gemini/live_rate_limit.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub async fn acquire()` - missing #[instrument]
- [ ] `pub fn record()` - missing #[instrument]
- [ ] `pub fn current_count()` - missing #[instrument]
- [ ] `pub fn max_per_minute()` - missing #[instrument]

#### `gemini/capabilities.rs`
- [ ] `pub fn standard()` - missing #[instrument]
- [ ] `pub fn embedding()` - missing #[instrument]
- [ ] `pub fn supports_streaming()` - missing #[instrument]
- [ ] `pub fn supports_tool_calling()` - missing #[instrument]
- [ ] `pub fn supports_vision()` - missing #[instrument]
- [ ] `pub fn supports_video()` - missing #[instrument]
- [ ] `pub fn supports_audio()` - missing #[instrument]
- [ ] `pub fn supports_documents()` - missing #[instrument]
- [ ] `pub fn supports_json_mode()` - missing #[instrument]
- [ ] `pub fn supports_token_counting()` - missing #[instrument]

#### `gemini/live_client.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub fn new_with_rate_limit()` - missing #[instrument]
- [ ] `pub async fn connect()` - missing #[instrument]
- [ ] `pub async fn connect_with_config()` - missing #[instrument]
- [ ] `pub async fn send_text()` - missing #[instrument]
- [ ] `pub async fn send_text_stream()` - missing #[instrument]
- [ ] `pub async fn close()` - missing #[instrument]
- [ ] `pub fn model()` - missing #[instrument]

### Ollama Module

#### `ollama/conversion.rs`
- [ ] `pub fn messages_to_prompt()` - missing #[instrument]
- [ ] `pub fn response_to_output()` - missing #[instrument]

#### `ollama/client.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub fn new_with_url()` - missing #[instrument]
- [ ] `pub async fn validate()` - missing #[instrument]
- [ ] `pub async fn ensure_model()` - missing #[instrument]

### Anthropic Module

#### `anthropic/types.rs`
- [ ] `pub fn from_mcp()` - missing #[instrument]
- [ ] `pub fn builder()` (multiple) - missing #[instrument]

#### `anthropic/client.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub async fn generate_anthropic()` - missing #[instrument]

### HuggingFace Module

#### `huggingface/driver.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub fn with_api_token()` - missing #[instrument]

### Groq Module

#### `groq/driver.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub fn with_api_key()` - missing #[instrument]

### OpenAI Module

#### `openai/dto.rs`
- [ ] `pub fn builder()` - missing #[instrument]

#### `openai/conversions.rs`
- [ ] `pub fn to_chat_request()` - missing #[instrument]
- [ ] `pub fn from_chat_response()` - missing #[instrument]

#### `openai/client.rs`
- [ ] `pub fn new()` - missing #[instrument]
- [ ] `pub async fn generate()` - missing #[instrument]

### Metrics Module

#### `metrics.rs`
- [ ] `pub fn get()` - missing #[instrument]
- [ ] `pub fn record_request()` - missing #[instrument]
- [ ] `pub fn record_error()` - missing #[instrument]
- [ ] `pub fn record_tokens()` - missing #[instrument]
- [ ] `pub fn classify_error()` - missing #[instrument]

### Token Counting Module

#### `token_counting.rs`
- [ ] `pub fn claude_tokenizer()` - missing #[instrument]

## Instrumentation Guidelines

### Required for All Public Functions
```rust
#[instrument(skip(large_param), fields(context_field))]
pub fn my_function(small_param: u32, large_param: &LargeStruct) -> Result<T, E> {
    debug!("Function entry");
    // implementation
}
```

### Skip Guidelines
- Skip connection objects: `skip(conn, client)`
- Skip large data: `skip(data, buffer)`
- Skip sensitive data: `skip(api_key, password)`

### Field Guidelines
- Include context: `fields(model_name, request_id)`
- Track counts: `fields(message_count, token_count)`
- Track IDs: `fields(user_id, session_id)`

### Event Guidelines
- `debug!()` - Function entry/exit, state changes
- `info!()` - Major operations (API calls, model loading)
- `warn!()` - Recoverable issues (retries, fallbacks)
- `error!()` - Errors before return

## Priority

1. **High Priority** - Async functions (API calls, I/O operations)
2. **Medium Priority** - Constructors and configuration functions
3. **Low Priority** - Simple getters/accessors (but still required)

## Notes

- Every public function violation is a defect
- Instrumentation is critical for debugging in production
- Proper spans enable distributed tracing
- Structured fields enable log aggregation and analysis
