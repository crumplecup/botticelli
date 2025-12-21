# Chat MCP Tool Implementation Roadmap

## Status: ✅ ALL TOOLS IMPLEMENTED AND REGISTERED

All elicitation tools are now fully implemented and registered in server.rs!

### ✅ Implemented Tools (Registered in server.rs)

**Core Tools:**
- `EchoTool` - Simple echo for testing
- `ServerInfoTool` - Server metadata

**Narrative Creation (Session-based):**
- `CreateNarrativeSessionTool` - Start new narrative session
- `ElicitMetadataTool` - Gather narrative metadata
- `ElicitActTool` - Elicit act details
- `FinalizeNarrativeTool` - Complete and save narrative
- `ElicitCarouselTool` - Carousel-based elicitation workflow
- `GetNarrativeStateTool` - Query current narrative state
- `ValidateNarrativeSessionTool` - Validate in-progress narrative
- `ApplyValidationFixesTool` - Apply validation fixes

**Narrative Management:**
- `CreateNarrativeTool` - Direct narrative creation
- `ModifyNarrativeTool` - Edit narratives
- `SaveNarrativeTool` - Persist narratives
- `ValidateNarrativeTool` - Validate TOML structure

**Execution:**
- `GenerateTool` - Generic LLM generation
- `ExecuteActTool` - Execute single act
- `ExecuteNarrativeTool` - Execute full narrative

**Optional (Feature-gated):**
- `ExportMetricsTool` - Prometheus metrics export
- LLM provider tools (Gemini, Anthropic, Ollama, HuggingFace, Groq)
- Discord tools (post, get messages, guild info, channels, bot commands)
- `QueryContentTool` - Database content queries

## Next Steps

1. **Test the elicitation workflow** - Verify all tools work together
2. **Document the workflow** - Create user guide for narrative creation
3. **Add carousel item types** - Implement FreeResponse, Rating, FileUpload if needed

**Missing Carousel Item Types:**
Per carousel.rs, we support:
- ✅ `Text` - Simple text display
- ✅ `MultiChoice` - Multiple choice questions
- ❌ `FreeResponse` - Open-ended questions (not implemented)
- ❌ `Rating` - Rating scales (not implemented)
- ❌ `FileUpload` - File uploads (not implemented)

---

## Phase 1: Complete Existing Elicitation Tools

### Task 1.1: Register Missing Tools ✅ COMPLETE
Register the four missing elicitation tools in `server.rs`:
- [x] `ElicitCarouselTool`
- [x] `GetNarrativeStateTool`
- [x] `ValidateNarrativeSessionTool`
- [x] `ApplyValidationFixesTool`

**Success Criteria:**
- All four tools appear in `just chat` tool list
- Tools execute without panics
- Basic functionality works

### Task 1.2: Implement ExecuteCarouselTool
Replace stub implementation with proper carousel execution logic.

**Requirements:**
- Execute carousel workflow from registry
- Handle user responses
- Update narrative state
- Return execution results

**Files to modify:**
- `crates/botticelli_mcp/src/tools/elicitation/session_tools.rs`
- `crates/botticelli_mcp/src/tools/elicitation/carousel.rs`

**Success Criteria:**
- Tool executes carousel from start to finish
- Handles all carousel item types
- Updates narrative registry properly
- Returns structured results

---

## Phase 2: Implement Missing Carousel Item Types

### Task 2.1: FreeResponse Item Type
Support open-ended text questions.

**Implementation:**
- Add `FreeResponse` variant to `CarouselItemType`
- Add handler in `ElicitCarouselTool::execute`
- Add validation logic
- Update serialization

**Success Criteria:**
- Can create FreeResponse items
- LLM can provide text answers
- Answers stored in session state

### Task 2.2: Rating Item Type
Support rating scale questions (e.g., 1-5, 1-10).

**Implementation:**
- Add `Rating` variant with min/max/step
- Add handler in `ElicitCarouselTool::execute`
- Add range validation
- Update serialization

**Success Criteria:**
- Can create Rating items with configurable scales
- LLM can provide numeric ratings
- Out-of-range values rejected

### Task 2.3: FileUpload Item Type
Support file uploads (for future use).

**Implementation:**
- Add `FileUpload` variant
- Add handler (may be no-op for LLM context)
- Add file metadata tracking
- Update serialization

**Success Criteria:**
- Type compiles and serializes
- Documented as future extension
- Doesn't break existing carousels

---

## Phase 3: Enhanced Narrative Validation

### Task 3.1: Comprehensive Validation Rules
Expand validation beyond TOML syntax.

**Validation Rules:**
- Act structure (title, description, scenes)
- Scene structure (setup, response, validation)
- Variable references (undefined variables)
- Loop dependencies (infinite loops)
- Resource references (missing files)

**Files to modify:**
- `crates/botticelli_mcp/src/tools/elicitation/validation.rs`
- `crates/botticelli_core/src/narrative.rs`

**Success Criteria:**
- Catches semantic errors, not just syntax
- Provides actionable error messages
- Suggests fixes where possible

### Task 3.2: Interactive Validation Fixes
Make `ApplyValidationFixesTool` more interactive.

**Requirements:**
- Present validation errors to LLM
- LLM proposes fixes
- User confirms or modifies
- Apply fixes atomically

**Success Criteria:**
- LLM can fix common validation errors
- User retains control over changes
- Rollback on failure

---

## Phase 4: Advanced Elicitation Features

### Task 4.1: Conditional Carousels
Support conditional branching in carousels.

**Implementation:**
- Add `condition` field to `CarouselItem`
- Evaluate conditions based on prior responses
- Skip items when conditions false
- Update navigation logic

**Success Criteria:**
- Can express "if X then ask Y"
- Conditions evaluate correctly
- Navigation skips properly

### Task 4.2: Carousel Templates
Pre-built carousel templates for common patterns.

**Templates:**
- "Basic Story" - Title, genre, premise, characters
- "Multi-Act Adventure" - Acts, scenes, branching
- "Character-Driven" - Deep character focus
- "World-Building" - Setting, history, factions

**Success Criteria:**
- Templates loaded from resources
- LLM can select template
- User can customize after selection

### Task 4.3: Narrative Resume/Checkpoint
Save and resume in-progress narratives.

**Requirements:**
- Serialize session state to database
- Load prior session by ID
- Resume from last completed item
- Handle schema migrations

**Success Criteria:**
- Can pause and resume elicitation
- State persists across sessions
- No data loss on resume

---

## Phase 5: Integration & Polish

### Task 5.1: Tool Discovery & Documentation
Improve tool discoverability for LLMs.

**Improvements:**
- Rich tool descriptions with examples
- Parameter constraints documented
- Error response formats standardized
- Example workflows provided

**Success Criteria:**
- LLM selects correct tool 90%+ of time
- Fewer malformed requests
- Better error recovery

### Task 5.2: Observability & Metrics
Add detailed metrics for tool usage.

**Metrics:**
- Tool call counts by type
- Success/failure rates
- Execution latency (p50, p95, p99)
- Carousel completion rates
- Validation error frequencies

**Success Criteria:**
- Grafana dashboard shows tool metrics
- Can identify bottlenecks
- Track UX improvements

### Task 5.3: End-to-End Testing
Comprehensive integration tests.

**Test Scenarios:**
- Complete narrative elicitation flow
- Validation error handling
- Carousel branching logic
- Resume/checkpoint functionality
- Multi-tool workflows

**Success Criteria:**
- All scenarios pass
- < 5% flake rate
- Covers 80%+ of tool code

---

## Dependencies & Prerequisites

### External Services
- ✅ PostgreSQL (for session state)
- ✅ LLM providers (Gemini primary, Groq fallback)
- ✅ MCP server running

### Code Dependencies
- ✅ `ToolRegistry` refactor complete
- ✅ `ToolCalling` trait implemented
- ✅ Fallback architecture in place
- ❌ File storage for templates (Phase 4)
- ❌ Database schema for checkpoints (Phase 4)

---

## Success Criteria (Overall)

1. **Completeness:** All carousel item types implemented
2. **Reliability:** Tools execute without panics, handle errors gracefully
3. **Usability:** LLM can guide user through full narrative creation
4. **Observability:** Tool usage visible in logs and metrics
5. **Testability:** Integration tests cover major workflows
6. **Documentation:** All tools documented with examples

---

## Timeline Estimate

- **Phase 1:** 2-4 hours (registration + ExecuteCarouselTool)
- **Phase 2:** 4-6 hours (carousel item types)
- **Phase 3:** 6-8 hours (validation enhancements)
- **Phase 4:** 8-12 hours (advanced features)
- **Phase 5:** 4-6 hours (polish & testing)

**Total:** 24-36 hours of focused development

---

## Next Steps

1. ✅ **Immediate:** Register missing tools (Task 1.1)
2. **Next:** Implement ExecuteCarouselTool (Task 1.2)
3. **Then:** Add FreeResponse carousel items (Task 2.1)
4. **Future:** Conditional carousels and templates (Phase 4)
