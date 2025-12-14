# Clean Sampling Architecture - Usage Guide

## Overview

The refactored sampling architecture provides a clean, testable, and extensible system for LLM-based narrative generation with tool calling support.

## Quick Start

### 1. Basic Text Generation

```rust
use botticelli_chat::ChatLlmSampler;
use botticelli_mcp::{ConversationSession, ConversationTurn, LlmSampler, ToolRegistry};
use std::sync::Arc;

// Create provider (Anthropic, OpenAI-compatible, etc.)
let provider = Arc::new(create_your_provider());
let tool_registry = Arc::new(ToolRegistry::default());
let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

// Create conversation
let mut session = ConversationSession::new("You are a helpful assistant");
session.add_turn(ConversationTurn::UserMessage {
    content: "Hello!".to_string(),
    attachments: None,
});

// Generate response
let tools = vec![];
let result = sampler.sample(&mut session, &tools).await?;

match result {
    SamplingResult::Completed { final_response } => {
        println!("Assistant: {}", final_response);
    }
}
```

### 2. Narrative Generation with Tool Calling

```rust
use botticelli_chat::ChatLlmSampler;
use botticelli_mcp::{SamplingCoordinator, ToolRegistry};
use std::sync::Arc;

// Setup
let provider = Arc::new(create_your_provider());
let mut tool_registry = ToolRegistry::default();

// Register tools
tool_registry.register_tool(create_narrative_tool);
tool_registry.register_tool(validate_tool);

let tool_registry = Arc::new(tool_registry);
let sampler = Arc::new(ChatLlmSampler::new(provider.clone(), tool_registry.clone()));
let coordinator = SamplingCoordinator::new(sampler, tool_registry);

// Generate narrative
let narrative = coordinator
    .generate_narrative("Create a space exploration story".to_string())
    .await?;

println!("Generated: {}", narrative.name().unwrap_or("unnamed"));
```

## Architecture Components

### 1. LlmProvider (Core Abstraction)

**Location:** `botticelli_core::LlmProvider`

The foundation trait for all LLM providers:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: &GenerateRequest) 
        -> Result<GenerateResponse, ProviderError>;
    
    fn provider_name(&self) -> &str;
    fn default_model(&self) -> &str;
    fn supports_tools(&self) -> bool { true }
}
```

**Implementations:**
- `AnthropicClient` - Claude models
- `OpenAICompatibleClient` - OpenAI, Groq, Ollama, etc.

**Benefits:**
- ✅ Swap providers without changing application code
- ✅ Easy to mock for testing
- ✅ Consistent error handling

### 2. ConversationSession (State Management)

**Location:** `botticelli_mcp::ConversationSession`

Models multi-turn conversations:

```rust
pub struct ConversationSession {
    pub system_prompt: String,
    pub turns: Vec<ConversationTurn>,
    pub state: SessionState,
}

pub enum ConversationTurn {
    UserMessage { content: String, attachments: Option<Vec<Input>> },
    AssistantMessage { content: String },
    AssistantToolCalls { tool_calls: Vec<ToolCall> },
    ToolResults { results: Vec<ToolResult> },
}
```

**Usage:**
```rust
let mut session = ConversationSession::new("System prompt");

// Add user message
session.add_turn(ConversationTurn::UserMessage {
    content: "What's the weather?".to_string(),
    attachments: None,
});

// Sampler adds assistant turns automatically
let result = sampler.sample(&mut session, &tools).await?;

// Session now contains full conversation history
println!("Turn count: {}", session.turn_count());
```

**Benefits:**
- ✅ Clean separation of conversation state
- ✅ Observable intermediate states
- ✅ Easy to persist/restore sessions
- ✅ Type-safe turn modeling

### 3. LlmSampler (Sampling Logic)

**Location:** `botticelli_mcp::LlmSampler`

High-level sampling with tool support:

```rust
#[async_trait]
pub trait LlmSampler: Send + Sync {
    // Low-level: single generation
    async fn generate(
        &self,
        session: &ConversationSession,
        tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError>;

    // High-level: full sampling loop with tool execution
    async fn sample(
        &self,
        session: &mut ConversationSession,
        tools: &[ToolDefinition],
    ) -> Result<SamplingResult, SamplingError> {
        // Default implementation handles tool calling loop
    }

    // Execute tool calls
    async fn execute_tools(
        &self,
        tool_calls: &[ToolCall],
    ) -> Result<Vec<ToolResult>, SamplingError>;
}
```

**Implementation:** `ChatLlmSampler`

```rust
pub struct ChatLlmSampler {
    provider: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
}
```

**Benefits:**
- ✅ Composable: low-level `generate()` + high-level `sample()`
- ✅ Automatic tool calling loop
- ✅ Clean error propagation
- ✅ Provider-agnostic

### 4. SamplingCoordinator (Orchestration)

**Location:** `botticelli_mcp::SamplingCoordinator`

Coordinates narrative generation:

```rust
pub struct SamplingCoordinator {
    sampler: Arc<dyn LlmSampler>,
    tool_registry: Arc<ToolRegistry>,
}

impl SamplingCoordinator {
    pub async fn generate_narrative(
        &self,
        description: String,
    ) -> BotticelliResult<PartialNarrative>;

    pub async fn refine_narrative(
        &self,
        narrative: PartialNarrative,
        feedback: String,
    ) -> BotticelliResult<PartialNarrative>;
}
```

**Benefits:**
- ✅ High-level narrative operations
- ✅ Manages tool registry
- ✅ Iterative refinement support

### 5. ToolRegistry (Tool Management)

**Location:** `botticelli_mcp::ToolRegistry`

Manages available tools:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>);
    pub fn execute(&self, name: &str, args: Value) -> Result<Value>;
    pub fn tool_definitions(&self) -> Vec<ToolDefinition>;
}
```

**Usage:**
```rust
let mut registry = ToolRegistry::default();

registry.register_tool(Arc::new(CreateNarrativeTool::new()));
registry.register_tool(Arc::new(ValidateNarrativeTool::new()));

// Tools are automatically available to LLM
let tools = registry.tool_definitions();
```

## Data Flow

```
User Input
    ↓
ConversationSession (add UserMessage turn)
    ↓
LlmSampler::sample()
    ↓
LlmSampler::generate() → LlmProvider::generate()
    ↓
GenerateResponse (with StopReason)
    ↓
if StopReason::ToolUse:
    ↓
    LlmSampler::execute_tools() → ToolRegistry::execute()
    ↓
    Add ToolResults turn to session
    ↓
    Loop back to generate()
    ↓
if StopReason::EndTurn:
    ↓
    Add AssistantMessage turn to session
    ↓
    Return SamplingResult::Completed
```

## Key Types

### GenerateRequest

```rust
pub struct GenerateRequest {
    messages: Vec<Message>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    model: Option<String>,
}
```

### GenerateResponse

```rust
pub struct GenerateResponse {
    outputs: Vec<Output>,
    stop_reason: StopReason,  // MCP-compliant
    usage: Option<TokenUsageData>,
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    ContentFilter,
    StopSequence,
    Other,
}
```

### Input/Output

```rust
pub enum Input {
    Text(String),
    Image { mime: Option<String>, data: Vec<u8> },
    ToolCall { id: String, name: String, arguments: Value },
    ToolResult { tool_call_id: String, content: String, is_error: bool },
    // ... more variants
}

pub enum Output {
    Text(String),
    ToolCalls(Vec<ToolCall>),
    // ... more variants
}
```

## Testing

### Unit Testing with Mocks

```rust
use botticelli_core::{GenerateResponse, LlmProvider, StopReason};

struct MockProvider {
    responses: Vec<GenerateResponse>,
    call_count: Arc<Mutex<usize>>,
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn generate(&self, _req: &GenerateRequest) 
        -> Result<GenerateResponse, ProviderError> 
    {
        let mut count = self.call_count.lock().unwrap();
        let response = self.responses[*count].clone();
        *count += 1;
        Ok(response)
    }
    // ... other methods
}

#[tokio::test]
async fn test_sampling_with_mock() {
    let mock = Arc::new(MockProvider::new(vec![
        GenerateResponseBuilder::default()
            .outputs(vec![Output::Text("Hello!".into())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .unwrap(),
    ]));
    
    let registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(mock, registry);
    
    let mut session = ConversationSession::new("Test");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Hi".to_string(),
        attachments: None,
    });
    
    let result = sampler.sample(&mut session, &[]).await.unwrap();
    
    assert!(matches!(result, SamplingResult::Completed { .. }));
}
```

### Integration Testing

See `crates/botticelli_chat/tests/sampling_end_to_end_test.rs` for complete examples.

## Error Handling

All errors follow the project standard:

```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Sampling error: {} at {}:{}", kind, file, line)]
pub struct SamplingError {
    pub kind: SamplingErrorKind,
    pub line: u32,
    pub file: &'static str,
}

pub enum SamplingErrorKind {
    MaxTurnsExceeded { max: usize },
    ToolExecutionFailed { tool_name: String, reason: String },
    UnknownTool { name: String },
    ProviderError(String),
    // ...
}
```

Errors include:
- ✅ Location tracking (`file:line`)
- ✅ Structured kinds with context
- ✅ `#[track_caller]` for automatic location
- ✅ `derive_more` for Display/Error

## Configuration

### Environment-Based Provider Selection

```toml
# chat.toml
[llm]
provider = "anthropic"  # or "openai", "ollama", etc.
model = "claude-3-5-sonnet-20241022"

[llm.anthropic]
api_key = "${ANTHROPIC_API_KEY}"

[llm.openai]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1/chat/completions"
```

```rust
// Provider is lazily initialized from config
let provider = services.llm_provider()?;
```

## Best Practices

### 1. Always Use Builders

```rust
// ❌ Never
let request = GenerateRequest {
    messages: vec![msg],
    max_tokens: Some(100),
    temperature: None,
    model: None,
};

// ✅ Always
let request = GenerateRequest::builder()
    .messages(vec![msg])
    .max_tokens(Some(100))
    .build();
```

### 2. Handle All StopReasons

```rust
match response.stop_reason() {
    StopReason::EndTurn => { /* normal completion */ },
    StopReason::ToolUse => { /* execute tools */ },
    StopReason::MaxTokens => { /* handle truncation */ },
    StopReason::ContentFilter => { /* handle policy */ },
    _ => { /* other cases */ },
}
```

### 3. Instrument Public Functions

```rust
#[instrument(skip(self, session), fields(turn_count = session.turn_count()))]
pub async fn sample(
    &self,
    session: &mut ConversationSession,
    tools: &[ToolDefinition],
) -> Result<SamplingResult, SamplingError> {
    debug!("Starting sampling loop");
    // ...
}
```

### 4. Use Arc for Shared State

```rust
let provider = Arc::new(provider);
let registry = Arc::new(registry);
let sampler = Arc::new(ChatLlmSampler::new(provider, registry));

// Can clone Arc cheaply for async tasks
let sampler_clone = Arc::clone(&sampler);
tokio::spawn(async move {
    sampler_clone.sample(&mut session, &[]).await
});
```

## Migration from Old Code

### Before (Old SessionState)

```rust
let mut state = SessionState::new();
state.add_message(role, content);
let response = client.generate_with_state(&state)?;
```

### After (New ConversationSession)

```rust
let mut session = ConversationSession::new("System prompt");
session.add_turn(ConversationTurn::UserMessage {
    content: content.to_string(),
    attachments: None,
});
let result = sampler.sample(&mut session, &tools).await?;
```

## Debugging Tips

### 1. Enable Tracing

```bash
RUST_LOG=botticelli_chat=debug,botticelli_mcp=debug cargo run
```

### 2. Inspect Session State

```rust
// After sampling
println!("Turns: {}", session.turn_count());
for (i, turn) in session.turns.iter().enumerate() {
    println!("{}: {:?}", i, turn);
}
```

### 3. Check StopReason

```rust
let response = sampler.generate(&session, &tools).await?;
debug!(?response.stop_reason(), "Generation stopped");
```

## Performance Considerations

### 1. Token Limits

```rust
let request = GenerateRequest::builder()
    .messages(messages)
    .max_tokens(Some(1000))  // Prevent excessive costs
    .build();
```

### 2. Tool Execution Timeout

```rust
// In your tool implementation
tokio::time::timeout(
    Duration::from_secs(30),
    tool.execute(args)
).await??
```

### 3. Session Cleanup

```rust
// Limit conversation history
if session.turn_count() > 50 {
    session.trim_old_turns(25);  // Keep last 25
}
```

## Further Reading

- **Architecture Document:** `NARRATIVE_SAMPLING_CLEAN_ARCHITECTURE.md`
- **Error Audit:** `ERROR_HANDLING_AUDIT.md`
- **Provider Implementations:** `crates/botticelli_models/src/`
- **Test Examples:** `crates/botticelli_chat/tests/`
- **MCP Specification:** https://modelcontextprotocol.io/specification

## Support

For issues or questions:
1. Check test files for usage examples
2. Review architecture document for design rationale
3. Enable debug logging to see internal flow
4. Consult MCP specification for protocol details
