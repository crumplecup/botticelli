# MCP Tool Implementation Roadmap

## Status: Fix Applied - Needs Verification

## Problem Discovery & Fix

**Root Cause Found:** Tools were being registered via `register_internal_tools()` but:
1. ✅ Database pool was NOT being created in `botticelli-chat.rs`
2. ✅ **FIXED**: Now creating db pool and passing it to `register_internal_tools()`
3. Tools should now be properly registered when database feature is enabled

## Current State (After Fix)

### ✅ What's Working
- Tool registry architecture (`ToolRegistry` in botticelli_mcp_client)
- `register_internal_tools()` function registers:
  - Database tools (create_table, query_table, inspect_table, table_exists)
  - Elicitation tools (create_elicitation_session, elicit_metadata, elicit_act, finalize_elicitation)
  - Carousel tools (create_carousel, execute_carousel)
- Database pool now created and passed to registration

### ❓ Needs Verification
1. Are all tools now visible to LLM?
2. Do tools execute successfully?
3. Are narrative tools (create_narrative, etc.) implemented?
4. Are actor/scene tools implemented?

## Phase 1: Verification ⏳ NEXT

### Task 1.1: Test Tool Visibility
**Objective:** Verify tools are exposed to LLM after db pool fix

**Actions:**
1. Run `just chat`
2. Ask LLM: "What tools do you have available?"
3. Check `botticelli-chat.log` for "Registered all internal tools"
4. Verify tool count > 4

**Success Criteria:**
- LLM lists 10+ tools (database + elicitation + carousel)
- Log shows successful registration
- No registration errors

### Task 1.2: Audit Missing Tools
**Objective:** Identify designed tools not yet implemented

**Check against design:**
- Narrative tools (create, list, load, validate, delete)
- Actor tools (create, list, update, delete)
- Scene tools (create, list, update, delete)
- Media tools (upload, retrieve, analyze)

## Phase 2: Implementation (If Needed)

### Task 2.1: Implement Missing Narrative Tools
- Design database schema if needed
- Create tool implementations
- Register in `register_internal_tools()`

### Task 2.2: Implement Actor Tools
- Design Actor CRUD operations
- Create tool implementations
- Register tools

### Task 2.3: Implement Scene Tools
- Design Scene CRUD operations
- Create tool implementations
- Register tools

## Phase 3: Integration Testing

### Task 3.1: End-to-End Tool Flow
- User sends narrative creation request
- LLM calls appropriate tools
- Tools execute and return results
- LLM synthesizes response

### Task 3.2: Error Handling
- Test missing parameters
- Test database connection failures
- Test invalid tool calls

## Success Metrics

- [ ] All tools visible to LLM
- [ ] Tools execute successfully
- [ ] Errors handled gracefully
- [ ] Complete conversation flow works

## Immediate Next Step

**RUN VERIFICATION**: Test `just chat` and verify tool count increased from 4 to 10+
