# Narrative Sampling Strategy

## Overview

This document defines the **sampling strategy** for LLM-driven narrative creation. The sampling strategy sits above the elicitation layer, enabling LLMs to orchestrate narrative creation through natural conversation by calling composable MCP tools.

**Architecture Layers**:
```
User Natural Language
        ↓
Sampling Strategy (LLM orchestrates via MCP)
        ↓
Elicitation Tools (Domain logic + MCP interface)
        ↓
PartialNarrative State
        ↓
Final Narrative TOML
```

## Key Insight

**Elicitation** = What information is needed and how to validate it
**Sampling** = How the LLM extracts that information through conversation

The sampling strategy treats elicitation tools as **composable primitives** that the LLM can call in any order, multiple times, based on conversational context.

## Design Principles

1. **Tool Composability**: Each elicitation tool is independent and reusable
2. **LLM Autonomy**: LLM decides tool call sequence based on user conversation
3. **State Consistency**: PartialNarrative tracks progress across tool calls
4. **Iterative Refinement**: User can modify any aspect through continued dialogue
5. **Progressive Disclosure**: Don't overwhelm - elicit details as needed
6. **Validation Checkpoints**: LLM validates before marking sections complete

## MCP Tool Interface

### Tool Design Pattern

Each elicitation tool follows this pattern:

```json
{
  "name": "tool_name",
  "description": "What it does and when to use it",
  "inputSchema": {
    "type": "object",
    "properties": {
      "narrative_id": {"type": "string", "description": "UUID tracking this narrative"},
      // ... tool-specific parameters
    },
    "required": ["narrative_id"]
  }
}
```

### State Management

**Key Design Decision**: Use `narrative_id` (UUID) to track state across tool calls.

```rust
// Server-side state storage
pub struct NarrativeRegistry {
    active_narratives: HashMap<Uuid, PartialNarrative>,
}
```

**Benefits**:
- Multiple concurrent narrative creations
- Resumable sessions
- State isolation between users
- Testable (deterministic UUIDs)

## Core MCP Tools

### 1. `create_narrative_session`

**Purpose**: Initialize a new narrative creation session

**When to call**: When user expresses intent to create a narrative

**Input**:
```json
{
  "description": "User's initial description of what they want to create"
}
```

**Output**:
```json
{
  "narrative_id": "uuid-here",
  "suggested_name": "Extracted from description",
  "analysis": {
    "detected_acts": ["act1", "act2"],
    "complexity": "simple|moderate|complex",
    "recommendations": ["Consider adding X", "Y might need Z"]
  }
}
```

**Implementation**:
- Generate UUID for session
- Create empty PartialNarrative
- Store in registry
- Use LLM to analyze description → suggest structure
- Return analysis to guide LLM's next steps

### 2. `elicit_metadata`

**Purpose**: Set or update narrative metadata

**When to call**: 
- After session creation (initial metadata)
- When user wants to change name/description/defaults

**Input**:
```json
{
  "narrative_id": "uuid",
  "name": "optional - if provided, update",
  "description": "optional - if provided, update", 
  "model": "optional - default model",
  "temperature": 0.7,
  "max_tokens": 1000
}
```

**Output**:
```json
{
  "success": true,
  "current_metadata": {
    "name": "current_name",
    "description": "current_description",
    "model": "claude-3-5-sonnet-20241022",
    "temperature": 0.7,
    "max_tokens": 1000
  },
  "missing_required": ["description"],
  "validation_warnings": []
}
```

**Implementation**:
- Retrieve PartialNarrative by UUID
- Update only provided fields (partial update pattern)
- Validate completeness
- Return current state + what's missing

### 3. `elicit_acts`

**Purpose**: Define workflow steps (acts) for the narrative

**When to call**:
- After metadata is minimally complete
- When user describes workflow/steps
- When modifying existing acts

**Input**:
```json
{
  "narrative_id": "uuid",
  "mode": "ai_assisted|sequential|named|replace",
  "acts": [
    {
      "name": "act_name",
      "prompt": "What this act should do",
      "model": "optional override",
      "temperature": 0.8
    }
  ],
  "act_description": "For ai_assisted mode - natural language description"
}
```

**Modes**:
- `ai_assisted`: LLM uses botticelli_mcp create_narrative to extract acts from description
- `sequential`: Number acts (act1, act2, ...) - simple workflows
- `named`: User provides explicit names - complex workflows  
- `replace`: Replace entire act list (for modifications)

**Output**:
```json
{
  "success": true,
  "acts": [
    {"name": "act1", "prompt": "...", "has_inputs": false},
    {"name": "act2", "prompt": "...", "has_inputs": false}
  ],
  "act_order": ["act1", "act2"],
  "suggestions": {
    "act1": "Consider adding a database query input",
    "act2": "This might benefit from image input"
  }
}
```

**Implementation**:
- For `ai_assisted`: Call internal create_narrative tool, parse acts
- For `sequential`/`named`: Create acts from provided list
- For `replace`: Full replacement
- Update PartialNarrative.acts and act_order
- Analyze each act → suggest inputs

### 4. `elicit_inputs`

**Purpose**: Add inputs to specific acts

**When to call**:
- After acts are defined
- When user mentions data sources, images, documents, etc.
- When adding context to specific workflow steps

**Input**:
```json
{
  "narrative_id": "uuid",
  "act_name": "which_act",
  "inputs": [
    {
      "type": "Text|Image|Audio|Video|Document|BotCommand|Table|Narrative",
      "content": "type-specific content",
      "metadata": {}
    }
  ],
  "mode": "append|replace"
}
```

**Input Type Schemas**:

```json
// Text
{"type": "Text", "content": "prompt text"}

// Image
{
  "type": "Image",
  "source": "url|base64",
  "url": "https://...",
  "base64": "data:image/png;base64,...",
  "media_type": "image/png"
}

// BotCommand
{
  "type": "BotCommand",
  "platform": "discord|bluesky|mastodon",
  "command": "server.get_stats",
  "args": {"guild_id": "123"},
  "use_cache": true,
  "history_retention": "session|persistent|none"
}

// Table
{
  "type": "Table",
  "table_name": "users",
  "columns": ["id", "name", "email"],
  "where_clause": "created_at > NOW() - INTERVAL '7 days'",
  "order_by": "created_at DESC",
  "limit": 100,
  "format": "csv|json|markdown"
}

// Narrative (composition)
{
  "type": "Narrative",
  "narrative_id": "other-narrative-uuid",
  "pass_through_inputs": true
}
```

**Output**:
```json
{
  "success": true,
  "act_name": "act1",
  "inputs_count": 3,
  "validation": {
    "warnings": ["Table query might be slow without index"],
    "errors": []
  }
}
```

**Implementation**:
- Retrieve PartialNarrative and specific act
- Parse input type, validate schema
- Append or replace inputs
- Validate inputs (table exists, narrative_id valid, etc.)
- Update PartialAct.inputs

### 5. `elicit_carousel`

**Purpose**: Configure iterative execution (loops)

**When to call**:
- When user mentions "repeat", "iterate", "for each", "multiple times"
- After acts/inputs are defined
- When configuring batch processing

**Input**:
```json
{
  "narrative_id": "uuid",
  "level": "narrative|act",
  "act_name": "if level=act, which act",
  "iterations": 10,
  "continue_on_error": false,
  "estimated_tokens_per_iteration": 500,
  "budget_multiplier": 2.0
}
```

**Output**:
```json
{
  "success": true,
  "carousel_config": {
    "level": "act",
    "act_name": "act2",
    "iterations": 10,
    "estimated_total_tokens": 5000,
    "budget_warnings": ["May exceed default budget limits"]
  }
}
```

**Implementation**:
- Create CarouselConfig from parameters
- Validate iteration count (1-1000)
- Estimate total tokens
- Attach to narrative-level or act-level

### 6. `validate_narrative`

**Purpose**: Validate narrative completeness and correctness

**When to call**:
- Before presenting narrative to user as "complete"
- After significant changes
- When user asks "is this ready?"

**Input**:
```json
{
  "narrative_id": "uuid",
  "strict": false
}
```

**Output**:
```json
{
  "valid": false,
  "errors": [
    {
      "severity": "critical|high|medium|low",
      "field": "acts.act1.inputs",
      "message": "Act 'act1' has no prompt defined",
      "suggestion": "Add a prompt describing what this act should do",
      "auto_fixable": true
    }
  ],
  "warnings": [
    {
      "severity": "medium",
      "field": "temperature",
      "message": "No default temperature set",
      "suggestion": "Consider setting temperature to 0.7"
    }
  ],
  "completeness": {
    "metadata": "complete",
    "acts": "incomplete",
    "inputs": "partial",
    "overall": "60%"
  }
}
```

**Implementation**:
- Call PartialNarrative.validate()
- Use botticelli_narrative::validator
- Classify errors by severity
- Suggest fixes
- Return structured validation report

### 7. `apply_validation_fixes`

**Purpose**: Auto-fix common validation errors

**When to call**:
- After validate_narrative shows auto_fixable errors
- When LLM decides to fix issues before asking user

**Input**:
```json
{
  "narrative_id": "uuid",
  "fix_types": ["empty_toc", "missing_defaults", "all"],
  "confirm": false
}
```

**Output**:
```json
{
  "success": true,
  "fixes_applied": [
    "Added TOC entries from acts",
    "Set default temperature to 0.7",
    "Added placeholder prompts to 2 acts"
  ],
  "remaining_errors": 3
}
```

### 8. `get_narrative_state`

**Purpose**: Retrieve current narrative state

**When to call**:
- To show user current progress
- Before making decisions about next steps
- For debugging

**Input**:
```json
{
  "narrative_id": "uuid",
  "format": "summary|full|toml"
}
```

**Output**:
```json
{
  "narrative_id": "uuid",
  "state": {
    "name": "MyBot",
    "acts_count": 3,
    "acts": ["act1", "act2", "act3"],
    "completeness": "60%",
    "has_carousel": false
  },
  "toml": "# if format=toml, full TOML content"
}
```

### 9. `finalize_narrative`

**Purpose**: Convert PartialNarrative to final TOML and save

**When to call**:
- When user confirms narrative is ready
- After validation passes (or user accepts warnings)

**Input**:
```json
{
  "narrative_id": "uuid",
  "save_to_db": true,
  "file_path": "optional - save to file"
}
```

**Output**:
```json
{
  "success": true,
  "toml": "full TOML content",
  "saved_to_db": true,
  "db_id": 123,
  "file_path": "/path/to/narrative.toml",
  "validation_summary": {
    "errors": 0,
    "warnings": 2
  }
}
```

**Implementation**:
- Call PartialNarrative.to_toml()
- Save to database if requested
- Write file if path provided
- Clean up session (remove from registry)

### 10. `update_narrative_field`

**Purpose**: Generic field update for modifications

**When to call**:
- User wants to change specific field
- Iterative refinement during conversation

**Input**:
```json
{
  "narrative_id": "uuid",
  "updates": [
    {"path": "name", "value": "NewName"},
    {"path": "acts.act1.prompt", "value": "New prompt"},
    {"path": "acts.act2.temperature", "value": 0.8}
  ]
}
```

**Output**:
```json
{
  "success": true,
  "updated_fields": ["name", "acts.act1.prompt", "acts.act2.temperature"],
  "validation": {"errors": [], "warnings": []}
}
```

## Sampling Strategy Patterns

### Pattern 1: Quick Creation (Minimal Interaction)

```
User: "Create a narrative about a greeting bot"
  ↓
LLM: [Calls create_narrative_session]
LLM: [Calls elicit_metadata with name="greeting_bot"]  
LLM: [Calls elicit_acts mode=ai_assisted, act_description="greeting bot"]
LLM: [Calls validate_narrative]
LLM: [Calls finalize_narrative]
  ↓
LLM: "I've created a greeting bot narrative with 2 acts. Would you like to modify it?"
```

### Pattern 2: Guided Detailed Creation

```
User: "Help me create a customer support bot"
  ↓
LLM: "What should we call it?"
User: "SupportBot"
  ↓
LLM: [Calls create_narrative_session]
LLM: [Calls elicit_metadata name="SupportBot"]
LLM: "What are the main steps SupportBot should perform?"
User: "Greet, understand issue, query database, provide solution"
  ↓
LLM: [Calls elicit_acts mode=sequential, acts=[...]]
LLM: "For understanding the issue, should the bot read from a database?"
User: "Yes, query the issues table"
  ↓
LLM: [Calls elicit_inputs act="act2", inputs=[Table{table="issues"}]]
LLM: [Calls validate_narrative]
LLM: "Your narrative is ready! Summary: ..."
```

### Pattern 3: Iterative Refinement

```
User: "Create a bot" 
LLM: [Creates basic bot]
User: "Add image processing"
  ↓
LLM: [Calls update_narrative_field, adds new act]
LLM: [Calls elicit_inputs act="image_act", inputs=[Image{...}]]
  ↓
User: "Make it run 10 times"
LLM: [Calls elicit_carousel iterations=10]
  ↓
User: "Change the name to ImageBot"
LLM: [Calls update_narrative_field path="name", value="ImageBot"]
```

### Pattern 4: Complex Composition

```
User: "Create a pipeline: fetch data, process, store results"
  ↓
LLM: [Calls create_narrative_session]
LLM: [Calls elicit_acts mode=named, acts=[
       {name: "fetch", ...},
       {name: "process", ...},
       {name: "store", ...}
     ]]
  ↓
LLM: "For the fetch step, what's the data source?"
User: "Query users table"
  ↓
LLM: [Calls elicit_inputs act="fetch", inputs=[Table{table="users"}]]
LLM: "Should the process step use the fetched data?"
User: "Yes, and also call another narrative"
  ↓
LLM: [Calls elicit_inputs act="process", inputs=[
       Narrative{narrative_id="other-narrative"},
       ...
     ]]
```

## LLM System Prompt Guidelines

### Core Instructions

```markdown
You are a narrative creation assistant. Your role is to help users create 
narrative TOML files through natural conversation.

You have access to narrative elicitation tools. Use them to:
1. Initialize narrative sessions
2. Gather required information progressively  
3. Validate completeness
4. Save final narratives

**Tools Available**:
- create_narrative_session: Start new narrative
- elicit_metadata: Set name, description, defaults
- elicit_acts: Define workflow steps
- elicit_inputs: Add data sources to acts
- elicit_carousel: Configure iterations
- validate_narrative: Check completeness
- apply_validation_fixes: Auto-fix errors
- get_narrative_state: Check progress
- update_narrative_field: Modify fields
- finalize_narrative: Save completed narrative

**Conversation Strategy**:
1. Understand user intent first - don't immediately start creating
2. Ask clarifying questions before calling tools
3. Call tools based on conversational context, not a fixed sequence
4. Use validate_narrative before presenting as "complete"
5. Allow iterative refinement - users can change anything
6. Explain what you're doing without overwhelming technical details

**Progressive Disclosure**:
- Start simple (name, basic structure)
- Add complexity as needed (inputs, carousels, etc.)
- Don't ask about optional features unless user needs them
- Validate before adding more complexity

**Error Handling**:
- If validation fails, explain issues clearly
- Suggest fixes using user-friendly language
- Use apply_validation_fixes for simple issues
- Ask user for guidance on complex issues

**State Management**:
- Track narrative_id throughout conversation
- Use get_narrative_state to check progress
- Call finalize_narrative only when user confirms ready
```

### Tool Call Decision Tree

```
User message received
  ↓
Does message express intent to create new narrative?
  YES → Call create_narrative_session
  NO → Continue
  ↓
Does message contain workflow steps/acts?
  YES → Call elicit_acts (mode based on specificity)
  NO → Continue
  ↓
Does message mention data sources (tables, images, etc.)?
  YES → Call elicit_inputs with appropriate type
  NO → Continue
  ↓
Does message mention iteration/repetition?
  YES → Call elicit_carousel
  NO → Continue
  ↓
Does message request changes to existing fields?
  YES → Call update_narrative_field
  NO → Continue
  ↓
Does message ask if narrative is ready?
  YES → Call validate_narrative
  NO → Continue
  ↓
Does user confirm narrative is complete?
  YES → Call finalize_narrative
  NO → Continue conversation
```

## Implementation Phases

### Phase 1: MCP Tool Infrastructure (Week 1)

**Goal**: Expose elicitation tools via MCP

**Tasks**:
1. Create `botticelli_mcp/src/tools/elicitation/` module
2. Implement NarrativeRegistry (UUID → PartialNarrative map)
3. Implement each tool (create_session, elicit_metadata, etc.)
4. Register tools in MCP server
5. Write tool schemas (JSON Schema)

**Files**:
```
crates/botticelli_mcp/src/tools/elicitation/
├── mod.rs
├── registry.rs          # NarrativeRegistry
├── session.rs           # create_narrative_session
├── metadata.rs          # elicit_metadata
├── acts.rs              # elicit_acts
├── inputs.rs            # elicit_inputs
├── carousel.rs          # elicit_carousel
├── validation.rs        # validate_narrative, apply_fixes
├── state.rs             # get_narrative_state
└── finalize.rs          # finalize_narrative
```

**Testing**:
- Mock tool calls with predefined inputs
- Verify state consistency across calls
- Test concurrent narratives (different UUIDs)

### Phase 2: LLM Integration (Week 2)

**Goal**: Enable LLM to use tools in conversation

**Tasks**:
1. Create sampling strategy system prompt
2. Integrate with CommandExecutor
3. Handle tool calls in chat loop
4. Track conversation context
5. Test end-to-end flows

**Files**:
```
crates/botticelli_chat/src/
├── sampling_strategy.rs  # System prompt + decision logic
└── narrative_agent.rs    # High-level orchestration
```

**Testing**:
- Simulate user conversations
- Verify tool call sequences
- Test error recovery
- Test iterative refinement

### Phase 3: Optimization (Week 3)

**Goal**: Improve UX and performance

**Tasks**:
1. State persistence (Redis/file)
2. Tool call batching (multiple updates in one call)
3. Smart defaults (reduce questions)
4. Context-aware suggestions
5. Performance profiling

**Enhancements**:
- Cache narrative templates
- Pre-analyze descriptions for better suggestions
- Batch validation with fix suggestions
- Session resumption after disconnect

## Success Metrics

**User Experience**:
- Narrative creation time < 5 minutes
- Average questions asked < 10
- User satisfaction with generated narratives > 80%

**Technical**:
- Tool call success rate > 95%
- Validation accuracy > 90%
- State consistency across sessions = 100%

**Composability**:
- Complex narratives (5+ acts) creatable without issues
- Nested narratives (composition) working correctly
- All input types supported and validated

## Open Questions

1. **Session Timeout**: How long to keep PartialNarrative in registry?
   - Proposal: 1 hour idle timeout, persist to disk after

2. **Concurrent Editing**: Can multiple users edit same narrative?
   - Proposal: Not in MVP - one owner per narrative_id

3. **Template Library**: Should we provide narrative templates?
   - Proposal: Phase 3 - "Create from template" tool

4. **Undo/Redo**: Should we track change history?
   - Proposal: Phase 3 - snapshot-based undo

5. **Multi-Modal Input**: How to handle image upload in chat?
   - Proposal: Use base64 encoding in chat, save to media storage

## References

- [NARRATIVE_ELICITATION_PLAN.md](./NARRATIVE_ELICITATION_PLAN.md) - Elicitation layer
- [ELICITATION_SYSTEM_COMPLETE.md](./ELICITATION_SYSTEM_COMPLETE.md) - Implementation summary
- [NARRATIVE_TOML_SPEC.md](./NARRATIVE_TOML_SPEC.md) - TOML format
- [MCP.md](./MCP.md) - MCP server implementation
- [AI_NARRATIVE_TOML_GUIDE.md](./AI_NARRATIVE_TOML_GUIDE.md) - Narrative authoring

## Next Steps

1. Review and approve this sampling strategy
2. Begin Phase 1: MCP tool infrastructure
3. Create tool schemas (JSON Schema definitions)
4. Implement NarrativeRegistry
5. Implement first tool (create_narrative_session)
6. Test tool in isolation
7. Iterate through remaining tools

---

**Status**: 🚧 In Progress - Phase 1
**Last Updated**: 2025-12-14  
**Author**: Claude + Erik (Co-authored)

## Implementation Status

### Completed
- ✅ Sampling strategy design document
- ✅ Core function implementations (carousel, validation, state, update)
- ✅ PartialNarrative and registry structure in elicitation module

### In Progress
- 🚧 Converting elicitation functions to McpTool trait implementations
- 🚧 Fixing compilation errors in elicitation tools
- 🚧 Proper error handling and async support

### Next Steps
1. Fix existing elicitation tool stubs to use McpTool trait pattern
2. Complete tool registration in server router  
3. Test tools individually via MCP protocol
4. Implement Phase 2 LLM integration

### Known Issues (Resolved)
- ~~Elicitation tool files use old `Tool` trait~~ - Old stubs removed
- ~~Need async/await throughout tool implementations~~ - Cleaned up
- ~~Image/Document source types need correct imports~~ - Fixed
- ~~Tool registration not yet added to server router~~ - Using existing registry

### Recent Updates (2025-12-14)
- ✅ Removed broken elicitation tool stubs (create_session, elicit_acts, etc.)
- ✅ Added sampling infrastructure (`LlmSampler`, `SamplingSession`, `Turn`)
- ✅ Created `SamplingHelper` with system prompt templates
- ✅ Added placeholder `ChatLlmSampler` for future implementation
- ✅ Fixed PartialAct/PartialNarrative field visibility issues
- ✅ Code compiles successfully

### Required for Full Implementation
1. Define `LlmClient` trait in `botticelli_core`
2. Implement tool call extraction from LLM responses
3. Complete `ChatLlmSampler` with multi-turn conversation logic
4. Integration tests for sampling workflows
