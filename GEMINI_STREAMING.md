# Gemini Streaming Implementation Plan

## Overview

This document outlines the plan to implement streaming support for Gemini "live" models (e.g., `gemini-2.0-flash-live`) in the Boticelli library. Currently, the library works with all Gemini models except those requiring streaming capabilities.

**Date**: 2025-01-17  
**Status**: Planning Phase  
**Priority**: Medium  
**Complexity**: High

---

## Current State

### What Works
- ✅ All non-streaming Gemini models (2.0-flash, 2.5-flash, 2.5-pro, etc.)
- ✅ Rate limiting per model
- ✅ Model pooling with lazy initialization
- ✅ Multi-model narratives
- ✅ Vision support (base64 images)
- ✅ Async operations with tokio

### What Doesn't Work
- ❌ Gemini Live models (require bidirectional streaming)
- ❌ Real-time interaction models
- ❌ Streaming response handling

### Current Architecture

```
┌──────────────────┐
│ GeminiClient     │
│ (BoticelliDriver)│
└────────┬─────────┘
         │
         ├─> ModelPool (HashMap<model_name, ClientWrapper>)
         │   └─> RateLimiter -> gemini_rust::Gemini
         │
         └─> generate() -> GenerateResponse (blocking, returns complete response)
```

---

## Problem Analysis

### What Are "Live" Models?

Gemini Live models (like `gemini-2.0-flash-live`) are designed for:
- Real-time conversational AI
- Voice/audio interactions
- Bidirectional streaming (client sends, server responds incrementally)
- Low-latency responses

### Technical Requirements

1. **Bidirectional Streaming**: Client and server both stream data
2. **Incremental Responses**: Server sends partial responses as they're generated
3. **Connection Management**: Long-lived connections with proper cleanup
4. **Backpressure Handling**: Manage flow control between sender/receiver
5. **Error Handling**: Graceful degradation for connection issues

### Current Limitations

1. **gemini_rust Library**: May not support streaming (needs verification)
2. **BoticelliDriver Trait**: Returns complete `GenerateResponse`, not streaming
3. **Narrative Executor**: Expects complete responses, not incremental
4. **Rate Limiting**: Designed for request/response, not streaming sessions

---

## Investigation Phase

### Step 1: Assess gemini_rust Library (PRIORITY)

**Task**: Determine if `gemini_rust` supports streaming

```bash
# Check gemini_rust source/docs
cd ~/.cargo/registry/src/
find . -name "gemini_rust*" -type d
# Review streaming capabilities
```

**Questions to Answer**:
- Does `gemini_rust` expose streaming APIs?
- What format does it use? (AsyncStream, Stream trait, channels?)
- Does it support bidirectional streaming?
- What's the API surface?

**Outcomes**:
- **If YES**: Proceed with Phase 2 (implement streaming in Boticelli)
- **If NO**: Need to fork/extend gemini_rust OR use direct HTTP/gRPC

### Step 2: Research Gemini Live API

**Documentation**:
- [Gemini API Docs](https://ai.google.dev/api/generate-content)
- WebSocket or Server-Sent Events (SSE)?
- Authentication with streaming
- Message format (Protocol Buffers? JSON?)

**Key Questions**:
- What protocol does Gemini Live use? (WebSocket, SSE, gRPC)
- What's the message format?
- How is authentication handled?
- What are rate limits for streaming vs. non-streaming?

---

## Implementation Strategy

### Phase 1: Extend BoticelliDriver Trait (Foundation)

**Goal**: Add streaming capability alongside existing blocking API

#### Option A: New Trait Method (Recommended)

```rust
#[async_trait]
pub trait BoticelliDriver: Send + Sync {
    // Existing method (unchanged)
    async fn generate(&self, request: &GenerateRequest) -> BoticelliResult<GenerateResponse>;
    
    // New streaming method
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> BoticelliResult<Pin<Box<dyn Stream<Item = BoticelliResult<StreamChunk>> + Send>>>;
    
    // Optional: Check if model supports streaming
    fn supports_streaming(&self, model: &str) -> bool {
        false  // Default: no streaming
    }
}

/// Incremental response chunk from streaming API
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub text: String,
    pub finished: bool,
    pub metadata: Option<ChunkMetadata>,
}

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub tokens_generated: Option<u32>,
    pub finish_reason: Option<String>,
}
```

**Pros**:
- Backward compatible (existing code unchanged)
- Clear separation of streaming vs. blocking
- Opt-in for drivers that support streaming

**Cons**:
- Drivers must implement both methods (or provide default)
- Consumers must handle two code paths

#### Option B: Callback-Based (Alternative)

```rust
#[async_trait]
pub trait BoticelliDriver: Send + Sync {
    async fn generate(&self, request: &GenerateRequest) -> BoticelliResult<GenerateResponse>;
    
    async fn generate_with_callback<F>(
        &self,
        request: &GenerateRequest,
        on_chunk: F,
    ) -> BoticelliResult<GenerateResponse>
    where
        F: Fn(StreamChunk) + Send + Sync;
}
```

**Pros**:
- Still returns complete response at end
- Allows progressive updates during generation

**Cons**:
- Less flexible than Stream
- Harder to compose with other async code

**Recommendation**: Use **Option A** (Stream-based) for maximum flexibility.

---

### Phase 2: Implement Streaming in GeminiClient

#### 2.1: Add Streaming Support to ModelClientWrapper

```rust
struct ModelClientWrapper {
    client: Gemini,
    rate_limiter: Option<RateLimiter>,
    supports_streaming: bool,  // New field
}

impl ModelClientWrapper {
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> BoticelliResult<impl Stream<Item = BoticelliResult<StreamChunk>>> {
        // Apply rate limiting
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await?;
        }
        
        // Call gemini_rust streaming API
        let stream = self.client.generate_content_stream(/* ... */)?;
        
        // Transform gemini_rust stream into BoticelliResult<StreamChunk>
        Ok(stream.map(|result| {
            result
                .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())).into())
                .and_then(|chunk| convert_gemini_chunk_to_boticelli(chunk))
        }))
    }
}
```

#### 2.2: Detect Streaming-Capable Models

```rust
impl GeminiClient {
    fn model_supports_streaming(model_name: &str) -> bool {
        model_name.contains("-live") || model_name.contains("-streaming")
    }
    
    fn parse_model_name(name: &str) -> (Model, bool) {
        let supports_streaming = Self::model_supports_streaming(name);
        let model = match name {
            "gemini-2.0-flash-live" => Model::Custom("models/gemini-2.0-flash-live"),
            // ... other models
        };
        (model, supports_streaming)
    }
}
```

#### 2.3: Implement BoticelliDriver::generate_stream

```rust
#[async_trait]
impl BoticelliDriver for GeminiClient {
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> BoticelliResult<Pin<Box<dyn Stream<Item = BoticelliResult<StreamChunk>> + Send>>> {
        let model_name = self.resolve_model_name(request);
        
        // Check if model supports streaming
        if !Self::model_supports_streaming(&model_name) {
            return Err(GeminiError::new(GeminiErrorKind::StreamingNotSupported(model_name)).into());
        }
        
        let wrapper = self.get_or_create_client(&model_name).await?;
        let stream = wrapper.generate_stream(request).await?;
        
        Ok(Box::pin(stream))
    }
    
    fn supports_streaming(&self, model: &str) -> bool {
        Self::model_supports_streaming(model)
    }
}
```

---

### Phase 3: Update Narrative Executor (Optional)

**Decision Point**: Do we need streaming in narrative execution?

#### Scenario A: No Narrative Streaming (Simpler)

- Narratives continue using blocking `generate()`
- Streaming is opt-in for custom code
- No changes needed to narrative executor

#### Scenario B: Progressive Narrative Execution (Advanced)

- Allow narratives to show progress as content generates
- Useful for long generations
- Requires executor changes

```rust
// Example: Stream-aware executor
impl<D: BoticelliDriver> NarrativeExecutor<D> {
    pub async fn execute_with_progress<N, F>(
        &self,
        narrative: &N,
        on_progress: F,
    ) -> BoticelliResult<NarrativeExecution>
    where
        N: NarrativeProvider,
        F: Fn(&str, &StreamChunk) + Send + Sync,
    {
        // For each act, if model supports streaming, use generate_stream
        // Otherwise fall back to generate()
        // ...
    }
}
```

**Recommendation**: Start with **Scenario A**. Add Scenario B later if needed.

---

### Phase 4: Testing Strategy

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    
    #[tokio::test]
    async fn test_streaming_basic() {
        let client = GeminiClient::new().unwrap();
        
        let request = GenerateRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![Input::Text("Count to 10".to_string())],
            }],
            model: Some("gemini-2.0-flash-live".to_string()),
            ..Default::default()
        };
        
        let mut stream = client.generate_stream(&request).await.unwrap();
        let mut chunks = Vec::new();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.unwrap();
            chunks.push(chunk.text.clone());
            
            if chunk.finished {
                break;
            }
        }
        
        assert!(!chunks.is_empty());
        let full_text = chunks.join("");
        assert!(full_text.contains("1"));
    }
    
    #[tokio::test]
    async fn test_non_streaming_model_returns_error() {
        let client = GeminiClient::new().unwrap();
        
        let request = GenerateRequest {
            model: Some("gemini-2.0-flash".to_string()),  // Non-streaming
            ..Default::default()
        };
        
        let result = client.generate_stream(&request).await;
        assert!(result.is_err());
    }
}
```

#### Integration Tests

```rust
#[tokio::test]
async fn test_streaming_rate_limiting() {
    // Verify rate limiter works with streaming
}

#[tokio::test]
async fn test_streaming_error_handling() {
    // Verify graceful handling of mid-stream errors
}

#[tokio::test]
async fn test_streaming_cancellation() {
    // Verify stream cleanup on early termination
}
```

---

## Error Handling

### New Error Types

```rust
pub enum GeminiErrorKind {
    // Existing variants...
    
    /// Model doesn't support streaming
    StreamingNotSupported(String),
    
    /// Stream was interrupted
    StreamInterrupted(String),
    
    /// Stream exceeded timeout
    StreamTimeout,
}
```

### Error Scenarios

1. **Model doesn't support streaming**: Return error immediately
2. **Connection drops mid-stream**: Wrap in StreamInterrupted error
3. **Rate limit hit during stream**: Pause stream, resume when allowed
4. **Client cancels stream**: Clean up resources properly

---

## Rate Limiting Considerations

### Challenge

Current rate limiter is designed for discrete requests. Streaming sessions may:
- Last minutes (not milliseconds)
- Generate many tokens incrementally
- Need different RPM/TPM accounting

### Solutions

#### Option 1: Count Stream as Single Request

```rust
// Acquire rate limit token at start of stream
limiter.acquire().await?;

// Stream proceeds without further checks
// Tokens counted at end of stream
```

**Pros**: Simple  
**Cons**: Doesn't account for long-running streams

#### Option 2: Periodic Rate Limit Checks

```rust
// Check rate limit every N chunks or N seconds
let mut chunk_count = 0;
while let Some(chunk) = stream.next().await {
    chunk_count += 1;
    
    if chunk_count % 100 == 0 {
        limiter.check_and_wait().await?;
    }
    
    yield chunk;
}
```

**Pros**: Better accounting  
**Cons**: More complex, may interrupt flow

**Recommendation**: Start with **Option 1**, monitor usage, add Option 2 if needed.

---

## Migration Path

### Phase 1: Foundation (Week 1-2)
- [ ] Investigate gemini_rust streaming capabilities
- [ ] Research Gemini Live API protocol
- [ ] Design BoticelliDriver streaming extension
- [ ] Create proof-of-concept with direct HTTP if needed

### Phase 2: Core Implementation (Week 3-4)
- [ ] Extend BoticelliDriver trait
- [ ] Implement streaming in GeminiClient
- [ ] Add model detection for streaming support
- [ ] Write unit tests

### Phase 3: Integration (Week 5)
- [ ] Test with real Gemini Live models
- [ ] Add integration tests
- [ ] Document usage in GEMINI.md
- [ ] Add examples

### Phase 4: Polish (Week 6)
- [ ] Rate limiting refinements
- [ ] Error handling improvements
- [ ] Performance testing
- [ ] User guide for streaming

---

## Open Questions

1. **Does gemini_rust support streaming?**
   - If no: Do we fork it, contribute upstream, or implement direct HTTP?

2. **What protocol does Gemini Live use?**
   - WebSocket? SSE? gRPC?
   - This affects implementation significantly

3. **How do Live models authenticate?**
   - Same API key as regular models?
   - Different endpoints?

4. **Are there different rate limits for streaming?**
   - Same RPM/TPM as non-streaming?
   - Per-session limits?

5. **Do we need streaming in narratives?**
   - Or just for custom code/CLI?
   - Impacts executor changes

6. **Should we support cancellation?**
   - Allow users to stop streams mid-generation?
   - How does this interact with rate limiting?

---

## Success Criteria

### Minimum Viable Product (MVP)

- [ ] Can successfully connect to gemini-2.0-flash-live model
- [ ] Receive incremental responses as stream
- [ ] Stream completes successfully
- [ ] Errors handled gracefully
- [ ] Basic rate limiting works

### Full Implementation

- [ ] All streaming-capable Gemini models work
- [ ] Rate limiting properly accounts for streaming
- [ ] Comprehensive tests (unit + integration)
- [ ] Documentation with examples
- [ ] CLI supports streaming (optional)
- [ ] Backward compatible with existing code

---

## Resources

### Documentation
- [Gemini API Reference](https://ai.google.dev/api/generate-content)
- [Tokio Streams](https://docs.rs/tokio-stream/latest/tokio_stream/)
- [Async Streams in Rust](https://rust-lang.github.io/async-book/05_streams/01_chapter.html)

### Libraries
- `tokio-stream`: Stream utilities
- `futures`: Stream trait and combinators
- `async-stream`: Macro for creating streams
- `pin-project`: Pin projection for streams

### Similar Implementations
- OpenAI Rust SDK (streaming support)
- Anthropic Rust SDK (streaming support)
- gRPC Rust examples (bidirectional streaming)

---

## Notes

- Start with read-only investigation of gemini_rust
- Prototype with smallest possible change
- Consider backward compatibility at each step
- Document learnings as we go
- Update this document with findings

---

## Timeline Estimate

- **Investigation**: 1-2 weeks
- **Design & Prototyping**: 1-2 weeks
- **Implementation**: 2-3 weeks
- **Testing & Documentation**: 1 week
- **Total**: 5-8 weeks for full implementation

**Fast Track** (MVP only): 3-4 weeks
