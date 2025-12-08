# Narrative Generation via MCP Tools

**Status**: Phase 1 Complete ✅  
**Created**: 2025-12-08  
**Updated**: 2025-12-08 17:10 UTC (Phase 1 implementation complete)  
**Goal**: Enable LLMs to generate and iteratively refine narrative TOML files from natural language

---

## Implementation Tracker

| Phase | Status | Description |
|-------|--------|-------------|
| **Phase 0** | ⏸️ Paused | Initial stateful approach (abandoned - wrong pattern) |
| **Phase 1** | �� Planning | Core tools: `create_narrative`, `modify_narrative`, `save_narrative` |
| **Phase 2** | ✅ Complete (2025-12-08) | Validation integration and error handling |
| **Phase 3** | 📋 Planned | Advanced features: templates, composition, resources |
| **Phase 4** | 📋 Planned | Intelligence: suggestions, optimization, best practices |

**Current Phase**: Phase 2 Complete, Ready for Phase 3  
**Blocked By**: None  

**Completed (Phase 1 - 2025-12-08)**: 
1. ✅ Deleted incorrect stateful implementation
2. ✅ Implemented Phase 1 tools following stateless pattern (789 lines)
3. ✅ All tools compile without errors or warnings
4. ✅ Created 10 comprehensive integration tests (all passing)
5. ✅ Wrote complete usage guide with examples (655 lines)
6. ✅ Committed to git (5 commits)
7. ✅ Updated planning index and documentation

**Completed (Phase 2 - 2025-12-08)**: 
1. ✅ Enhanced validation error messages with priorities (critical/high/medium/low)
2. ✅ Auto-fix for common TOML errors (missing sections, formatting)
3. ✅ Better TOML formatting and helpful comments
4. ✅ Structured validation results with fix suggestions
5. ✅ Created narrative_validation_helpers module (300+ lines)
6. ✅ Created 15 comprehensive tests for Phase 2 features (400+ lines)
7. ✅ All tests passing (25/25 total)
8. ✅ Committed to git (3 commits: bf2af84, 08ad509, 3dc2012)

**Next Actions**: 
1. ⏭️ Begin Phase 3: Advanced features (resources, templates)
2. ⏭️ Or test Phase 1+2 with MCP clients first

---

## Corrected Vision

Create MCP tools for **immutable narrative transformation** - each tool takes a complete narrative (or description) and returns a complete, valid narrative. No sessions, no state. The LLM maintains context across iterations by passing the current narrative forward in the conversation.

**Key Insight**: Tools are pure functions (TOML in → TOML out). The LLM handles iteration memory.

## Motivation

**Current state:**
- Users must manually write narrative TOML files
- Requires understanding complex TOML syntax and spec
- Trial-and-error process with validation errors
- High barrier to entry for new users

**Desired state:**
- Users describe what they want in natural language
- LLM generates/modifies narratives through iterative refinement
- Each iteration produces valid, executable TOML
- Automatic validation ensures compliance at every step
- Users can refine orthogonal aspects (model choice, prompt content, structure) independently

## Design Principles (Corrected)

1. **Stateless tools**: Each tool call is independent, no sessions
2. **Immutable transformations**: Narrative in → transformed narrative out
3. **Complete outputs**: Always return full, valid TOML
4. **LLM-driven iteration**: LLM passes narratives between tool calls
5. **Leverage existing infrastructure**: Use `toml_parser`, `validator`, existing patterns
6. **Match existing MCP patterns**: Follow `validate_narrative` and `execute_narrative` style

## Architecture (Corrected)

### Stateless Transformation Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    User + LLM Iteration                      │
│                                                              │
│  User: "Create Discord stats narrative"                     │
│    → LLM: create_narrative(desc) → TOML_v1                  │
│                                                              │
│  User: "Add image analysis"                                 │
│    → LLM: modify_narrative(TOML_v1, "add image") → TOML_v2  │
│                                                              │
│  User: "Change model to Claude"                             │
│    → LLM: modify_narrative(TOML_v2, "use claude") → TOML_v3 │
│                                                              │
│  User: "Save it"                                            │
│    → LLM: save_narrative(TOML_v3, path) → file written      │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    MCP Tools (Stateless)                     │
│                                                              │
│  create_narrative:                                           │
│    Input: description → Output: complete TOML               │
│                                                              │
│  modify_narrative:                                           │
│    Input: TOML + modification → Output: updated TOML        │
│                                                              │
│  save_narrative:                                             │
│    Input: TOML + path → Output: file written                │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              Internal Implementation                         │
│  • Parse existing TOML (if provided)                         │
│  • Apply transformation (LLM-assisted)                       │
│  • Validate result                                           │
│  • Serialize to TOML                                         │
│  • Return complete narrative                                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              Existing Infrastructure                         │
│  • toml_parser (parse/serialize TOML)                        │
│  • validator (validate_narrative_toml)                       │
│  • core (Narrative types)                                    │
│  • LLM backends (for generating TOML from descriptions)      │
└─────────────────────────────────────────────────────────────┘
```

### Key Differences from Original Plan

❌ **Old approach**: Stateful session builder with multiple incremental tools  
✅ **New approach**: Stateless transformation tools, LLM maintains context

❌ **Old**: `create_session() → add_act() → add_act() → finalize()`  
✅ **New**: `create_narrative() → modify_narrative() → modify_narrative()`

❌ **Old**: Session IDs, global state, cleanup tasks  
✅ **New**: Pure functions, no state, no cleanup

**Why this works**: LLMs have conversation context. They can hold the current narrative TOML and pass it to the next tool call.

## MCP Tools

### Phase 1: Core Tools (MVP)

#### Tool 1: `create_narrative`

**Purpose**: Generate a complete narrative from natural language description.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "description": {
      "type": "string",
      "description": "Natural language description of the narrative workflow"
    },
    "name": {
      "type": "string",
      "description": "Narrative name (alphanumeric + underscores)"
    },
    "default_model": {
      "type": "string",
      "description": "Optional default model for all acts",
      "enum": ["gemini-2.0-flash-exp", "claude-3-5-sonnet-20241022", "gpt-4", ...]
    },
    "default_temperature": {
      "type": "number",
      "description": "Optional default temperature (0.0-1.0)",
      "minimum": 0.0,
      "maximum": 1.0
    }
  },
  "required": ["description", "name"]
}
```

**Example Input:**
```json
{
  "description": "Fetch Discord server stats, analyze them with a dashboard image, and post the top 3 insights to a channel",
  "name": "discord_insights",
  "default_model": "gemini-2.0-flash-exp"
}
```

**Example Output:**
```json
{
  "toml": "[narrative]\nname = \"discord_insights\"\ndescription = \"Fetch Discord server stats, analyze them with a dashboard image, and post the top 3 insights to a channel\"\nmodel = \"gemini-2.0-flash-exp\"\n\n[toc]\norder = [\"fetch_stats\", \"analyze\", \"post_insights\"]\n\n[acts]\nfetch_stats = \"Fetch current Discord server statistics\"\nanalyze = \"Analyze the statistics and identify the top 3 insights\"\npost_insights = \"Post the insights to the Discord channel\"\n",
  "validation": {
    "valid": true,
    "errors": [],
    "warnings": []
  },
  "summary": "Created narrative with 3 acts: fetch_stats, analyze, post_insights",
  "suggestions": [
    "Consider adding bot commands for Discord integration",
    "Consider adding media input for dashboard image"
  ]
}
```

**Implementation:**
```rust
use crate::{McpError, McpResult, McpTool};
use async_trait::async_trait;
use botticelli_narrative::validator::validate_narrative_toml;
use serde_json::{json, Value};

pub struct CreateNarrativeTool {
    llm_client: Arc<dyn BotticelliDriver>,
}

#[async_trait]
impl McpTool for CreateNarrativeTool {
    fn name(&self) -> &str {
        "create_narrative"
    }

    fn description(&self) -> &str {
        "Generate a complete narrative TOML from a natural language description. \
         Returns valid, executable narrative ready for use or further refinement."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Natural language description of the narrative workflow"
                },
                "name": {
                    "type": "string",
                    "description": "Narrative name"
                },
                "default_model": {
                    "type": "string",
                    "description": "Optional default model"
                },
                "default_temperature": {
                    "type": "number",
                    "description": "Optional default temperature (0.0-1.0)"
                }
            },
            "required": ["description", "name"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        // 1. Extract inputs
        let description = input.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'description'".into()))?;
        
        let name = input.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'name'".into()))?;
        
        // 2. Generate TOML via LLM
        let prompt = format!(
            "Generate a valid narrative TOML file for this workflow:\n\n{}\n\n\
             Follow the NARRATIVE_TOML_SPEC. Include [narrative], [toc], and [acts] sections. \
             Use name '{}'. Return only the TOML, no explanations.",
            description, name
        );
        
        let toml = generate_toml_with_llm(&self.llm_client, &prompt).await?;
        
        // 3. Validate
        let validation = validate_narrative_toml(&toml);
        
        // 4. Return
        Ok(json!({
            "toml": toml,
            "validation": {
                "valid": validation.is_valid(),
                "errors": format_errors(&validation.errors),
                "warnings": format_warnings(&validation.warnings)
            },
            "summary": generate_summary(&toml)
        }))
    }
}
```

#### Tool 2: `modify_narrative`

**Purpose**: Transform an existing narrative based on natural language modification request.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "narrative_toml": {
      "type": "string",
      "description": "Existing narrative TOML to modify"
    },
    "modification": {
      "type": "string",
      "description": "Natural language description of the modification"
    },
    "save_to": {
      "type": "string",
      "description": "Optional path to save modified narrative"
    }
  },
  "required": ["narrative_toml", "modification"]
}
```

**Example Input:**
```json
{
  "narrative_toml": "[narrative]\nname = \"discord_insights\"\n...",
  "modification": "Add a bot command to fetch Discord stats using server.get_stats with guild_id 123456"
}
```

**Example Output:**
```json
{
  "toml": "[narrative]\nname = \"discord_insights\"\n...\n[bots.get_stats]\nplatform = \"discord\"\ncommand = \"server.get_stats\"\nguild_id = \"123456\"\n\n[acts]\nfetch_stats = \"bots.get_stats\"\n...",
  "validation": {
    "valid": true,
    "errors": [],
    "warnings": []
  },
  "changes": [
    "Added bot command 'bots.get_stats'",
    "Updated act 'fetch_stats' to reference bot command"
  ],
  "saved_to": null
}
```

**Use Cases:**
- Add/remove/modify acts
- Add bot commands, table queries, media
- Change model, temperature, max_tokens
- Reorder acts
- Add/modify narrative metadata

#### Tool 3: `save_narrative`

**Purpose**: Save narrative TOML to file.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "narrative_toml": {
      "type": "string",
      "description": "Narrative TOML to save"
    },
    "file_path": {
      "type": "string",
      "description": "Path where to save the file"
    },
    "overwrite": {
      "type": "boolean",
      "description": "Allow overwriting existing file",
      "default": false
    }
  },
  "required": ["narrative_toml", "file_path"]
}
```

**Example:**
```json
{
  "narrative_toml": "[narrative]\n...",
  "file_path": "./narratives/discord_insights.toml",
  "overwrite": false
}
```

### Phase 2: Validation & Error Handling

**Goals:**
- Detailed validation feedback
- Actionable error messages
- Warning detection
- Auto-fix suggestions

**Enhancements to existing tools:**
- Better validation integration
- Suggested fixes for common errors
- Schema compliance checking

### Phase 3: Advanced Features

**Goals:**
- Template-based generation
- Narrative composition (references)
- Resource definitions (bots, tables, media)
- Security policies

**New capabilities:**
- `create_narrative` accepts template parameter
- `modify_narrative` handles resource additions
- Support for all NARRATIVE_TOML_SPEC features

### Phase 4: Intelligence Features

**Goals:**
- Smart suggestions
- Optimization recommendations
- Best practice enforcement

**Features:**
- Suggest appropriate models per act
- Detect inefficient patterns
- Recommend resource reuse
- Token cost estimation

## Implementation Phases

### Phase 1: Core Tools ✅ COMPLETE (2025-12-08)

**Deliverables:**
- ✅ Planning document (NARRATIVE_GENERATION_MCP_PLAN.md - 586 lines)
- ✅ `CreateNarrativeTool` implementation (289 lines)
- ✅ `ModifyNarrativeTool` implementation (382 lines)
- ✅ `SaveNarrativeTool` implementation (118 lines)
- ✅ Integration tests (10 tests, all passing)
- ✅ Usage documentation (NARRATIVE_GENERATION_USAGE.md - 655 lines)

**Success Criteria:** ✅ ALL MET
- ✅ Generate simple text-only narratives
- ✅ Modify existing narratives (model, temperature, acts)
- ✅ Save to files with safety checks
- ✅ All generated narratives validate successfully

**Commits:**
- 7e988c8: feat(mcp): implement Phase 1 narrative generation tools
- f05111e: test(mcp): add comprehensive narrative generation tests
- dba7a8f: docs(mcp): add comprehensive narrative generation usage guide
- 3c07597: docs: update planning index with Phase 1 completion

### Phase 2: Validation Integration ✅ COMPLETE (2025-12-08)

**Deliverables:**
- ✅ Enhanced validation feedback with priorities
- ✅ Error message formatting with structure
- ✅ Auto-fix suggestions and automatic repairs
- ✅ Warning detection with severity levels
- ✅ narrative_validation_helpers module (300+ lines)

**Success Criteria:** ✅ ALL MET
- ✅ Detailed validation errors with priority levels
- ✅ Fix suggestions for all error types
- ✅ Auto-fix applied for common issues
- ✅ Warnings categorized by severity
- ✅ Formatted TOML with helpful comments

**Commits:**
- bf2af84: feat(mcp): implement Phase 2 enhanced validation
- 08ad509: docs: mark Phase 2 complete in narrative generation plan
- 3dc2012: test(mcp): add comprehensive Phase 2 validation tests

**Test Coverage:**
- narrative_validation_test.rs: 15 tests covering all Phase 2 features
- All Phase 2 functionality tested (auto-fix, formatting, comments, validation)

### Phase 3: Advanced Features (Week 3)

**Deliverables:**
- [ ] Resource support (bots, tables, media)
- [ ] Template-based generation
- [ ] Narrative composition
- [ ] Security policy support

**Success Criteria:**
- Generate complex multi-resource narratives
- Use existing narratives as templates
- Create narratives that reference other narratives

### Phase 4: Intelligence (Week 4)

**Deliverables:**
- [ ] Smart suggestions
- [ ] Optimization recommendations
- [ ] Best practice checks
- [ ] Token cost estimation

**Success Criteria:**
- Suggest appropriate models
- Detect inefficient patterns
- Estimate costs before execution

## Example Workflow

### Iteration 1: Initial Creation
```
User: "Create a narrative that analyzes Discord server stats"

LLM calls: create_narrative({
  description: "Analyze Discord server stats",
  name: "discord_analysis"
})

Returns: Basic narrative with 2-3 acts
```

### Iteration 2: Add Bot Command
```
User: "Add a bot command to fetch the stats"

LLM calls: modify_narrative({
  narrative_toml: <previous TOML>,
  modification: "Add bot command for Discord stats using server.get_stats"
})

Returns: Updated narrative with [bots.get_stats] section
```

### Iteration 3: Add Image Analysis
```
User: "Also analyze a dashboard screenshot"

LLM calls: modify_narrative({
  narrative_toml: <previous TOML>,
  modification: "Add media input for dashboard screenshot and analyze it"
})

Returns: Updated narrative with [media.dashboard] and modified acts
```

### Iteration 4: Change Model
```
User: "Use Claude for the analysis"

LLM calls: modify_narrative({
  narrative_toml: <previous TOML>,
  modification: "Set model to claude-3-5-sonnet for the analyze act"
})

Returns: Updated narrative with model override
```

### Iteration 5: Save
```
User: "Save this"

LLM calls: save_narrative({
  narrative_toml: <final TOML>,
  file_path: "./narratives/discord_analysis.toml"
})

Returns: Confirmation of saved file
```

## Testing Strategy

### Unit Tests
- TOML parsing/serialization
- Validation integration
- Error handling
- Edge cases

### Integration Tests
- End-to-end narrative generation
- Multi-iteration workflows
- File I/O operations
- LLM prompt effectiveness

### Manual Testing
- Real-world user scenarios
- Various narrative complexities
- Error recovery
- Suggestion quality

## Success Metrics

**Usability:**
- 90%+ of generated narratives validate on first try
- Users achieve goals in 3-5 iterations
- LLMs generate appropriate TOML without manual intervention

**Quality:**
- Generated narratives follow best practices
- Appropriate model selection
- Efficient resource usage
- Valid TOML syntax

**Performance:**
- Tool execution < 5 seconds
- LLM generation < 10 seconds
- File I/O < 1 second

## References

- [NARRATIVE_TOML_SPEC.md](./NARRATIVE_TOML_SPEC.md) - Complete TOML specification
- [NARRATIVE_VALIDATOR_DESIGN.md](./NARRATIVE_VALIDATOR_DESIGN.md) - Validation infrastructure
- [crates/botticelli_mcp/src/tools/validate_narrative.rs](./crates/botticelli_mcp/src/tools/validate_narrative.rs) - Existing validation tool pattern
- [crates/botticelli_mcp/src/tools/execute_narrative.rs](./crates/botticelli_mcp/src/tools/execute_narrative.rs) - Existing execution tool pattern
- [crates/botticelli_narrative/src/toml_parser.rs](./crates/botticelli_narrative/src/toml_parser.rs) - TOML parsing types

## Implementation Results

### Phase 1 Complete ✅

**Completed Steps:**
1. ✅ Updated planning document with corrected approach
2. ✅ Deleted incorrect stateful implementation
3. ✅ Implemented `CreateNarrativeTool` following existing patterns
4. ✅ Implemented `ModifyNarrativeTool`
5. ✅ Implemented `SaveNarrativeTool`
6. ✅ Created comprehensive tests (10/10 passing)
7. ✅ Wrote complete usage documentation

**Quality Metrics:**
- Lines of Code: 789 (implementation)
- Lines of Tests: 315 (integration tests)
- Lines of Docs: 1,241 (planning + usage)
- Build Status: Clean (0 errors, 0 warnings)
- Test Pass Rate: 100% (10/10)
- Time to Complete: ~2 hours

**Production Ready:** ✅ YES
- MCP server integration ready
- Full validation at every step
- Comprehensive error handling
- Safety features (overwrite protection, path validation)
- Complete documentation with examples

---

**Status**: Phase 1 Complete ✅ - Production Ready  
**Actual Effort**: 2 hours (Phase 1 only)  
**Next Phase**: Phase 2 - Enhanced Validation  
**Priority**: High - ready for real-world testing
