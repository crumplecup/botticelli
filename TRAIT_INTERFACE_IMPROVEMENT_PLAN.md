# Trait Interface Improvement Plan

## Executive Summary

The Botticelli project aims to provide a unified, trait-based interface (`BotticelliDriver`) that allows chat clients to work with any LLM provider. While the core architecture is sound, critical gaps prevent true provider interoperability:

1. **Tool calling is broken** - Tools work via `GenerateRequest` field, but `ToolUse` trait is unimplemented
2. **Duplicate type definitions** - Two incompatible `ToolDefinition` types exist
3. **Fake implementations** - Some providers claim capabilities they don't have (streaming)
4. **Missing implementations** - Anthropic lacks streaming, no provider implements advanced traits
5. **No capability discovery** - Cannot query which features a provider supports at runtime

This plan provides a roadmap to fix these issues and achieve the architectural vision of full provider interoperability.

---

## Current State Analysis

### Vision vs Reality

**Architectural Vision:**
- Single `BotticelliDriver` trait for all providers
- Optional capability traits (Streaming, Vision, Audio, Tools, etc.)
- Chat clients work with any provider via common interface
- Runtime capability discovery
- Consistent behavior across providers

**Current Reality:**
- ✅ Core `BotticelliDriver` fully implemented by all providers
- ⚠️ Tool support bypasses `ToolUse` trait entirely
- ❌ Duplicate `ToolDefinition` types create ambiguity
- ❌ Fake streaming implementations (Groq, HuggingFace)
- ❌ Missing Anthropic streaming despite API support
- ❌ No capability discovery mechanism
- ❌ Most capability traits have zero implementations

### Where Vision Matches Codebase

**✅ Core Generation Interface**
- All 6 providers implement `BotticelliDriver`
- `generate()` method works consistently
- Request/response types are unified (`GenerateRequest`, `GenerateResponse`)
- Provider identification via `provider_name()` and `model_name()`
- Rate limiting abstraction works

**✅ Token Counting**
- All 6 providers implement `TokenCounting` trait
- Consistent interface via `count_tokens()` method
- Approximations work reasonably well (tiktoken-based)

**✅ Type System**
- Unified `Input`, `Output`, `Message`, `Role` types
- `StopReason` enum properly standardized
- `ToolCall` structure defined consistently

### Where Vision Diverges from Codebase

**❌ Tool Calling Architecture**

*Problem:* Two parallel, incompatible approaches exist:

1. **GenerateRequest Field Approach** (what's actually used):
   ```rust
   // In GenerateRequest
   tools: Option<Vec<ToolDefinition>>  // botticelli_core type

   // Anthropic implementation
   if let Some(tools) = request.tools() {
       let anthropic_tools = tools.iter()
           .map(AnthropicTool::from_mcp)
           .collect();
       builder = builder.tools(Some(anthropic_tools));
   }
   ```

2. **ToolUse Trait Approach** (defined but unused):
   ```rust
   pub trait ToolUse: BotticelliDriver {
       async fn generate_with_tools(
           &self,
           req: &GenerateRequest,
           tools: &[ToolDefinition],  // botticelli_interface type
       ) -> BotticelliResult<GenerateResponse>;
   }
   ```

*Impact:*
- Code calling `generate_with_tools()` fails (method doesn't exist)
- Two `ToolDefinition` types with different field names (`input_schema` vs `parameters`)
- Serialization failures between types
- Architectural inconsistency

**❌ Streaming Implementations**

*Problem:* Three different streaming states:

1. **True Streaming** (Gemini, Ollama):
   - Incremental chunks arrive over time
   - Each chunk has partial content
   - Last chunk marked `is_final=true`

2. **Fake Streaming** (Groq, HuggingFace):
   ```rust
   let response = self.generate(req).await?;  // Get full response
   let chunks = response.outputs().iter().map(|output| {
       StreamChunk::builder()
           .content(output.clone())
           .is_final(true)  // ALL chunks marked final!
           .build()
   });
   ```
   - Fetches entire response first
   - Wraps in fake stream
   - All chunks marked final immediately

3. **Not Implemented** (Anthropic):
   - API supports streaming but trait not implemented
   - Trait exists but returns `Err(Unimplemented)`

*Impact:*
- Clients expecting incremental updates get full response at once
- Cannot distinguish real vs fake streaming
- Anthropic streaming unavailable despite API support

**❌ Capability Discovery**

*Problem:* No way to check if a provider supports a capability:

```rust
// What we want but can't do:
if provider.supports::<Streaming>() {
    let stream = provider.generate_stream(req).await?;
}

// What we have to do instead:
match provider.generate_stream(req).await {
    Ok(stream) => { /* use stream */ },
    Err(BotticelliError::Unimplemented) => {
        // Fall back to non-streaming
        let response = provider.generate(req).await?;
    },
    Err(e) => return Err(e),
}
```

*Impact:*
- Cannot query capabilities before attempting use
- Defensive error handling required everywhere
- No way to select provider based on capabilities

**❌ Incomplete Trait Coverage**

Current implementation status:

| Trait | Anthropic | Gemini | Ollama | Groq | HuggingFace |
|-------|-----------|--------|--------|------|-------------|
| BotticelliDriver | ✅ | ✅ | ✅ | ✅ | ✅ |
| TokenCounting | ✅ | ✅ | ✅ | ✅ | ✅ |
| Streaming | ❌ | ✅ | ✅ | ~fake | ~fake |
| Vision | ❌ | ✅ | ❌ | ❌ | ❌ |
| Metadata | ❌ | ✅ | ❌ | ❌ | ❌ |
| ToolUse (trait) | ❌ | ❌ | ❌ | ❌ | ❌ |
| Audio | ❌ | ❌ | ❌ | ❌ | ❌ |
| Video | ❌ | ❌ | ❌ | ❌ | ❌ |
| JsonMode | ❌ | ❌ | ❌ | ❌ | ❌ |
| Embeddings | ❌ | ❌ | ❌ | ❌ | ❌ |
| BatchGeneration | ❌ | ❌ | ❌ | ❌ | ❌ |
| Health | ❌ | ❌ | ❌ | ❌ | ❌ |

*Impact:*
- Most capability traits are theoretical only
- Cannot write portable code using advanced features
- Traits don't guide implementation

---

## Critical Issues Preventing Interoperability

### Issue 1: Tool Calling Type Conflict ⚠️ BLOCKER

**Severity:** CRITICAL - Prevents tool calling from working correctly

**Problem:**
Two incompatible `ToolDefinition` types exist:

```rust
// botticelli_core/src/tool_definition.rs
pub struct ToolDefinition {
    name: String,
    description: String,
    input_schema: Value,  // ← Field name
}

// botticelli_interface/src/types.rs
pub struct ToolDefinition {
    name: String,
    description: String,
    parameters: Value,  // ← Different field name!
}
```

Both are re-exported at crate level, creating ambiguity:
```rust
// botticelli_core/src/lib.rs
pub use tool_definition::ToolDefinition;

// botticelli_interface/src/lib.rs (types.rs)
pub use types::ToolDefinition;
```

**Impact:**
- Serialization fails when types are mixed
- `GenerateRequest` uses core version, `ToolUse` trait uses interface version
- Compiler warnings about ambiguous glob re-exports
- JSON schemas differ between types

**Root Cause:**
Tool calling was retrofitted via `GenerateRequest` field after interface trait was already defined.

---

### Issue 2: ToolUse Trait Bypassed ⚠️ BLOCKER

**Severity:** CRITICAL - Architectural inconsistency

**Problem:**
`ToolUse` trait exists but no provider implements it:

```rust
// Trait definition (botticelli_interface/src/traits.rs:169-192)
pub trait ToolUse: BotticelliDriver {
    async fn generate_with_tools(
        &self,
        req: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse>;
}

// What actually happens (botticelli_models/src/anthropic/client.rs)
impl BotticelliDriver for AnthropicClient {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        // Tools come from req.tools(), not a separate parameter
        let anthropic_request = self.convert_request(req)?;  // Handles tools internally
        // ...
    }
}

// ToolUse trait: NOT IMPLEMENTED
```

**Impact:**
- Code expecting `generate_with_tools()` method fails
- `TuiLlmBackend` works around this by implementing `LlmBackend::generate_with_tools()` which calls `driver.generate()`
- Tools and messages are separated in trait but combined in request
- No standard interface for tool calling

---

### Issue 3: Fake Streaming Implementations ⚠️ MAJOR

**Severity:** MAJOR - Misleading behavior

**Problem:**
Groq and HuggingFace claim to implement `Streaming` trait but just wrap synchronous responses:

```rust
// crates/botticelli_models/src/groq/driver.rs:98-124
impl Streaming for GroqDriver {
    async fn generate_stream(&self, req: &GenerateRequest) -> BotticelliResult<StreamBox> {
        // Fetch entire response synchronously
        let response = self.generate(req).await?;

        // Convert to fake stream - all chunks arrive immediately
        let chunks: Vec<BotticelliResult<StreamChunk>> = response
            .outputs()
            .iter()
            .map(|output| {
                StreamChunk::builder()
                    .content(output.clone())
                    .is_final(true)  // Mark ALL chunks as final
                    .build()
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Return stream that yields all chunks immediately
        Ok(Box::pin(futures_util::stream::iter(chunks)))
    }
}
```

**Impact:**
- Clients expecting incremental chunks get entire response at once
- Cannot distinguish real vs fake streaming without testing
- Misleading performance characteristics
- Wastes resources converting to stream format

---

### Issue 4: Missing Anthropic Streaming ⚠️ MAJOR

**Severity:** MAJOR - Missing critical feature

**Problem:**
Anthropic API supports streaming but it's not implemented:

```rust
// crates/botticelli_models/src/anthropic/client.rs
impl BotticelliDriver for AnthropicClient {
    // generate() exists
}

// Streaming trait: NOT IMPLEMENTED
// But Anthropic API has:
// POST https://api.anthropic.com/v1/messages
// with "stream": true
```

**Impact:**
- Cannot use streaming with Claude models
- Poor UX for long responses
- Competitive disadvantage vs other providers

---

### Issue 5: No Capability Discovery ⚠️ MAJOR

**Severity:** MAJOR - Prevents runtime adaptation

**Problem:**
No standard way to check if a provider supports a capability:

```rust
// What we want:
fn select_provider(providers: &[Box<dyn BotticelliDriver>]) -> &dyn BotticelliDriver {
    providers.iter()
        .find(|p| p.supports::<Vision>() && p.supports::<Streaming>())
        .unwrap_or(&providers[0])
}

// What we can't do because supports() doesn't exist
```

**Impact:**
- Cannot select provider based on capabilities
- Cannot gracefully degrade features
- Must use defensive error handling everywhere
- Cannot auto-configure based on provider

---

## Strategic Plan

### Phase 1: Resolve Type Conflicts (CRITICAL)

**Goal:** Single source of truth for tool definitions

**Tasks:**

1. **Unify ToolDefinition Types**
   - Keep `botticelli_core::ToolDefinition` as canonical (already used by providers)
   - Delete `botticelli_interface::types::ToolDefinition`
   - Update all references to import from core
   - Fix ambiguous glob re-exports

2. **Standardize Field Name**
   - Use `input_schema` (matches MCP spec and current usage)
   - Update any code using `parameters` field name
   - Add deprecation notice if needed

**Success Criteria:**
- ✅ Single ToolDefinition type in botticelli_core
- ✅ No ambiguous re-exports
- ✅ All providers use same type
- ✅ Serialization works consistently

**Files Modified:**
- Delete: `crates/botticelli_interface/src/types.rs` ToolDefinition (lines 63-82)
- Update: `crates/botticelli_interface/src/lib.rs` exports
- Update: All imports of ToolDefinition

**Estimated Effort:** 1 session

---

### Phase 2: Deprecate or Align ToolUse Trait (CRITICAL)

**Goal:** Remove architectural inconsistency

**Two Options:**

#### Option A: Remove ToolUse Trait (RECOMMENDED)

**Rationale:**
- Tools are inherently part of generation, not separate
- `GenerateRequest.tools()` field works well
- No provider implements the trait
- Simpler architecture

**Tasks:**
1. Mark `ToolUse` trait as deprecated
2. Add documentation explaining `GenerateRequest.tools()` is preferred
3. Remove trait in next major version
4. Update `LlmBackend` to not reference `generate_with_tools()`

#### Option B: Implement ToolUse Trait Properly

**Rationale:**
- Separates tool-capable from non-tool-capable providers
- Explicit trait bounds for type safety
- Clearer API

**Tasks:**
1. Align trait signature with current usage:
   ```rust
   pub trait ToolUse: BotticelliDriver {
       async fn generate_with_tools(
           &self,
           req: &GenerateRequest,
       ) -> BotticelliResult<GenerateResponse> {
           // Default: delegate to generate()
           self.generate(req).await
       }
   }
   ```
2. Implement for Anthropic (trivial - just delegate)
3. Update TuiLlmBackend to use trait

**Recommendation:** Option A (remove trait)
- Simpler
- Matches actual usage pattern
- Tools are optional, not a separate capability

**Success Criteria:**
- ✅ No ambiguity about how to call with tools
- ✅ Documentation clear
- ✅ Code consistent

**Estimated Effort:** 1 session

---

### Phase 3: Fix Streaming Implementations (MAJOR)

**Goal:** True streaming where claimed, honest about capabilities where not

**Tasks:**

1. **Implement Anthropic Streaming**
   - Add `Streaming` trait implementation for `AnthropicClient`
   - Use Anthropic's SSE streaming API
   - Parse incremental chunks
   - Handle errors and reconnection

2. **Remove Fake Streaming**
   - Remove `Streaming` impl from Groq and HuggingFace
   - Document that these providers don't support streaming
   - Add note about using non-streaming `generate()` instead

3. **Add Streaming Capability Detection**
   - Add `fn supports_streaming(&self) -> bool` to `BotticelliDriver`
   - Implement for all providers
   - Update documentation

**Success Criteria:**
- ✅ Anthropic implements true streaming
- ✅ Groq/HuggingFace don't claim streaming
- ✅ Capability queryable before use
- ✅ All streaming implementations are real

**Files Modified:**
- `crates/botticelli_models/src/anthropic/client.rs` - add Streaming impl
- `crates/botticelli_models/src/groq/driver.rs` - remove Streaming impl
- `crates/botticelli_models/src/huggingface/driver.rs` - remove Streaming impl
- `crates/botticelli_interface/src/traits.rs` - add supports_streaming()

**Estimated Effort:** 3-4 sessions

---

### Phase 4: Add Capability Discovery (MAJOR)

**Goal:** Runtime capability introspection

**Approach:** Extend `Metadata` trait

**Tasks:**

1. **Enhance Metadata Trait**
   ```rust
   pub trait Metadata: BotticelliDriver {
       fn metadata(&self) -> ModelMetadata;

       // New capability queries
       fn supports_streaming(&self) -> bool {
           self.metadata().supports_streaming
       }

       fn supports_vision(&self) -> bool {
           self.metadata().supports_vision
       }

       fn supports_tools(&self) -> bool {
           self.metadata().supports_tool_use
       }

       fn supports_json_mode(&self) -> bool {
           self.metadata().supports_json_mode
       }
   }
   ```

2. **Implement Metadata for All Providers**
   - Anthropic: supports_tools=true, supports_streaming=true (after Phase 3)
   - Gemini: supports_vision=true, supports_streaming=true
   - Ollama: supports_streaming=true, capabilities vary by model
   - Groq: supports_streaming=false
   - HuggingFace: supports_streaming=false

3. **Make Metadata Required**
   - Move methods from trait to `BotticelliDriver` directly
   - All drivers must implement
   - Remove separate `Metadata` trait

**Success Criteria:**
- ✅ All providers implement capability queries
- ✅ Capabilities accurate
- ✅ Can select provider based on needs
- ✅ Documentation updated

**Files Modified:**
- `crates/botticelli_interface/src/traits.rs` - enhance Metadata
- All provider client files - implement metadata()

**Estimated Effort:** 2-3 sessions

---

### Phase 5: Complete Trait Coverage (OPTIONAL)

**Goal:** Implement remaining capability traits where applicable

**Priority Order:**

1. **Vision** (Anthropic supports this)
   - Implement for AnthropicClient
   - Document image input handling
   - Add examples

2. **JsonMode** (most providers support structured output)
   - Implement for providers that support it
   - Anthropic: response_format parameter
   - Gemini: JSON schema generation config

3. **Embeddings** (separate from generation)
   - Consider as separate driver type
   - Gemini has embedding models
   - OpenAI has dedicated embedding endpoints

4. **BatchGeneration** (useful for high-volume)
   - Anthropic has batch API
   - Implement where available

5. **Audio/Video/DocumentProcessing**
   - Gemini supports these
   - Implement as needed

**Success Criteria:**
- ✅ Traits implemented where providers support them
- ✅ Documentation updated
- ✅ Examples provided

**Estimated Effort:** 5-10 sessions depending on scope

---

### Phase 6: Provider Selection Framework (OPTIONAL)

**Goal:** Automatic provider selection based on requirements

**Design:**

```rust
pub struct ProviderRequirements {
    pub needs_streaming: bool,
    pub needs_vision: bool,
    pub needs_tools: bool,
    pub needs_json_mode: bool,
    pub max_cost_per_million_tokens: Option<f64>,
}

pub fn select_provider(
    providers: &[Arc<dyn BotticelliDriver>],
    requirements: &ProviderRequirements,
) -> Option<Arc<dyn BotticelliDriver>> {
    providers.iter()
        .filter(|p| {
            (!requirements.needs_streaming || p.supports_streaming()) &&
            (!requirements.needs_vision || p.supports_vision()) &&
            (!requirements.needs_tools || p.supports_tools()) &&
            (!requirements.needs_json_mode || p.supports_json_mode())
        })
        .min_by_key(|p| p.rate_limits().cost_per_million_tokens())
        .cloned()
}
```

**Success Criteria:**
- ✅ Can select optimal provider for task
- ✅ Fallback logic works
- ✅ Cost optimization possible

**Estimated Effort:** 2 sessions

---

## Implementation Priority

### Must Have (Blockers)

1. **Phase 1: Resolve Type Conflicts** - CRITICAL
   - Prevents correct tool calling
   - Causes serialization failures
   - Creates confusion

2. **Phase 2: Align ToolUse Trait** - CRITICAL
   - Architectural inconsistency
   - Code maintainability issue

### Should Have (Major Issues)

3. **Phase 3: Fix Streaming** - MAJOR
   - Missing Anthropic streaming hurts UX
   - Fake streaming misleading
   - Capability detection needed

4. **Phase 4: Capability Discovery** - MAJOR
   - Enables adaptive behavior
   - Improves developer experience
   - Required for Phase 6

### Nice to Have (Enhancements)

5. **Phase 5: Complete Trait Coverage** - OPTIONAL
   - Gradual improvement
   - As needed by features

6. **Phase 6: Provider Selection** - OPTIONAL
   - Advanced feature
   - Builds on Phase 4

---

## Success Metrics

### Architectural Health

- ✅ Single ToolDefinition type
- ✅ No duplicate/conflicting definitions
- ✅ All trait implementations are honest (no fake claims)
- ✅ Capability discovery works for all providers
- ✅ Zero architecture warnings in compilation

### Developer Experience

- ✅ Can write code that works with any provider
- ✅ Clear documentation of capabilities
- ✅ Type system guides correct usage
- ✅ Errors are clear and actionable
- ✅ Examples demonstrate portable patterns

### Feature Parity

- ✅ Tool calling works consistently across providers
- ✅ Streaming available where API supports it
- ✅ Vision support where available
- ✅ All capability traits have at least one implementation

---

## Migration Path

### For Application Code

**Breaking Changes:**
- Import `ToolDefinition` from `botticelli_core` only
- Remove `ToolUse` trait bounds (use `BotticelliDriver` instead)
- Check `supports_streaming()` before calling `generate_stream()`

**Migration Guide:**

```rust
// Before:
use botticelli_interface::ToolDefinition;  // ❌ Will be removed

// After:
use botticelli_core::ToolDefinition;  // ✅ Canonical location

// Before:
async fn call_with_tools<T: ToolUse>(driver: &T) {
    driver.generate_with_tools(req, tools).await?;  // ❌ Method doesn't exist
}

// After:
async fn call_with_tools(driver: &dyn BotticelliDriver) {
    let request = GenerateRequest::builder()
        .messages(messages)
        .tools(Some(tools))  // ✅ Tools in request
        .build()?;
    driver.generate(&request).await?;
}

// Before:
let stream = driver.generate_stream(req).await?;  // ❌ Might fail or be fake

// After:
if driver.supports_streaming() {  // ✅ Check capability
    let stream = driver.generate_stream(req).await?;
} else {
    let response = driver.generate(req).await?;
}
```

---

## Testing Strategy

### Unit Tests

- Each provider must have:
  - `generate()` test with tools
  - `generate_stream()` test (if supported)
  - `metadata()` test showing accurate capabilities
  - Fake/mock tests for error handling

### Integration Tests

- Multi-provider test suite:
  - Same request to all providers
  - Verify tool calling works
  - Verify streaming behavior
  - Verify capability queries

### Compatibility Tests

- Ensure:
  - ToolDefinition serializes consistently
  - Request/response types work across providers
  - Error handling is consistent

---

## Documentation Requirements

### API Documentation

- Update all docstrings to reference unified types
- Document capability trait usage patterns
- Provide migration guide for breaking changes

### User Guide

- "Choosing a Provider" section
- Capability comparison matrix
- Tool calling tutorial
- Streaming best practices

### Examples

- Portable chat client (works with any provider)
- Tool calling across providers
- Provider selection based on capabilities
- Streaming vs non-streaming usage

---

## Timeline Estimate

### Phase 1-2 (Critical Fixes)
- **Duration:** 1-2 weeks
- **Effort:** 2-3 sessions
- **Outcome:** Tool calling works correctly, no type conflicts

### Phase 3 (Streaming)
- **Duration:** 2-3 weeks
- **Effort:** 3-4 sessions
- **Outcome:** Honest streaming implementations, Anthropic streaming works

### Phase 4 (Capability Discovery)
- **Duration:** 1-2 weeks
- **Effort:** 2-3 sessions
- **Outcome:** Can query capabilities at runtime

### Phase 5-6 (Enhancements)
- **Duration:** 3-6 weeks
- **Effort:** 7-12 sessions
- **Outcome:** Full trait coverage, provider selection

**Total Timeline:** 2-3 months for complete implementation

---

## Risk Analysis

### High Risk

**Type Conflicts (Phase 1)**
- **Risk:** Breaking changes affect downstream users
- **Mitigation:** Deprecation period, clear migration guide, semantic versioning

**Streaming Removal (Phase 3)**
- **Risk:** Code expecting streaming from Groq/HuggingFace breaks
- **Mitigation:** Deprecation warnings, version bump, documentation

### Medium Risk

**Anthropic Streaming (Phase 3)**
- **Risk:** Implementation complexity, API changes
- **Mitigation:** Thorough testing, SSE parser libraries, graceful degradation

**Capability Discovery (Phase 4)**
- **Risk:** Changes to core trait
- **Mitigation:** Default implementations, gradual rollout

### Low Risk

**Trait Coverage (Phase 5)**
- **Risk:** Scope creep
- **Mitigation:** Prioritize, implement incrementally, skip if not needed

---

## Conclusion

The Botticelli trait architecture has a solid foundation but needs refinement to achieve true provider interoperability. The critical issues are:

1. **Type conflicts** - Must resolve duplicate ToolDefinition
2. **Trait alignment** - ToolUse trait unused, should be removed or aligned
3. **Streaming honesty** - Fake implementations misleading, Anthropic missing
4. **Capability discovery** - Need runtime introspection

Fixing Phases 1-2 is **critical** and should be prioritized immediately. Phases 3-4 are **major improvements** that significantly enhance usability. Phases 5-6 are **nice to have** enhancements.

With these improvements, the vision of a truly interoperable, trait-based LLM interface will be fully realized.

---

## Next Steps

1. Review and approve this plan
2. Create GitHub issues for each phase
3. Prioritize Phase 1 (critical)
4. Begin implementation
5. Update PLANNING_INDEX.md with this document

**Recommended Start:** Phase 1 (Type Conflict Resolution)
**Estimated First Milestone:** Phases 1-2 complete in 2 weeks
