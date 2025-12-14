# Narrative Sampling Architecture Analysis

## Executive Summary

After reviewing the codebase, I found that **most infrastructure already exists** but has **architectural issues that will cause problems**. This document presents:

1. What currently exists and its problems
2. Two implementation paths: Pragmatic (ship fast) vs Ideal (do it right)
3. Concrete recommendation

## Current Architecture Review

### What Already Exists ✅

#### 1. Provider Clients (botticelli_models)
- `AnthropicClient`, `OpenAICompatClient`, etc.
- Each has generate methods
- Return responses with tool calls

#### 2. Core Types (botticelli_core)
- `GenerateRequest` - provider-agnostic request
- `GenerateResponse` - contains `Vec<Output>`
- `Output::ToolCalls(Vec<ToolCall>)` - tool calls as one variant
- `ToolCall` - has `id`, `name`, `arguments` ✅

#### 3. Sampling Infrastructure (botticelli_mcp)
- `LlmSampler` trait
- `SamplingSession`, `Turn`, `ToolResponse`
- `SamplingCoordinator`
- `ChatLlmSampler` (STUB - unimplemented)

#### 4. Tool Infrastructure (botticelli_mcp) ✅
- `McpTool` trait - perfect interface
- `ToolRegistry` - can register/get/list tools
- Elicitation tools all implement `McpTool`
- `NarrativeRegistry` for shared state

#### 5. Integration (botticelli_chat)
- `SamplingIntegration` - wires pieces together
- `CommandExecutor` - has sampling reference
- Calls coordinator methods (but they're stubs)

### What's Missing

1. **ChatLlmSampler implementation** - currently returns `unimplemented!()`
2. **ToolRegistry.execute()** method - registry can't execute tools directly
3. **Helper methods on GenerateResponse** - extracting tool calls is awkward
4. **Provider abstraction** - each client has different interface
5. **Command implementations** - narrative commands don't exist

---

## Critical Architectural Issues

### Issue 1: LlmSampler Trait is Too High-Level ⚠️

**Current Design:**
```rust
pub trait LlmSampler: Send + Sync {
    async fn sample(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> BotticelliResult<SamplingSession>;
}
```

**Problems:**
- All-or-nothing: Give strings, get complete session back
- No control over the loop - can't observe intermediate states
- Can't inject custom logic between turns
- Can't implement streaming
- Hard to test individual parts
- Hard to debug (black box)

**Example of the problem:**
```rust
// You want to:
// 1. Send initial message
// 2. LLM calls tool
// 3. Log what tool was called
// 4. Execute tool
// 5. Continue...

// But current trait only allows:
let session = sampler.sample("system", "user").await?;
// Can't observe steps 2-4!
```

**Better Design:**
```rust
pub trait LlmSampler: Send + Sync {
    // Low-level: single LLM call with tools
    async fn generate(
        &self,
        request: GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse>;

    // High-level: full session (can have default impl)
    async fn sample(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> BotticelliResult<SamplingSession> {
        // Default implementation using generate() in loop
        // Subclasses can override for custom behavior
    }
}
```

**Impact:** Medium - Current design works but is inflexible

**Recommendation:** Refactor to add low-level `generate()` method

---

### Issue 2: Turn Structure Couples Too Much ⚠️

**Current Design:**
```rust
pub struct Turn {
    pub request: GenerateRequest,     // HTTP request
    pub response: GenerateResponse,   // HTTP response
    pub tool_calls: Vec<ToolCall>,    // Extracted from response
    pub tool_responses: Vec<ToolResponse>, // Tool execution results
}
```

**Problems:**
- Single struct represents multiple concepts
- "Tool result turn" doesn't have a request/response (only tool results going back to LLM)
- Violates single responsibility principle
- Hard to represent conversation accurately

**Example of the problem:**
```rust
// Conversation flow:
// 1. User: "Create a narrative"
// 2. Assistant: [calls create_narrative_session tool]
// 3. Tool result: {"session_id": "123", "status": "created"}
// 4. Assistant: "I created session 123, what should it be about?"

// Turn 1: request=user_message, response=tool_calls ✅
// Turn 2: request=???, response=??? ❌ (just sending tool results back)
// Turn 3: request=tool_results, response=text ✅

// Current Turn struct doesn't fit turn 2 well
```

**Better Design:**
```rust
pub enum ConversationTurn {
    UserMessage { content: String },
    AssistantMessage { content: String },
    AssistantToolCalls { calls: Vec<ToolCall> },
    ToolResults { results: Vec<ToolResult> },
}

pub struct SamplingSession {
    pub id: String,
    pub turns: Vec<ConversationTurn>,
    pub state: SessionState,
}
```

**Impact:** Low - Current design works for basic cases, awkward for complex flows

**Recommendation:** Keep for MVP, refactor later for clarity

---

### Issue 3: Tool Calls Mixed with Content ⚠️

**Current Design:**
```rust
pub enum Output {
    Text(String),
    Image { mime: String, data: Vec<u8> },
    Audio { mime: String, data: Vec<u8> },
    ToolCalls(Vec<ToolCall>),  // Mixed with content!
    Embedding(Vec<f32>),
    Json(serde_json::Value),
}

pub struct GenerateResponse {
    outputs: Vec<Output>,  // Could have Text AND ToolCalls
}
```

**Problems:**
- Tool calling is semantically different from content generation
- When LLM calls a tool, it's requesting an ACTION, not generating CONTENT
- Forces awkward iteration:
  ```rust
  for output in response.outputs() {
      match output {
          Output::ToolCalls(calls) => handle_tools(calls),
          Output::Text(text) => handle_content(text),
          _ => {} // Ignore images/audio/etc in chat context
      }
  }
  ```

**Why it exists:**
- Claude can return: `[Text("Let me check..."), ToolCalls([call1])]`
- Need to preserve ordering of content blocks
- Different providers structure this differently

**Better Design (Ideal):**
```rust
pub struct GenerateResponse {
    content: Vec<ContentBlock>,  // Text, Image, Audio, etc
    tool_calls: Vec<ToolCall>,   // Separate! Not mixed with content
    stop_reason: StopReason,     // Why generation stopped
}

pub enum ContentBlock {
    Text(String),
    Image { mime: String, data: Vec<u8> },
    // NO ToolCalls variant
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
}
```

**Pragmatic Workaround (Works with current design):**
```rust
// Add helper methods to GenerateResponse
impl GenerateResponse {
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.outputs.iter()
            .filter_map(|o| match o {
                Output::ToolCalls(calls) => Some(calls.clone()),
                _ => None
            })
            .flatten()
            .collect()
    }

    pub fn text_content(&self) -> String {
        self.outputs.iter()
            .filter_map(|o| match o {
                Output::Text(s) => Some(s.as_str()),
                _ => None
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn has_tool_calls(&self) -> bool {
        self.outputs.iter()
            .any(|o| matches!(o, Output::ToolCalls(_)))
    }
}
```

**Impact:** Medium - Current design works but is awkward

**Recommendation:** Add helper methods for MVP, consider refactor for v2

---

### Issue 4: No Provider Abstraction 🔴

**Current Design:**
```rust
// Each client has different signature:
impl AnthropicClient {
    pub async fn generate_anthropic(&self, req: &AnthropicRequest)
        -> Result<AnthropicResponse, ModelsError>;
}

impl OpenAICompatClient {
    pub async fn generate(&self, req: &GenerateRequest)
        -> Result<GenerateResponse, OpenAICompatError>;
}
```

**Problems:**
- `ChatLlmSampler` can't use them polymorphically
- Must have provider-specific code paths
- Can't swap providers at runtime
- Tight coupling

**Example of the problem:**
```rust
impl ChatLlmSampler {
    async fn sample(&self, ...) -> Result<SamplingSession> {
        // How to call LLM?
        // Option 1: Store specific client
        let response = self.anthropic_client.generate_anthropic(...)?;
        // Tightly coupled to Anthropic!

        // Option 2: Enum of all clients
        let response = match &self.client {
            LlmClient::Anthropic(c) => c.generate_anthropic(...)?,
            LlmClient::OpenAI(c) => c.generate(...)?,
            // Must add case for every provider
        };

        // Option 3: Trait object
        let response = self.client.generate(...)?;
        // Doesn't exist!
    }
}
```

**Better Design:**
```rust
// Option A: Trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(
        &self,
        request: GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError>;
}

impl LlmProvider for AnthropicClient { ... }
impl LlmProvider for OpenAICompatClient { ... }

// Option B: Adapter pattern
pub struct ProviderAdapter {
    inner: ProviderImpl,
}

enum ProviderImpl {
    Anthropic(AnthropicClient),
    OpenAI(OpenAICompatClient),
}

impl ProviderAdapter {
    async fn generate(&self, req: GenerateRequest)
        -> Result<GenerateResponse, ProviderError> {
        match &self.inner {
            ProviderImpl::Anthropic(c) => {
                // Convert GenerateRequest -> AnthropicRequest
                // Call c.generate_anthropic()
                // Convert AnthropicResponse -> GenerateResponse
            },
            ProviderImpl::OpenAI(c) => c.generate(req),
        }
    }
}
```

**Impact:** HIGH - Blocks ChatLlmSampler implementation

**Recommendation:** MUST ADD for MVP (use adapter pattern for simplicity)

---

### Issue 5: ToolRegistry Missing Execute Method ⚠️

**Current Design:**
```rust
impl ToolRegistry {
    pub fn register(&mut self, tool: Arc<dyn McpTool>) { }
    pub fn get(&self, name: &str) -> Option<Arc<dyn McpTool>> { }
    pub fn list(&self) -> Vec<Arc<dyn McpTool>> { }
    // execute() method is MISSING
}

// Current usage:
let tool = registry.get("tool_name")
    .ok_or("tool not found")?;
let result = tool.execute(input).await?;
```

**Better Design:**
```rust
impl ToolRegistry {
    // Add this method
    pub async fn execute(&self, name: &str, input: Value)
        -> McpResult<Value> {
        let tool = self.get(name)
            .ok_or_else(|| McpError::tool_not_found(name))?;
        tool.execute(input).await
    }

    // Also add tool definitions for LLM
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
            })
            .collect()
    }
}
```

**Impact:** Low - Easy to add

**Recommendation:** Add for MVP

---

## Two Implementation Paths

### Path 1: Pragmatic (Ship Fast) 🚀

**Goal:** Get MVP working with minimal changes

**Changes Required:**
1. Add helper methods to `GenerateResponse` (tool extraction)
2. Add `execute()` and `tool_definitions()` to `ToolRegistry`
3. Create `ProviderAdapter` for unified client interface
4. Implement `ChatLlmSampler` using adapter
5. Add narrative commands to `CommandExecutor`

**Pros:**
- ✅ Ships quickly (1-2 weeks)
- ✅ No breaking changes
- ✅ Works with existing architecture
- ✅ Proves the concept

**Cons:**
- ❌ Keeps architectural debt
- ❌ Hard to extend later
- ❌ Limited observability
- ❌ Testing is harder

**Code That Gets Written:**
- ~200 lines for GenerateResponse helpers
- ~50 lines for ToolRegistry methods
- ~300 lines for ProviderAdapter
- ~400 lines for ChatLlmSampler impl
- ~200 lines for command integration
- **Total: ~1150 lines**

**Technical Debt Created:**
- LlmSampler trait remains inflexible
- Turn structure remains awkward
- Output enum remains mixed
- Will need refactoring in 3-6 months

---

### Path 2: Ideal (Do It Right) 🏗️

**Goal:** Fix architectural issues, then implement

**Changes Required:**
1. Refactor `LlmSampler` trait to add `generate()` method
2. Create `ConversationTurn` enum to replace current `Turn`
3. Separate tool calls from content in response handling
4. Create proper provider trait/adapter
5. Implement `ChatLlmSampler` with new architecture
6. Update `SamplingCoordinator` to use new types
7. Add narrative commands

**Pros:**
- ✅ Clean architecture
- ✅ Easy to extend (streaming, custom loops)
- ✅ Better observability
- ✅ Easier to test
- ✅ No technical debt

**Cons:**
- ❌ Takes longer (3-4 weeks)
- ❌ More code to change
- ❌ Some breaking changes
- ❌ Risk of scope creep

**Code That Gets Changed/Written:**
- Refactor ~150 lines in sampling types
- ~300 lines for provider abstraction
- ~500 lines for ChatLlmSampler impl
- ~100 lines updating SamplingCoordinator
- ~200 lines for commands
- ~400 lines for tests
- **Total: ~1650 lines, 150 changed**

**Benefits Gained:**
- Extensible architecture for future needs
- Easier to add providers
- Easier to add features (streaming, resumption)
- Better debuggability
- No refactoring needed for 12+ months

---

## Concrete Recommendation

### My Recommendation: **Path 2 (Ideal)** 🏆

**Reasoning:**

1. **We're early** - No users yet, no production code depending on this
2. **Debt compounds** - Path 1 creates debt that costs 2-3x to fix later
3. **The issues are real** - Not over-engineering, these will bite you
4. **Time delta is small** - 2 weeks vs 4 weeks, but saves months later
5. **You value quality** - Your CLAUDE.md shows you care about architecture

**However, I recommend a HYBRID approach:**

### Hybrid Path: Iterate Smartly 🎯

**Phase 1 (Week 1): Fix Critical Blockers**
- Add provider abstraction (adapter or trait)
- Add ToolRegistry.execute() method
- Add GenerateResponse helpers
- These are NECESSARY for any path

**Phase 2 (Week 2): Implement Basic Sampling**
- Implement ChatLlmSampler with current types
- Get end-to-end working
- Add basic commands
- **Ship internal MVP**

**Phase 3 (Week 3): Refactor Based on Learnings**
- Use the working system to inform refactoring
- Refactor LlmSampler trait if needed
- Improve Turn structure if pain points emerge
- Clean up based on real usage

**Phase 4 (Week 4): Polish & Test**
- Comprehensive tests
- Error handling
- Documentation
- **Ship external MVP**

**Why This Works:**
- ✅ Makes progress immediately
- ✅ Validates design with working code
- ✅ Refactors based on real pain, not speculation
- ✅ Still avoids worst technical debt
- ✅ Allows learning and adjustment

---

## Next Steps

**If you choose Pragmatic Path:**
1. I'll write a detailed implementation plan for Path 1
2. Focus on minimal changes to ship fast
3. Document technical debt for later

**If you choose Ideal Path:**
1. I'll write a detailed implementation plan for Path 2
2. Break it into phases
3. Identify refactoring safe points

**If you choose Hybrid Path:**
1. I'll write a 4-phase plan with clear milestones
2. Phase 1-2 get you working code in 2 weeks
3. Phase 3-4 refine architecture based on learnings

**Which path do you want to take?**
