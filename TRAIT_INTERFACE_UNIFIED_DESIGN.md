# Trait Interface Unified Design

## Vision Statement

**The trait interface is the contract for all LLM behaviors.** Clients interact with providers through traits, not by encoding behavior in request fields. Every capability—generation, streaming, tool calling, vision, embeddings—is explicitly declared and discoverable through the trait system.

---

## Design Principles

### 1. Traits Define Behavior, Not Requests

**Bad** (current state):
```rust
// Tools hidden in request field
let request = GenerateRequest::builder()
    .messages(messages)
    .tools(Some(tools))  // ❌ Capability hidden in data
    .build()?;

// No way to know if provider supports tools
driver.generate(&request).await?;
```

**Good** (vision):
```rust
// Tools explicit in trait
trait ToolCalling: BotticelliDriver {
    async fn generate_with_tools(
        &self,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<GenerateResponse>;
}

// Capability discoverable via trait bound
fn use_tools<T: ToolCalling>(driver: &T) {
    driver.generate_with_tools(messages, tools).await?;
}
```

### 2. Composition Over Inheritance

Capabilities are separate traits that compose:

```rust
trait BotticelliDriver {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResponse>;
}

trait Streaming: BotticelliDriver {
    async fn generate_stream(&self, messages: &[Message]) -> Result<Stream<Chunk>>;
}

trait ToolCalling: BotticelliDriver {
    async fn generate_with_tools(
        &self,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<GenerateResponse>;
}

trait Vision: BotticelliDriver {
    async fn generate_with_images(
        &self,
        messages: &[Message],
        images: &[Image],
    ) -> Result<GenerateResponse>;
}
```

### 3. Capabilities Are Discoverable

```rust
// Runtime capability checking
if driver.is::<dyn ToolCalling>() {
    // Use tool calling
}

// Compile-time capability checking
fn requires_tools<T: ToolCalling>(driver: &T) {
    // Type system ensures tools are supported
}
```

### 4. Request Types Are Simple

```rust
// Requests contain only data, not capabilities
struct Message {
    role: Role,
    content: Vec<Content>,
}

// Not this:
struct GenerateRequest {
    messages: Vec<Message>,
    tools: Option<Vec<Tool>>,        // ❌ Capability as field
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}
```

---

## Proposed Trait Hierarchy

### Core Driver (Required)

```rust
/// Core LLM driver interface - all providers must implement
#[async_trait]
pub trait BotticelliDriver: Send + Sync {
    /// Generate response from messages
    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse>;

    /// Provider identifier (e.g., "anthropic", "gemini")
    fn provider_name(&self) -> &'static str;

    /// Model identifier (e.g., "claude-3-5-sonnet-20241022")
    fn model_name(&self) -> &str;

    /// Rate limit configuration for this provider
    fn rate_limits(&self) -> &RateLimitConfig;

    /// Query provider capabilities at runtime
    fn capabilities(&self) -> Capabilities;
}

/// Simplified request - no capability fields
pub struct GenerateRequest {
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    // No tools, no vision, no audio - those are trait methods
}

/// Capability flags for runtime discovery
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub streaming: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub audio: bool,
    pub video: bool,
    pub embeddings: bool,
    pub json_mode: bool,
}
```

### Streaming (Optional)

```rust
/// Incremental response streaming
#[async_trait]
pub trait Streaming: BotticelliDriver {
    /// Generate response as stream of chunks
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
}
```

### Tool Calling (Optional)

```rust
/// Tool/function calling capability
#[async_trait]
pub trait ToolCalling: BotticelliDriver {
    /// Generate response with tool calling enabled
    ///
    /// The model can request tool executions via Output::ToolCall.
    /// Client is responsible for executing tools and providing results.
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[Tool],
    ) -> Result<GenerateResponse>;

    /// Maximum number of tools supported in single request
    fn max_tools(&self) -> usize {
        128  // Anthropic default
    }

    /// Whether provider supports parallel tool calls
    fn supports_parallel_tools(&self) -> bool {
        false
    }
}

/// Tool definition (MCP-compliant)
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: JsonSchema,
}
```

### Vision (Optional)

```rust
/// Multimodal vision capability
#[async_trait]
pub trait Vision: BotticelliDriver {
    /// Generate response with image inputs
    async fn generate_with_vision(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse>;

    /// Supported image formats
    fn supported_image_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP]
    }

    /// Maximum image size in bytes
    fn max_image_size(&self) -> usize {
        5_000_000  // 5MB default
    }
}

/// Message content can include images
pub enum Content {
    Text(String),
    Image(ImageData),
    Audio(AudioData),
    Video(VideoData),
}
```

### Token Counting (Optional but Common)

```rust
/// Token counting for cost estimation
pub trait TokenCounting: BotticelliDriver {
    /// Count tokens in text
    fn count_tokens(&self, text: &str) -> Result<usize>;

    /// Count tokens in full request
    fn count_request_tokens(&self, request: &GenerateRequest) -> Result<usize> {
        request.messages()
            .iter()
            .flat_map(|msg| msg.content())
            .filter_map(|content| match content {
                Content::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .map(|text| self.count_tokens(text))
            .sum()
    }
}
```

### Embeddings (Optional)

```rust
/// Vector embedding generation
#[async_trait]
pub trait Embeddings: BotticelliDriver {
    /// Generate embedding vector for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Batch embed multiple texts
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Embedding dimension
    fn embedding_dimension(&self) -> usize;
}
```

---

## Migration Strategy

### Phase 1: Add Capabilities Query (Non-Breaking)

Add `capabilities()` method to `BotticelliDriver`:

```rust
impl BotticelliDriver for AnthropicClient {
    // Existing methods...

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            streaming: true,
            tool_calling: true,
            vision: true,
            audio: false,
            video: false,
            embeddings: false,
            json_mode: true,
        }
    }
}
```

**Benefit:** Can query capabilities before Phase 2 changes.

**Estimated Effort:** 1 session

---

### Phase 2: Implement ToolCalling Trait (Breaking)

**Current State:**
```rust
// Tools passed via request field
impl BotticelliDriver for AnthropicClient {
    async fn generate(&self, req: &GenerateRequest) -> Result<GenerateResponse> {
        // Reads req.tools() internally
        let anthropic_req = self.convert_request(req)?;  // Handles tools
        // ...
    }
}
```

**Target State:**
```rust
// Tools passed via trait method
impl ToolCalling for AnthropicClient {
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[Tool],
    ) -> Result<GenerateResponse> {
        // Convert tools to Anthropic format
        let anthropic_tools: Vec<AnthropicTool> = tools.iter()
            .map(AnthropicTool::from_mcp)
            .collect();

        // Build Anthropic request with tools
        let anthropic_req = AnthropicRequest::builder()
            .model(&self.model)
            .messages(self.convert_messages(request.messages())?)
            .tools(Some(anthropic_tools))
            .build()?;

        // Call API and convert response
        let anthropic_resp = self.call_api(&anthropic_req).await?;
        self.convert_response(&anthropic_resp)
    }

    fn max_tools(&self) -> usize {
        128
    }

    fn supports_parallel_tools(&self) -> bool {
        true
    }
}

// Simple generate() delegates to generate_with_tools() with empty tools
impl BotticelliDriver for AnthropicClient {
    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse> {
        self.generate_with_tools(request, &[]).await
    }
}
```

**Migration Path:**

1. Implement `ToolCalling` trait for providers
2. Deprecate `GenerateRequest.tools` field
3. Update callers to use `generate_with_tools()` instead
4. Remove `tools` field in next major version

**Breaking Changes:**
- Code calling `req.tools()` must migrate to `generate_with_tools()`
- Type signature changes for tool-calling code

**Estimated Effort:** 2-3 sessions

---

### Phase 3: Simplify GenerateRequest (Breaking)

Remove all capability fields:

```rust
// Before
pub struct GenerateRequest {
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<Tool>>,  // ❌ Remove
}

// After
pub struct GenerateRequest {
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    // Clean - no capability fields
}
```

**Estimated Effort:** 1 session

---

### Phase 4: Implement Other Capability Traits (Optional)

**Vision:**
```rust
impl Vision for AnthropicClient {
    async fn generate_with_vision(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse> {
        // Images come from Message.content() as Content::Image
        // Convert to Anthropic format and call API
    }
}
```

**Streaming:**
```rust
impl Streaming for AnthropicClient {
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        // Use Anthropic's SSE streaming endpoint
    }
}
```

**Estimated Effort:** 3-5 sessions per capability

---

### Phase 5: Remove Fake Implementations (Breaking)

Remove `Streaming` implementations that aren't real:

```rust
// Before
impl Streaming for GroqDriver {
    async fn generate_stream(...) -> ... {
        // Wraps synchronous response - fake streaming
    }
}

// After
// No Streaming impl - capabilities().streaming == false
```

**Estimated Effort:** 1 session

---

## Usage Patterns

### Pattern 1: Static Dispatch with Trait Bounds

```rust
// Require specific capabilities at compile time
async fn use_tools_with_vision<T>(driver: &T, messages: &[Message], tools: &[Tool])
where
    T: ToolCalling + Vision,
{
    // Type system ensures both capabilities exist
    let response = driver.generate_with_tools(request, tools).await?;
    // ...
}
```

### Pattern 2: Dynamic Dispatch with Capability Queries

```rust
// Adapt based on runtime capabilities
async fn adaptive_generate(
    driver: &dyn BotticelliDriver,
    messages: &[Message],
    tools: Option<&[Tool]>,
) -> Result<GenerateResponse> {
    let caps = driver.capabilities();

    if caps.tool_calling && tools.is_some() {
        // Downcast to ToolCalling trait
        let tool_driver = driver
            .as_any()
            .downcast_ref::<dyn ToolCalling>()
            .ok_or("Provider claims tool support but doesn't implement trait")?;

        tool_driver.generate_with_tools(request, tools.unwrap()).await
    } else {
        driver.generate(request).await
    }
}
```

### Pattern 3: Provider Selection

```rust
// Select provider based on required capabilities
fn select_provider(
    providers: &[Arc<dyn BotticelliDriver>],
    needs_tools: bool,
    needs_vision: bool,
) -> Option<Arc<dyn BotticelliDriver>> {
    providers
        .iter()
        .find(|p| {
            let caps = p.capabilities();
            (!needs_tools || caps.tool_calling)
                && (!needs_vision || caps.vision)
        })
        .cloned()
}
```

### Pattern 4: Graceful Degradation

```rust
// Try advanced features, fall back to basic
async fn smart_generate(
    driver: &dyn BotticelliDriver,
    request: &GenerateRequest,
    tools: &[Tool],
) -> Result<GenerateResponse> {
    if driver.capabilities().tool_calling {
        // Use tools if supported
        if let Some(tool_driver) = driver.as_any().downcast_ref::<dyn ToolCalling>() {
            return tool_driver.generate_with_tools(request, tools).await;
        }
    }

    // Fall back to basic generation
    driver.generate(request).await
}
```

---

## Type System Benefits

### 1. Compile-Time Guarantees

```rust
// This won't compile if T doesn't support tools
fn requires_tools<T: ToolCalling>(driver: &T) {
    driver.generate_with_tools(request, tools).await?;
}

// Prevents calling unsupported features
fn use_groq(driver: &GroqDriver) {
    driver.generate_with_tools(request, tools).await?;  // ❌ Compile error
}
```

### 2. Clear Error Messages

```rust
// Current (confusing):
let response = driver.generate(&request).await?;
// Tool calls ignored silently if provider doesn't support them

// Proposed (clear):
let response = driver.generate_with_tools(request, tools).await?;
// ❌ Compile error: GroqDriver doesn't implement ToolCalling

// Or runtime:
if !driver.capabilities().tool_calling {
    return Err("Provider doesn't support tool calling");
}
```

### 3. Explicit Capabilities

```rust
// What can this provider do?
let caps = driver.capabilities();
println!("Streaming: {}", caps.streaming);
println!("Tools: {}", caps.tool_calling);
println!("Vision: {}", caps.vision);
```

---

## Provider Implementation Matrix

After full migration:

| Provider | BotticelliDriver | ToolCalling | Streaming | Vision | TokenCounting | Embeddings |
|----------|------------------|-------------|-----------|--------|---------------|------------|
| **Anthropic** | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| **Gemini** | ✅ | 🚧 | ✅ | ✅ | ✅ | ✅ |
| **Ollama** | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ |
| **Groq** | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **HuggingFace** | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |

✅ = Full support
🚧 = Partial (needs implementation)
⚠️ = Model-dependent
❌ = Not supported

---

## Open Questions

### 1. How to Handle Model-Specific Capabilities?

**Problem:** Ollama capabilities vary by model.

**Options:**

A. **Model-Specific Drivers** (RECOMMENDED)
```rust
// Create separate driver per model
let llama_vision = OllamaClient::new("llama3.2-vision");
let llama_base = OllamaClient::new("llama3.2");

impl Vision for OllamaClient {
    fn capabilities(&self) -> Capabilities {
        // Query model capabilities
        match self.model_name() {
            "llama3.2-vision" => Capabilities { vision: true, .. },
            _ => Capabilities { vision: false, .. },
        }
    }
}
```

B. **Dynamic Capability Query**
```rust
// Query capabilities at runtime based on loaded model
let driver = OllamaClient::new();
driver.load_model("llama3.2-vision").await?;
assert!(driver.capabilities().vision);
```

**Recommendation:** Option A - simpler, compile-time safe

### 2. How to Handle Tool Results?

**Current Flow:**
1. LLM requests tool execution → `Output::ToolCall`
2. Client executes tools
3. Client sends results back → ???

**Options:**

A. **Separate Method**
```rust
trait ToolCalling {
    async fn generate_with_tools(...) -> Result<GenerateResponse>;

    async fn continue_with_tool_results(
        &self,
        conversation: &[Message],
        tool_results: &[ToolResult],
    ) -> Result<GenerateResponse>;
}
```

B. **Encode in Messages**
```rust
// Tool results are just messages
let tool_result_message = Message {
    role: Role::Tool,
    content: vec![Content::ToolResult(result)],
};

// Continue conversation
driver.generate_with_tools(request, tools).await?;
```

**Recommendation:** Option B - simpler, fits existing message model

### 3. How to Handle Optional Parameters?

**Problem:** Some parameters apply to all providers (temperature), some don't (top_k).

**Options:**

A. **Common in Request, Specific in Builder**
```rust
struct GenerateRequest {
    messages: Vec<Message>,
    temperature: Option<f32>,     // Common
    max_tokens: Option<u32>,      // Common
    // Provider-specific via builder
}

impl GenerateRequestBuilder {
    fn with_gemini_top_k(mut self, top_k: i32) -> Self {
        self.provider_params.insert("top_k", top_k);
        self
    }
}
```

B. **All Provider-Specific**
```rust
struct GenerateRequest {
    messages: Vec<Message>,
    params: HashMap<String, Value>,
}

// Each provider defines own params
const ANTHROPIC_TEMPERATURE: &str = "temperature";
const GEMINI_TOP_K: &str = "top_k";
```

**Recommendation:** Option A - common params typed, rare params flexible

---

## Success Criteria

### Architectural

- ✅ All capabilities defined in traits, not request fields
- ✅ Trait hierarchy is simple and composable
- ✅ Capabilities discoverable at compile-time and runtime
- ✅ No fake implementations (honest trait impls only)
- ✅ Type system prevents unsupported operations

### Developer Experience

- ✅ Clear error messages when capability missing
- ✅ Can write provider-agnostic code with trait bounds
- ✅ Can adapt based on runtime capabilities
- ✅ Obvious which providers support which features
- ✅ Examples demonstrate all patterns

### Code Quality

- ✅ Single source of truth (traits, not request fields)
- ✅ No architectural inconsistencies
- ✅ No duplicate definitions
- ✅ Follows Rust best practices
- ✅ CLAUDE.md compliant

---

## Timeline

### Phase 1: Capabilities Query
- **Duration:** 1 week
- **Effort:** 1 session
- **Breaking:** No

### Phase 2: ToolCalling Trait
- **Duration:** 2-3 weeks
- **Effort:** 2-3 sessions
- **Breaking:** Yes (deprecation path)

### Phase 3: Simplify Request
- **Duration:** 1 week
- **Effort:** 1 session
- **Breaking:** Yes (after Phase 2 migration)

### Phase 4: Other Traits
- **Duration:** 3-6 weeks
- **Effort:** 3-5 sessions
- **Breaking:** No (pure additions)

### Phase 5: Remove Fakes
- **Duration:** 1 week
- **Effort:** 1 session
- **Breaking:** Yes

**Total:** 8-14 weeks for complete migration

---

## Relationship to Other Plans

### Builds On:
- **ARCHITECTURAL_GAPS_RESOLUTION_PLAN_V2.md** - Fixed tool calling mechanics
  - Tools now work end-to-end
  - This plan elevates them to trait interface

### Supersedes:
- **TRAIT_INTERFACE_IMPROVEMENT_PLAN.md** - Same goal, different approach
  - Old plan: Remove ToolUse trait, keep request field
  - New plan: Implement ToolUse trait properly, remove request field

### Complements:
- **PLANNING_INDEX.md** - Strategic documentation
  - This defines the architectural vision
  - Implementation tracked separately

---

## Migration Example

### Before (Current)

```rust
// Client code - unclear what's supported
async fn generate_content(driver: &dyn BotticelliDriver, tools: &[Tool]) {
    let request = GenerateRequest::builder()
        .messages(messages)
        .tools(Some(tools.to_vec()))  // Hope provider supports this
        .build()?;

    // No way to know if tools will be used
    let response = driver.generate(&request).await?;

    // Did tools work? Who knows!
}
```

### After (Vision)

```rust
// Client code - explicit and type-safe
async fn generate_content<T: ToolCalling>(driver: &T, tools: &[Tool]) {
    // Type system guarantees tools are supported
    let response = driver.generate_with_tools(request, tools).await?;

    // Tools definitely worked
}

// Or with runtime check:
async fn adaptive_generate(driver: &dyn BotticelliDriver, tools: &[Tool]) {
    if driver.capabilities().tool_calling {
        let tool_driver = driver.as_any().downcast_ref::<dyn ToolCalling>()?;
        tool_driver.generate_with_tools(request, tools).await?
    } else {
        warn!("Provider doesn't support tools, proceeding without them");
        driver.generate(request).await?
    }
}
```

---

## Conclusion

This unified design achieves your vision:

1. **Traits define behavior** - Tool calling is a trait method, not a request field
2. **Clean and simple** - Composable traits, no hidden capabilities
3. **Fully discoverable** - Capabilities queryable at compile-time and runtime
4. **Type-safe** - Compiler prevents calling unsupported features
5. **Provider-agnostic** - Write once, run on any compatible provider

The migration is incremental with clear deprecation paths. After completion, the Botticelli interface will be a best-in-class example of Rust trait-based design for LLM providers.

**Next Step:** Implement Phase 1 (add `capabilities()` query) to start validating the design.
