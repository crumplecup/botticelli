# MCP Integration Completion Plan

**Status:** In Progress  
**Created:** 2025-12-05  
**Last Updated:** 2025-12-05

## Progress Summary

**Phase 1: Validation Integration** - ✅ COMPLETE
- Added `dotenvy::dotenv()` to MCP server binary for environment variable support
- Validation tool already implemented and working
- Ready for user testing with Claude Desktop

**Phase 2: Execution Tools** - ✅ COMPLETE
- Added `execute_act` tool for single LLM calls without narrative overhead
- Supports all 5 backends (gemini, anthropic, ollama, huggingface, groq)
- Auto-selects backend based on model prefix
- Added error types: `BackendUnavailable`, `UnsupportedModel`, `ExecutionError`

**Phase 3: Observability** - ✅ COMPLETE
- Enhanced narrative executor with comprehensive timing metrics
- Track execution duration per act and total narrative duration
- Record performance data in OpenTelemetry spans for dashboard integration
- Summary logging includes: total acts, duration, average per-act timing
- Infrastructure ready for token counting when drivers implement TokenCounting trait

**Phase 6: Discord MCP Tools** - 🚧 IN PROGRESS
- Discord tools implemented and registered
- Basic tests added
- Needs feature flag verification and full integration testing

## Current State Analysis

### What's Implemented (Actual Codebase)

#### Phase 1-5: Core Infrastructure ✅
- **MCP Server**: JSON-RPC over stdio via `ByteTransport`, complete Router implementation
- **Validation Tools**: `validate_narrative` with syntax, structure, reference, model, and circular dependency checks
- **Execution Tools**: `execute_narrative` with full multi-backend support (gemini, anthropic, ollama, huggingface, groq)
- **LLM Integration**: Feature-gated drivers with runtime API key validation and error handling
- **Resources**: `NarrativeResource` and `ContentResource` (feature-gated) with full URI handling
- **Database Tools**: `query_content` for PostgreSQL database queries (feature-gated)

**Tools Available:**
1. `echo` - Connection test
2. `server_info` - Server metadata
3. `validate_narrative` - TOML validation
4. `generate` - Text generation framework
5. `execute_act` - Single act execution (NEW - Phase 2)
6. `execute_narrative` - Narrative execution
7. `generate_gemini` - Google Gemini
8. `generate_anthropic` - Anthropic Claude
9. `generate_ollama` - Local Ollama
10. `generate_huggingface` - HuggingFace
11. `generate_groq` - Groq
12. `query_content` - Database queries

#### Existing Social Media Infrastructure

**botticelli_social crate:**
- `discord/client.rs` - Discord API client
- `discord/commands.rs` - Command implementations
- `discord/handler.rs` - Event handlers
- `discord/repository.rs` - Database operations
- `discord/processors.rs` - Message processing
- `bot_commands.rs` - Bot command execution

**Key Discord Capabilities:**
- HTTP-based Discord client (not serenity/gateway)
- Repository pattern for database operations
- Command execution framework
- Message/channel/guild operations
- State management

### Gaps Analysis

#### Missing MCP Integration

**Phase 6: Advanced Execution Features** (Partially Complete)
- ✅ Basic narrative execution
- ✅ Multi-backend LLM support
- ❌ Processors for structured data extraction
- ❌ Bot command integration (Discord queries in narratives)
- ❌ Table query integration (database lookups in narratives)
- ❌ State management across executions
- ❌ Carousel execution (looping narratives)

**Phase 7: Observability** - ✅ COMPLETE
- ✅ Trace each act execution (via #[instrument])
- ✅ Duration tracking per act and total narrative execution
- ✅ Performance metrics recorded in tracing spans
- ✅ Summary logging with act count and timing statistics
- ⏳ Token usage tracking (waiting for TokenCounting trait implementation in drivers)
- ⏳ Cost monitoring (depends on token tracking)

**Phase 8: Social Media Integration** (Not Started)
- ❌ Post to Discord via MCP tools
- ❌ Query Discord data via MCP tools
- ❌ Content generation → posting pipeline
- ❌ Scheduled narrative execution

## Implementation Plan

### Phase 6: Discord MCP Tools

**Goal:** Expose Discord operations as MCP tools for LLM usage.

#### Tools to Implement

1. **`discord_post_message`**
   - Post message to specific channel
   - Input: `channel_id`, `content`, optional `embed`
   - Uses: `botticelli_social::discord::client`
   - Output: Message ID, timestamp

2. **`discord_get_channels`**
   - List channels in guild
   - Input: `guild_id`, optional `type_filter`
   - Uses: `botticelli_social::discord::repository`
   - Output: Array of channel info

3. **`discord_get_messages`**
   - Fetch message history
   - Input: `channel_id`, `limit`, optional `before`/`after`
   - Uses: `botticelli_social::discord::client`
   - Output: Array of messages

4. **`discord_get_guild_info`**
   - Get guild metadata
   - Input: `guild_id`
   - Output: Guild name, member count, channels

5. **`discord_execute_bot_command`**
   - Run bot command through MCP
   - Input: `command_name`, `args`
   - Uses: `botticelli_social::bot_commands`
   - Output: Command result

#### Implementation Steps

1. **Create `src/tools/discord.rs`**
   - Implement trait `McpTool` for each Discord tool
   - Feature gate with `#[cfg(feature = "discord")]`
   - Add to Cargo.toml: `discord = ["botticelli_social", "database"]`

2. **Add to ToolRegistry**
   - Register conditionally in `tools/mod.rs`
   - Log availability based on credentials

3. **Environment Configuration**
   - `DISCORD_TOKEN` - Bot token
   - `DISCORD_APPLICATION_ID` - Application ID
   - Validate on tool registration

4. **Tests**
   - Mock Discord API responses
   - Test tool registration with/without credentials
   - Integration tests with actual API (feature-gated)

5. **Documentation**
   - Update MCP.md with new tools
   - Add examples to each tool
   - Document required permissions

### Phase 7: Observability Integration

**Goal:** Track narrative execution metrics for cost/performance analysis.

#### Features to Implement

1. **Execution Tracing**
   - Instrument `ExecuteNarrativeTool::execute`
   - Add span for each act with attributes:
     - Act name
     - Model used
     - Token count (input/output)
     - Duration
     - Status (success/error)

2. **Token Tracking**
   - Extract token counts from LLM responses
   - Aggregate per narrative execution
   - Store in execution result JSON

3. **Cost Calculation**
   - Model pricing table (configurable)
   - Calculate cost per act
   - Sum total narrative cost
   - Include in tool output

4. **Metrics Export**
   - Prometheus metrics for:
     - Narrative executions (counter)
     - Token usage by model (histogram)
     - Execution duration (histogram)
     - Cost per execution (histogram)
   - Expose via HTTP endpoint (optional feature)

#### Implementation Steps

1. **Add Metrics Structs**
   - `ExecutionMetrics` - per-narrative aggregates
   - `ActMetrics` - per-act details
   - Include in `execute_narrative` output

2. **Instrument Execution**
   - Add `#[instrument]` to act execution
   - Extract metrics from LLM responses
   - Aggregate in execution loop

3. **Pricing Configuration**
   - `pricing.toml` - Model costs (input/output per 1M tokens)
   - Load at startup
   - Calculate dynamically

4. **Prometheus Integration**
   - Feature gate: `#[cfg(feature = "metrics")]`
   - Optional HTTP server on separate port
   - Export standard MCP metrics

5. **Tests**
   - Verify metrics collection
   - Test cost calculation
   - Mock token counts

### Phase 8: Social Media Workflows

**Goal:** Enable end-to-end content generation → posting workflows.

#### Features to Implement

1. **Narrative → Discord Pipeline**
   - Execute narrative
   - Extract structured output
   - Post to Discord channel
   - Store content in database

2. **Template Variables from Discord**
   - Fetch guild/channel context
   - Pass as narrative variables
   - Enable context-aware generation

3. **Multi-Step Workflows**
   - Tool: `discord_content_workflow`
   - Input: narrative path, channel ID
   - Steps:
     1. Execute narrative
     2. Extract content
     3. Post to Discord
     4. Log result
   - Output: Message ID, metrics

4. **Scheduled Execution**
   - Tool: `schedule_narrative`
   - Input: narrative path, cron expression
   - Requires: Persistent scheduler (future)
   - For now: Document manual scheduling

#### Implementation Steps

1. **Workflow Tool**
   - Create `src/tools/discord_workflow.rs`
   - Combine execution + posting
   - Handle errors gracefully

2. **Content Extraction**
   - Parse narrative output for structured data
   - Support JSON/TOML sections
   - Extract for posting

3. **Database Logging**
   - Store execution records
   - Link to Discord messages
   - Track success/failure

4. **Documentation**
   - End-to-end examples
   - Common patterns
   - Error handling

## Success Criteria

### Phase 6 Complete
- [ ] 5 Discord tools implemented and tested
- [ ] Feature-gated with `discord` feature
- [ ] Environment-based credential management
- [ ] Documentation updated
- [ ] Integration tests passing

### Phase 7 Complete
- [ ] Execution metrics collected
- [ ] Token tracking functional
- [ ] Cost calculation accurate
- [ ] Prometheus export working (optional)
- [ ] Tests for all metrics

### Phase 8 Complete
- [ ] Content workflow tool functional
- [ ] End-to-end examples documented
- [ ] Database logging working
- [ ] Error handling robust

## Dependencies

### Crate Dependencies
- `botticelli_social` - Discord operations
- `botticelli_database` - Database operations
- `botticelli_narrative` - Narrative execution
- `botticelli_models` - LLM backends

### External Dependencies
- Discord token and application ID
- Database connection (for logging)
- LLM API keys (existing)

### Optional Dependencies
- `prometheus` crate (metrics feature)
- HTTP server (metrics export)

## Timeline Estimates

**Phase 6: Discord Tools**
- Implementation: 4-6 hours
- Testing: 2-3 hours
- Documentation: 1-2 hours
- **Total:** 7-11 hours

**Phase 7: Observability**
- Implementation: 3-4 hours
- Testing: 2 hours
- Documentation: 1 hour
- **Total:** 6-7 hours

**Phase 8: Workflows**
- Implementation: 3-4 hours
- Testing: 2 hours
- Documentation: 2 hours
- **Total:** 7-8 hours

**Grand Total:** 20-26 hours

## Open Questions

1. **Discord Permissions:** What minimum permissions needed for bot?
2. **Rate Limiting:** How to handle Discord rate limits in MCP tools?
3. **Metrics Storage:** Store metrics in database or just expose via Prometheus?
4. **Scheduling:** Defer to external scheduler or build internal?
5. **Content Format:** Standardize narrative output format for posting?

## Next Steps

1. Review this plan with stakeholders
2. Start Phase 6 implementation
3. Add to PLANNING_INDEX.md
4. Create feature branch: `mcp-discord-integration`
5. Implement incrementally with commits per tool

---

**Related Documents:**
- [MCP.md](./MCP.md) - Current MCP documentation
- [DISCORD_SCHEMA.md](./DISCORD_SCHEMA.md) - Discord data models
- [NARRATIVE_TOML_SPEC.md](./NARRATIVE_TOML_SPEC.md) - Narrative format

🤖 *Generated by Claude Code - Botticelli MCP Integration Planning*
