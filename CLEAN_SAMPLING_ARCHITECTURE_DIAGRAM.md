# Clean Sampling Architecture - Diagram

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER APPLICATION                         │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│                    SAMPLING COORDINATOR                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  • generate_narrative(description) → PartialNarrative   │  │
│  │  • refine_narrative(narrative, feedback)                 │  │
│  └──────────────────────────────────────────────────────────┘  │
└──────────────┬─────────────────────────────────┬────────────────┘
               │                                 │
               ↓                                 ↓
    ┌──────────────────┐              ┌──────────────────┐
    │   LLM SAMPLER    │              │  TOOL REGISTRY   │
    │   (ChatLlmSampler)│              │                  │
    └──────────────────┘              └──────────────────┘
               │                                 │
               ↓                                 │
    ┌──────────────────┐                        │
    │  LLM PROVIDER    │                        │
    │  (Anthropic,     │                        │
    │   OpenAI, etc.)  │                        │
    └──────────────────┘                        │
               │                                 │
               └─────────────┬───────────────────┘
                             ↓
                ┌───────────────────────────┐
                │  CONVERSATION SESSION     │
                │  • system_prompt          │
                │  • turns: Vec<Turn>       │
                │  • state: SessionState    │
                └───────────────────────────┘
```

## Layer Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                      APPLICATION LAYER                         │
│  • SamplingCoordinator                                        │
│  • High-level narrative generation APIs                       │
└───────────────────────────────┬───────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────┐
│                      ORCHESTRATION LAYER                       │
│  • LlmSampler trait (sampling loop logic)                    │
│  • ChatLlmSampler implementation                             │
│  • Tool execution coordination                                │
└───────────────────────────────┬───────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────┐
│                      ABSTRACTION LAYER                         │
│  • LlmProvider trait                                          │
│  • GenerateRequest / GenerateResponse                         │
│  • Provider-agnostic interfaces                               │
└───────────────────────────────┬───────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────┐
│                      PROVIDER LAYER                            │
│  • AnthropicClient                                            │
│  • OpenAICompatibleClient                                     │
│  • Provider-specific implementations                          │
└───────────────────────────────┬───────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────┐
│                      INFRASTRUCTURE LAYER                      │
│  • HTTP clients                                               │
│  • Authentication                                             │
│  • Network communication                                      │
└───────────────────────────────────────────────────────────────┘
```

## Sampling Flow (Detailed)

```
┌─────────────┐
│ User Input  │
└──────┬──────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ ConversationSession                                          │
│                                                              │
│  turns: [                                                    │
│    UserMessage { content: "Create a story", ... }           │
│  ]                                                           │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ LlmSampler::sample()                                         │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Loop (max 20 iterations):                             │ │
│  │                                                         │ │
│  │  1. generate(session, tools) → GenerateResponse        │ │
│  │                                                         │ │
│  │  2. Check stop_reason:                                 │ │
│  │     - EndTurn → Done                                   │ │
│  │     - ToolUse → Execute tools, continue loop           │ │
│  │     - MaxTokens → Done (truncated)                     │ │
│  │                                                         │ │
│  │  3. Add assistant turn to session                      │ │
│  │                                                         │ │
│  └────────────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ LlmSampler::generate()                                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  1. Build GenerateRequest from session                 │ │
│  │     • Convert turns to Messages                        │ │
│  │     • Include tool definitions if provided             │ │
│  │                                                         │ │
│  │  2. Call provider.generate(request)                    │ │
│  │                                                         │ │
│  │  3. Return GenerateResponse                            │ │
│  └────────────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ LlmProvider::generate()                                      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Anthropic:                                            │ │
│  │  1. Convert to Anthropic format                        │ │
│  │  2. HTTP POST to API                                   │ │
│  │  3. Parse response                                     │ │
│  │  4. Map stop_reason                                    │ │
│  │  5. Return GenerateResponse                            │ │
│  │                                                         │ │
│  │  OpenAI-compatible:                                    │ │
│  │  1. Convert to OpenAI format                           │ │
│  │  2. HTTP POST to API                                   │ │
│  │  3. Parse response                                     │ │
│  │  4. Map finish_reason → stop_reason                    │ │
│  │  5. Return GenerateResponse                            │ │
│  └────────────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ GenerateResponse                                             │
│  • outputs: Vec<Output>                                      │
│  • stop_reason: StopReason (MCP-compliant)                  │
│  • usage: Option<TokenUsageData>                            │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
    If ToolUse:
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ LlmSampler::execute_tools()                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  For each ToolCall:                                    │ │
│  │    1. tool_registry.execute(name, args)                │ │
│  │    2. Create ToolResult (success or error)             │ │
│  │                                                         │ │
│  │  Return Vec<ToolResult>                                │ │
│  └────────────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ Add turns to session:                                        │
│  • AssistantToolCalls { tool_calls }                        │
│  • ToolResults { results }                                  │
└──────┬───────────────────────────────────────────────────────┘
       │
       │ (Loop back to generate with updated session)
       │
       ↓
    If EndTurn:
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ Add turn to session:                                         │
│  • AssistantMessage { content: "Final response" }           │
└──────┬───────────────────────────────────────────────────────┘
       │
       ↓
┌──────────────────────────────────────────────────────────────┐
│ Return SamplingResult::Completed { final_response }         │
└──────────────────────────────────────────────────────────────┘
```

## Turn Flow

```
Session Turns Evolution:

Initial:
┌────────────────────────────────────┐
│ turns: [                           │
│   UserMessage("Create a story")   │
│ ]                                  │
└────────────────────────────────────┘

After first generation (tool use):
┌────────────────────────────────────┐
│ turns: [                           │
│   UserMessage("Create a story"),   │
│   AssistantToolCalls([             │
│     ToolCall {                     │
│       id: "call_123",              │
│       name: "create_narrative",    │
│       args: {...}                  │
│     }                              │
│   ])                               │
│ ]                                  │
└────────────────────────────────────┘

After tool execution:
┌────────────────────────────────────┐
│ turns: [                           │
│   UserMessage("Create a story"),   │
│   AssistantToolCalls([...]),       │
│   ToolResults([                    │
│     ToolResult {                   │
│       tool_call_id: "call_123",    │
│       content: "narrative created", │
│       is_error: false              │
│     }                              │
│   ])                               │
│ ]                                  │
└────────────────────────────────────┘

After final generation:
┌────────────────────────────────────┐
│ turns: [                           │
│   UserMessage("Create a story"),   │
│   AssistantToolCalls([...]),       │
│   ToolResults([...]),              │
│   AssistantMessage(                │
│     "I've created the narrative"   │
│   )                                │
│ ]                                  │
└────────────────────────────────────┘
```

## Component Dependencies

```
┌───────────────────────────────────────────────────────────────┐
│                     botticelli_core                            │
│  • LlmProvider trait                                          │
│  • GenerateRequest / GenerateResponse                         │
│  • Input / Output enums                                       │
│  • Message, Role types                                        │
│  • StopReason enum (MCP-compliant)                           │
└───────────────────────────────────────────────────────────────┘
                                ▲
                                │
                   ┌────────────┴────────────┐
                   │                         │
┌──────────────────▼─────┐    ┌─────────────▼──────────────────┐
│  botticelli_mcp        │    │  botticelli_models             │
│  • ConversationSession │    │  • AnthropicClient             │
│  • ConversationTurn    │    │  • OpenAICompatibleClient      │
│  • LlmSampler trait    │    │  • Provider implementations    │
│  • SamplingCoordinator │    │                                │
│  • ToolRegistry        │    │  (implements LlmProvider)      │
│  • ToolDefinition      │    │                                │
└────────────────────────┘    └────────────────────────────────┘
                   │                         │
                   └────────────┬────────────┘
                                ↓
                   ┌────────────────────────┐
                   │  botticelli_chat       │
                   │  • ChatLlmSampler      │
                   │  • ServiceContainer    │
                   │  • Config management   │
                   │                        │
                   │  (implements LlmSampler)│
                   └────────────────────────┘
```

## Error Flow

```
                    ┌──────────────────┐
                    │  API Error       │
                    └────────┬─────────┘
                             │
                             ↓
                    ┌──────────────────┐
                    │  ProviderError   │
                    │  (from provider) │
                    └────────┬─────────┘
                             │
                             ↓
                    ┌──────────────────┐
                    │  SamplingError   │
                    │  (from sampler)  │
                    └────────┬─────────┘
                             │
                             ↓
                    ┌──────────────────┐
                    │  BotticelliError │
                    │  (top-level)     │
                    └────────┬─────────┘
                             │
                             ↓
                    ┌──────────────────┐
                    │  Application     │
                    │  Error Handler   │
                    └──────────────────┘

All errors include:
• Error kind (structured enum)
• Location (file:line via #[track_caller])
• Context (IDs, names, etc.)
• Display formatting via derive_more
```

## Testing Layers

```
┌───────────────────────────────────────────────────────────────┐
│                      UNIT TESTS                                │
│  • Mock providers (no API calls)                              │
│  • Isolated component testing                                 │
│  • Fast, deterministic                                        │
│                                                                │
│  Examples:                                                     │
│  • sampling_test.rs - ChatLlmSampler with mocks              │
└───────────────────────────────────────────────────────────────┘
                                │
                                ↓
┌───────────────────────────────────────────────────────────────┐
│                    INTEGRATION TESTS                           │
│  • Mock providers (controlled sequences)                      │
│  • Full flow testing                                          │
│  • Multi-turn conversations                                   │
│                                                                │
│  Examples:                                                     │
│  • sampling_end_to_end_test.rs - complete flows              │
└───────────────────────────────────────────────────────────────┘
                                │
                                ↓
┌───────────────────────────────────────────────────────────────┐
│                      API TESTS (Gated)                         │
│  • Real provider calls                                         │
│  • Feature-gated: #[cfg(feature = "api")]                    │
│  • Run sparingly (rate limits, costs)                        │
│                                                                │
│  Run with: just test-api                                      │
└───────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Provider Abstraction
```
Why: Swap LLM providers without changing application code
How: LlmProvider trait with standard Request/Response types
Benefit: Test with mocks, deploy with any provider
```

### 2. Turn-Based Modeling
```
Why: Clean representation of multi-turn conversations
How: ConversationTurn enum with typed variants
Benefit: Type-safe, observable, easy to persist
```

### 3. Composable Sampling
```
Why: Support both single generation and full loops
How: Low-level generate() + high-level sample() with default impl
Benefit: Flexible for different use cases
```

### 4. Explicit State Management
```
Why: Observable intermediate states for debugging
How: ConversationSession with public turns vector
Benefit: Easy to inspect, test, and debug
```

### 5. MCP Compliance
```
Why: Follow Model Context Protocol specification
How: StopReason enum with standard values
Benefit: Interoperability, future-proof
```

## Performance Characteristics

```
Operation                    | Time Complexity | Space Complexity
─────────────────────────────|─────────────────|─────────────────
Add turn to session          | O(1)            | O(1)
Generate single response     | O(n) (API call) | O(m) (message size)
Execute tool                 | O(1)            | O(k) (tool result)
Full sampling loop           | O(i*n)          | O(i*m)
  where i = iterations (≤20)
  where n = API latency
  where m = message size
  where k = tool result size

Memory usage per session:
• Base: ~200 bytes
• Per turn: ~100 bytes + content size
• Tool calls: ~50 bytes each + args size
• Tool results: ~100 bytes each + content size

Typical session: ~2-10 KB
```

## Configuration Flow

```
Environment Variables
        │
        ↓
    .toml Config Files
        │
        ↓
    EnvironmentConfig
        │
        ↓
    ServiceContainer
        │
        ├─→ LLM Provider (lazy init)
        ├─→ Database Connection
        ├─→ Tool Registry
        └─→ MCP Server Config
```

## Observability

All components instrumented with tracing:

```
TRACE: Loop iteration details
DEBUG: Function entry/exit, state changes
INFO:  Major operations (generation, tool execution)
WARN:  Recoverable issues (max turns approaching)
ERROR: Failures requiring investigation

Example trace:
┌─ botticelli_chat::sampling::sample
│  ├─ session.turn_count=1
│  ├─ botticelli_chat::sampling::generate
│  │  ├─ request.messages=1
│  │  ├─ botticelli_models::anthropic::generate
│  │  │  └─ response.stop_reason=tool_use
│  │  └─ duration=1.2s
│  ├─ botticelli_mcp::tool_registry::execute
│  │  ├─ tool=create_narrative
│  │  └─ duration=0.1s
│  └─ result=completed
└─ duration=2.5s
```
