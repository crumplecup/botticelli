# Botticelli MCP Crate Architecture Audit - FINDINGS

## Executive Summary

**Current implementation:** `rmcp_server/` directory + supporting DTOs  
**Legacy implementation:** NONE - initial assessment was wrong  
**Status:** Single, cohesive rmcp-based codebase

## Architecture Breakdown

### ✅ CURRENT & KEEP: rmcp_server/ (Core Implementation)

```
rmcp_server/
├── server.rs              # BotticelliServer struct (main entry point)
├── handler.rs             # rmcp ServerHandler trait implementation
├── helpers.rs             # Shared helper functions
└── tools/                 # Tool implementations as impl BotticelliServer
    ├── core.rs
    ├── discord.rs
    ├── elicitation.rs
    ├── execution/         # Narrative execution tools
    ├── extraction_tools.rs
    ├── library.rs
    ├── models.rs
    ├── narrative.rs
    ├── rate_limit.rs
    ├── scene.rs
    ├── security.rs
    └── storage.rs
```

**Purpose:** rmcp-based MCP server implementation  
**Status:** ACTIVE - used by main.rs  
**Tooling:** Already using #[tool] macro extensively

### ✅ CURRENT & KEEP: Root-level DTO files

```
Root DTOs (parameter/result types):
├── conversation.rs         # ConversationSession, ConversationTurn
├── create_narrative.rs     # CreateNarrativeParams/Result
├── dialog_resource.rs      # DialogResource
├── discord_tools.rs        # Discord-specific types
├── echo.rs                 # EchoParams/Result
├── elicit_bool.rs          # ElicitBoolParams/Result
├── elicit_number.rs        # ElicitNumberParams/Result
├── elicit_select.rs        # ElicitSelectParams/Result
├── elicit_text.rs          # ElicitTextParams/Result
├── execution.rs            # Execute*Params/Result, Generate*
├── export_metrics.rs       # Metrics types
├── modify_narrative.rs     # ModifyNarrativeParams/Result
├── partial.rs              # PartialNarrative (elicitation state)
├── query_content.rs        # QueryContentParams/Result
├── save_narrative.rs       # SaveNarrativeParams/Result
├── scene.rs                # Scene CRUD types
├── server_info.rs          # ServerInfoResult
├── session_tools.rs        # Session management types
└── validate_narrative.rs   # Validation types
```

**Purpose:** DTOs for MCP tool parameters and results  
**Status:** ACTIVE - used by rmcp_server/ tools  
**Tooling:** These are data structures, not functions - NO #[tool] needed

### ✅ CURRENT & KEEP: tools/ (Shared Helpers)

```
tools/
├── narrative_validation_helpers.rs  # ✅ USED by rmcp_server
├── NarrativeHelper                  # ✅ USED by rmcp_server
└── ... (others need investigation)
```

**Purpose:** Shared business logic and utilities  
**Status:** PARTIALLY ACTIVE - some used by rmcp_server  
**Tooling:** Functions that can be called independently should get #[tool]

### ❓ UNCLEAR: resources/

```
resources/
├── content.rs
├── mod.rs
├── narrative.rs
├── registry.rs
└── resource_info.rs
```

**Need to check:** Does rmcp_server use ResourceRegistry?  
**Investigation:** Search for "ResourceRegistry" usage in rmcp_server/

## Key Findings

1. **No legacy code detected** - everything is part of current rmcp implementation
2. **Root files are DTOs** - data structures for tool params/results
3. **tools/ is shared helpers** - business logic used by rmcp_server
4. **rmcp_server/ is the implementation** - where BotticelliServer methods live
5. **main.rs confirms** - runs BotticelliServer (rmcp-based)

## Tooling Strategy

### What needs #[tool]?

**YES - Add #[tool]:**
- Functions in `tools/` that are callable operations
- Helper functions in `rmcp_server/helpers.rs` if independently useful
- Any standalone utility functions

**NO - Don't add #[tool]:**
- DTO struct definitions (they're data, not functions)
- Trait implementations on BotticelliServer (already registered as MCP tools)
- Builder patterns (derives handle this)

### Investigation Needed

1. **tools/ directory:** Which functions are:
   - Used only internally by rmcp_server? (helper - maybe tool)
   - Standalone operations? (definitely tool)
   - Deprecated? (remove)

2. **resources/ directory:**
   - Is this used by rmcp_server?
   - Is this a separate MCP resource feature?
   - Should this get tooled?

## Next Actions

1. Check resources/ usage in rmcp_server
2. Audit tools/ for toolable functions
3. Identify any truly unused/dead code
4. Create tooling plan for keeper functions
