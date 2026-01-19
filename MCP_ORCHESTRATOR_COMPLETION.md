# MCP Orchestrator Completion Plan

## Executive Summary

**Goal**: Complete the "tool everything" architecture by creating delegation wrappers in `botticelli_mcp` for all 294 tool primitives across dependency crates.

**Current State**:
- ✅ Dependencies: 294 atomic tool primitives (`#[tool]` on functions)
- ✅ BotticelliServer: 31 tools (just added extraction, narrative, rate_limit, security wrappers)
- ❌ **Gap**: 263 dependency tools NOT accessible through BotticelliServer

**Architecture Vision**:
```
┌─────────────────────────────────────────────────────────┐
│ LLM Clients (Claude, GPT, etc.)                          │
└────────────────────┬────────────────────────────────────┘
                     │ MCP Protocol
┌────────────────────▼────────────────────────────────────┐
│ BotticelliServer (#[tool_router])                        │
│ - Orchestrator with complete vocabulary                  │
│ - All 325 tools registered and discoverable              │
└────────────────────┬────────────────────────────────────┘
                     │ Delegation
┌────────────────────▼────────────────────────────────────┐
│ Dependency Crates (Primitives)                           │
│ - botticelli_core: 25 tools                              │
│ - botticelli_storage: 5 tools                            │
│ - botticelli_database: 54 tools                          │
│ - botticelli_models: 78 tools                            │
│ - botticelli_cache: 11 tools                             │
│ - botticelli_security: 41 tools                          │
│ - botticelli_rate_limit: 19 tools                        │
│ - botticelli_social: 7 tools                             │
│ - botticelli_narrative: 44 tools                         │
└─────────────────────────────────────────────────────────┘
```

**Benefits**:
- Single unified API for all workspace functionality
- LLMs can discover and compose ALL primitives
- Type-safe tool invocation with parameter validation
- Consistent instrumentation and error handling across all tools
- "Dictionary" of atomic operations LLMs can script together

---

## Tool Inventory

### Current Distribution

| Crate                    | Tools | Status      |
|--------------------------|-------|-------------|
| botticelli_models        | 78    | Needs wrappers |
| botticelli_database      | 54    | Needs wrappers |
| botticelli_narrative     | 44    | Needs wrappers |
| botticelli_security      | 41    | Needs wrappers |
| botticelli_core          | 25    | Needs wrappers |
| botticelli_rate_limit    | 19    | ✅ 9 done, 10 remain |
| botticelli_cache         | 11    | Needs wrappers |
| botticelli_social        | 7     | Needs wrappers |
| botticelli_storage       | 5     | Needs wrappers |
| **botticelli_mcp**       | **41**| **✅ Complete** |
| **TOTAL**                | **325** | **31 done, 294 remain** |

---

## Implementation Pattern

### Delegation Wrapper Template

```rust
// In crates/botticelli_mcp/src/rmcp_server/tools/{dependency}.rs

use crate::rmcp_server::BotticelliServer;
use botticelli_{dependency}::{function_name, ParamsType, ResultType};
use rmcp::tool;
use tracing::instrument;

impl BotticelliServer {
    /// Documentation copied from source function.
    #[tool]
    #[instrument(skip(self, params), fields(
        tool = "{dependency}_{function_name}",
        // Add contextual fields from params as needed
    ))]
    pub fn {dependency}_{function_name}(&self, params: ParamsType) -> ResultType {
        tracing::debug!("Delegating to {dependency}::{function_name}");
        
        // Simple delegation to dependency tool
        let result = botticelli_{dependency}::{function_name}(params);
        
        // Log result/error before returning
        match &result {
            Ok(_) => tracing::debug!("Delegation succeeded"),
            Err(e) => tracing::error!(error = ?e, "Delegation failed"),
        }
        
        result
    }
}
```

**Why Instrumentation on Wrappers?**

This creates a crucial observability layer:

```
TRACE mcp_server.database_create_pool  <- Wrapper span (orchestrator layer)
  TRACE database.create_pool            <- Primitive span (implementation)
    TRACE diesel.get_connection         <- Library span
```

Benefits:
1. **Call chain visibility** - See path through orchestrator to primitive
2. **Performance profiling** - Measure orchestration overhead vs implementation
3. **Error location** - Know if failure is in MCP layer or dependency
4. **Rate limiting** - Track tool usage patterns at orchestrator level
5. **AI debugging** - Gives LLMs precise failure context

### Key Principles

1. **Name prefixing**: `{crate}_{function}` to avoid collisions (e.g., `database_create_pool`)
2. **Simple delegation**: Wrappers just forward to dependency - no logic duplication
3. **Full instrumentation**: All wrappers have `#[instrument(skip(self, params))]` + debug/error logging
4. **Span context**: Add `fields(tool = "name")` to identify orchestrator layer in traces
5. **Result logging**: Log success/error before returning for observability
6. **Documentation**: Copy original doc comments from source functions
7. **Feature gates**: Match dependency feature requirements
8. **Type re-exports**: All parameter/result DTOs exported at `_mcp` crate level

---

## File Organization

Create one file per dependency crate in `crates/botticelli_mcp/src/rmcp_server/tools/`:

```
rmcp_server/tools/
├── core.rs              ✅ Already exists (needs expansion)
├── storage.rs           ✅ Already exists (needs expansion) 
├── database.rs          ❌ Create new (54 wrappers)
├── models.rs            ❌ Create new (78 wrappers)
├── cache.rs             ❌ Create new (11 wrappers)
├── security.rs          ✅ Already exists (1 done, 40 remain)
├── rate_limit.rs        ✅ Already exists (9 done, 10 remain)
├── social.rs            ❌ Create new (7 wrappers)
├── narrative_wrapper.rs ❌ Create new (44 wrappers)
└── mod.rs               ❌ Update exports
```

**Note**: `narrative.rs` already exists with 7 generic/DB wrappers. Create `narrative_wrapper.rs` for the 44 dependency primitives to avoid confusion.

---

## Execution Plan

### Phase 1: Small Crates (Warm-up) ✅ START HERE

**Target**: storage (5), social (7), cache (11) = **23 tools**

#### 1.1 Storage (5 tools)
- [ ] `storage_new` - Create filesystem storage
- [ ] `storage_compute_hash` - Hash data
- [ ] `storage_get_path` - Get storage path for hash
- [ ] `storage_verify_hash` - Verify hash matches data  
- [ ] `storage_media_type_as_str` - MediaType to string

#### 1.2 Social (7 tools)
- [ ] `social_hashmap_to_params` - Convert HashMap to params
- [ ] `social_convert_args_to_strings` - Convert args to strings
- [ ] `social_convert_security_error` - Convert security errors
- [ ] `social_new` - Create social media manager
- [ ] `social_with_cache` - Set cache on manager
- [ ] `social_platforms` - Get configured platforms
- [ ] `social_has_platform` - Check if platform configured

#### 1.3 Cache (11 tools)
- [ ] `cache_is_expired` - Check if cache entry expired
- [ ] `cache_time_remaining` - Time until expiration
- [ ] `cache_new` - Create new cache
- [ ] `cache_cleanup_expired` - Remove expired entries
- [ ] `cache_clear` - Clear all entries
- [ ] `cache_len` - Number of entries
- [ ] `cache_is_empty` - Check if cache empty
- [ ] `cache_evict_lru` - Evict least recently used

**Deliverable**: 3 new files, 23 wrappers, zero warnings

---

### Phase 2: Core Infrastructure (25 tools)

**Target**: botticelli_core = **25 tools**

Core tools (observability, config, token counting, budgets):
- [ ] `core_init_observability` - Initialize tracing/metrics
- [ ] `core_init_observability_with_config` - With custom config
- [ ] `core_shutdown_observability` - Cleanup observability
- [ ] `core_init_metrics` - Initialize metrics only
- [ ] `core_default_log_level` - Get default log level
- [ ] `core_get_tokenizer` - Get tokenizer for model
- [ ] `core_budget_builder` - Create BudgetConfig builder
- [ ] `core_budget_validate` - Validate budget config
- [ ] `core_budget_merge` - Merge two budgets
- [ ] `core_budget_apply_rpm/tpm/rpd` - Apply rate limits
- [ ] `core_budget_history_retention` - Get retention policy
- [ ] `core_generate_request_builder` - Create request builder
- [ ] `core_generate_response_builder` - Create response builder
- [ ] `core_message_builder` - Create message builder
- [ ] `core_stream_chunk_builder` - Create chunk builder
- [ ] `core_token_usage_new` - Create usage data
- [ ] `core_token_usage_calculate_cost` - Calculate cost
- [ ] `core_observability_config_builder` - Create config builder
- [ ] `core_observability_config_from_env` - Load from env
- [ ] `core_default_multiplier_tool` - Get default multiplier

**Deliverable**: 1 expanded file, 25 wrappers

---

### Phase 3: Rate Limit Completion (10 tools)

**Target**: botticelli_rate_limit = **10 remaining tools**

Currently has 9 tier query wrappers. Add the remaining 10:
- [ ] `rate_limit_new` - Create rate limiter
- [ ] `rate_limit_config` - Get config
- [ ] `rate_limit_reset_windows` - Reset time windows
- [ ] `rate_limit_can_afford` - Check if tokens available
- [ ] `rate_limit_consume` - Consume tokens
- [ ] `rate_limit_remaining` - Get remaining capacity
- [ ] `rate_limit_for_model` - Create limiter for model
- [ ] `rate_limit_from_tier` - Create from tier config
- [ ] `rate_limit_unlimited` - Create unlimited limiter
- [ ] `rate_limit_from_file` - Load config from file

**Deliverable**: 1 expanded file, 10 additional wrappers

---

### Phase 4: Security Completion (40 tools)

**Target**: botticelli_security = **40 remaining tools**

Currently has 1 validator wrapper. Add the remaining 40:

#### Permission Checking (4)
- [ ] `security_check_command` - Validate command permission
- [ ] `security_check_resource` - Validate resource access
- [ ] `security_check_user_protected` - Check user protection
- [ ] `security_check_role_protected` - Check role protection

#### Approval Workflow (10)
- [ ] `security_approval_request_new` - Create approval request
- [ ] `security_approval_request_is_expired` - Check expiration
- [ ] `security_approval_request_approve` - Approve request
- [ ] `security_approval_request_deny` - Deny request
- [ ] `security_command_config_new` - Create command config
- [ ] `security_command_config_set_requires_approval` - Set approval flag
- [ ] Plus 4 more approval-related tools

#### Security Context (15)
- [ ] `security_context_new` - Create security context
- [ ] `security_context_user_id` - Get user ID
- [ ] `security_context_roles` - Get roles
- [ ] `security_context_has_role` - Check role membership
- [ ] Plus 11 more context tools

#### Discord Integration (11)
- [ ] `security_discord_validator_new` - Create Discord validator
- [ ] Plus 10 more Discord-specific tools

**Deliverable**: 1 expanded file, 40 additional wrappers

---

### Phase 5: Database Operations (54 tools)

**Target**: botticelli_database = **54 tools**

Large crate with rich database functionality:

#### Connection Management (3)
- [ ] `database_establish_connection` - Create single connection
- [ ] `database_create_pool` - Create connection pool
- [ ] `database_create_pool_from_url` - Pool from URL

#### Content CRUD (8)
- [ ] `database_list_content` - List rows from table
- [ ] `database_get_content_by_id` - Get single row
- [ ] `database_insert_content` - Insert row
- [ ] `database_update_content_metadata` - Update metadata
- [ ] `database_update_review_status` - Update review status
- [ ] `database_delete_content` - Delete row
- [ ] `database_pull_and_delete` - Pull batch and delete
- [ ] `database_query_content` - Query with filters

#### Table Management (5)
- [ ] `database_create_content_table` - Create from template
- [ ] `database_create_inferred_table` - Create from schema
- [ ] `database_table_exists` - Check table existence
- [ ] `database_reflect_table_schema` - Get table schema
- [ ] `database_generate_create_table_sql` - Generate DDL

#### Schema Inference (5)
- [ ] `database_infer_schema` - Infer schema from JSON
- [ ] `database_infer_column_type` - Infer single column type
- [ ] `database_resolve_type_conflict` - Resolve type conflicts
- [ ] `database_generate_schema_prompt` - Create schema prompt
- [ ] `database_has_field` - Check field existence

#### Execution Tracking (8)
- [ ] `database_execution_to_new_row` - Convert execution to row
- [ ] `database_act_execution_to_new_row` - Convert act to row
- [ ] `database_input_to_new_row` - Convert input to row
- [ ] `database_rows_to_narrative_execution` - Convert rows to execution
- [ ] `database_rows_to_act_execution` - Convert rows to act
- [ ] `database_status_to_string` - Serialize status
- [ ] `database_string_to_status` - Deserialize status
- [ ] Plus 1 more execution tool

#### Formatting (3)
- [ ] `database_format_as_json` - Format rows as JSON
- [ ] `database_format_as_csv` - Format rows as CSV
- [ ] `database_format_as_markdown` - Format rows as Markdown

#### Repository/Query Executors (22+)
- Multiple `new()` constructors for different types
- Repository pattern methods
- Query executor methods
- Prompt assembly tools

**Deliverable**: 1 new file, 54 wrappers, proper feature gating

---

### Phase 6: Models (78 tools)

**Target**: botticelli_models = **78 tools**

Largest crate with multi-provider model operations:

#### Rate Limiting (10)
- [ ] `models_rate_limiter_new` - Create rate limiter
- [ ] `models_rate_limiter_acquire` - Acquire permit
- [ ] `models_rate_limiter_record` - Record usage
- [ ] `models_rate_limiter_current_count` - Get current count
- [ ] `models_rate_limiter_max_per_minute` - Get max rate
- [ ] Plus 5 more rate limit tools

#### Model Selection (8)
- [ ] `models_model_id_as_str` - Model ID to string
- [ ] `models_model_id_standard` - Get standard model
- [ ] `models_model_id_embedding` - Get embedding model
- [ ] `models_model_id_capabilities` - Get capabilities
- [ ] `models_model_range_allows` - Check if model in range
- [ ] `models_model_range_both` - Create range from bounds
- [ ] Plus 2 more selection tools

#### Content Type Handling (5)
- [ ] `models_input_as_text` - Extract text from input
- [ ] Plus 4 more input/output tools

#### Provider-Specific (55+)

**Ollama** (10 tools):
- [ ] `models_ollama_validate` - Validate model available
- [ ] `models_ollama_ensure_model` - Pull if missing
- [ ] Plus 8 more Ollama tools

**Anthropic** (12 tools):
- [ ] `models_anthropic_generate` - Call Claude API
- [ ] `models_anthropic_stream` - Stream responses
- [ ] Plus 10 more Anthropic tools

**OpenAI** (12 tools):
- [ ] `models_openai_generate` - Call GPT API
- [ ] `models_openai_stream` - Stream responses
- [ ] Plus 10 more OpenAI tools

**Gemini** (10 tools):
- [ ] `models_gemini_generate` - Call Gemini API
- [ ] `models_gemini_stream` - Stream responses
- [ ] Plus 8 more Gemini tools

**Groq** (8 tools):
- [ ] `models_groq_generate` - Call Groq API
- [ ] Plus 7 more Groq tools

**HuggingFace** (3 tools):
- [ ] `models_hf_generate` - Call HF API
- [ ] Plus 2 more HF tools

**Deliverable**: 1 new file, 78 wrappers, extensive feature gating (one per provider)

---

### Phase 7: Narrative Primitives (44 tools)

**Target**: botticelli_narrative = **44 tools**

Note: `narrative.rs` already has 7 high-level wrappers. Create separate `narrative_wrapper.rs` for primitives.

#### Narrative Construction (10)
- [ ] `narrative_new` - Create narrative
- [ ] `narrative_set_source_path` - Set source path
- [ ] `narrative_validate` - Validate narrative
- [ ] `narrative_ordered_acts` - Get acts in order
- [ ] `narrative_acts_mut` - Get mutable acts
- [ ] `narrative_has_composition_context` - Check context
- [ ] `narrative_name` - Get narrative name
- [ ] `narrative_get_narrative` - Get narrative data
- [ ] Plus 2 more construction tools

#### Act Management (12)
- [ ] `act_new` - Create act
- [ ] `act_validate` - Validate act
- [ ] `act_prompts` - Get prompts
- [ ] `act_add_prompt` - Add prompt
- [ ] Plus 8 more act tools

#### Scene Operations (10)
- [ ] `scene_new` - Create scene
- [ ] `scene_validate` - Validate scene
- [ ] `scene_content` - Get scene content
- [ ] Plus 7 more scene tools

#### Execution (8)
- [ ] `execution_assemble_act_prompts` - Assemble prompts
- [ ] Plus 7 more execution tools

#### State Management (4)
- [ ] `state_manager_new` - Create state manager
- [ ] Plus 3 more state tools

**Deliverable**: 1 new file, 44 wrappers, feature gating for database integration

---

## Export Chain Requirements

For each wrapper added, must update THREE files:

### 1. `rmcp_server/tools/mod.rs`
```rust
#[cfg(feature = "database")]
pub use database::{
    DatabaseCreatePoolParams,
    DatabaseCreatePoolResult,
    // ... all database DTOs
};
```

### 2. `rmcp_server/mod.rs`
```rust
#[cfg(feature = "database")]
pub use tools::{
    DatabaseCreatePoolParams,
    DatabaseCreatePoolResult,
    // ... all database DTOs
};
```

### 3. `lib.rs` (crate root)
```rust
#[cfg(feature = "database")]
pub use rmcp_server::{
    DatabaseCreatePoolParams,
    DatabaseCreatePoolResult,
    // ... all database DTOs
};
```

All three levels must have matching feature gates!

---

## Feature Gate Strategy

### Dependency Feature Requirements

| Crate              | Features Needed                          |
|--------------------|------------------------------------------|
| core               | None (always available)                  |
| storage            | None (always available)                  |
| cache              | None (always available)                  |
| social             | None (always available)                  |
| rate_limit         | None (always available)                  |
| security           | None (always available)                  |
| database           | `#[cfg(feature = "database")]`           |
| narrative          | `#[cfg(feature = "llm")]` for some tools |
| models             | Per-provider: `anthropic`, `openai`, `gemini`, `groq`, `ollama`, `huggingface` |

### Example Multi-Provider Gating
```rust
// Always available
impl BotticelliServer {
    #[tool]
    pub fn models_model_id_as_str(&self, ...) { }
}

// Ollama-specific
#[cfg(feature = "ollama")]
impl BotticelliServer {
    #[tool]
    pub fn models_ollama_generate(&self, ...) { }
}

// Anthropic-specific  
#[cfg(feature = "anthropic")]
impl BotticelliServer {
    #[tool]
    pub fn models_anthropic_generate(&self, ...) { }
}
```

---

## Verification Strategy

After each phase:

```bash
# Check warnings (must be zero)
just check botticelli_mcp

# Verify feature gate combinations
just check-features

# Count registered tools (should increase by phase count)
grep -r "#\[tool\]" crates/botticelli_mcp/src/rmcp_server/tools --include="*.rs" | wc -l

# Verify exports compile
cargo doc -p botticelli_mcp --no-deps
```

---

## Success Criteria

- [ ] All 325 tools accessible through BotticelliServer
- [ ] Zero compilation warnings in botticelli_mcp
- [ ] All feature gate combinations compile cleanly
- [ ] All tool DTOs exported at crate level with proper feature gates
- [ ] Documentation for all wrappers (copied from source)
- [ ] Full instrumentation (`#[tracing::instrument(skip(self))]` on all wrappers)
- [ ] Name collisions avoided via prefixing
- [ ] MCP server exposes complete "dictionary" of primitives to LLMs

---

## Estimated Effort

| Phase | Tools | Complexity | Estimated Time |
|-------|-------|------------|----------------|
| 1. Small crates | 23 | Low | 2-3 hours |
| 2. Core | 25 | Low | 2 hours |
| 3. Rate limit | 10 | Low | 1 hour |
| 4. Security | 40 | Medium | 3-4 hours |
| 5. Database | 54 | Medium | 4-5 hours |
| 6. Models | 78 | High | 6-8 hours |
| 7. Narrative | 44 | Medium | 3-4 hours |
| **TOTAL** | **274** | | **21-29 hours** |

**Note**: This is manual, careful work. Automation (sed/python) will backfire. Slow and steady wins.

---

## Next Actions

1. ✅ Review and approve this plan
2. ⏳ Start Phase 1: Small crates (storage, social, cache)
3. ⏳ Iterate through phases sequentially
4. ⏳ Update TOOL_EVERYTHING_PLAN.md with final counts
5. ⏳ Document complete orchestrator architecture

Ready to begin Phase 1?
