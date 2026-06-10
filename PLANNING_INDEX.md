# Planning Document Index

This document provides a comprehensive index of all planning and strategy documents in the Botticelli workspace, organized by category with their last commit information.

## About This Index

This index tracks all planning documents in the workspace. When documents are completed or superseded, they are **deleted from the workspace** but remain accessible in git history. The index preserves entries for deleted documents with their last commit hash, allowing easy retrieval via `git show <commit>:<path>`.

**To view a deleted document**: `git show <commit-hash>:<document-path>`

---

## Active Planning Documents

### rmcp + Elicitation Migration (Active)
- **RMCP_ELICITATION_MIGRATION.md** - `current` (2026-06-10) **🚧 IN PROGRESS**
  - Full migration from pmcp → rmcp + elicitation framework across botticelli_mcp
  - Replaces pmcp, mcp-server, mcp-spec with rmcp + elicitation crate
  - Propagates `#[derive(Elicit, JsonSchema, Serialize, Deserialize)]` workspace-wide
  - Collapses 8-tool session stack to single `create_narrative` tool using `PartialNarrative::elicit()`
  - Single binary (stdio + HTTP) with clap subcommands
  - 10-phase checklist, tracked commit-by-commit on dev branch

### MCP Server & Testing
- **TUI_MCP_CLIENT_FIX.md** - `current` (2025-12-24) **✅ FIXED**
  - Applied MCP server testing lessons to fix TUI client startup issues
  - Problem: Hardcoded wrong port (3030 vs 8080), no server verification, poor observability
  - Solution: Environment-based config, early verification, structured logging with emoji markers
  - Key improvement: Logs show exact endpoint, helpful error messages guide user
  - Status: ✅ Fixed and documented

- **server_lifecycle_test.rs** - `current` (2025-12-24) **✅ IMPLEMENTED**
  - Comprehensive MCP server lifecycle tests with full observability
  - Tests: startup, initialization, tool listing, error handling, concurrent requests
  - Discovered: Server uses `/sse` endpoint (SSE protocol), not plain JSON-RPC
  - Added: Tracing throughout tests, server logs port/endpoint explicitly
  - Status: ✅ 5/5 tests passing (startup test simplified to just spawn verification)

### Core Architecture & Refactoring

- **ELICITATION_MCP_INTEGRATION_PLAN.md** - `current` (2025-12-28) **🎯 INTEGRATION PLAN**
  - Comprehensive plan for integrating elicitation crate with botticelli_mcp
  - Add primitive elicitation tools (elicit_text, elicit_select, etc.) to MCP server
  - DialogResource wrapper for sharing dialog across tools
  - InProcTransport for in-process MCP server
  - Status: ✅ Phase 1 complete - primitive tools implemented
  - Status: ✅ Phase 2 complete - InProcTransport implemented and tested
  - Status: ✅ Phase 3 complete - integration tested, derive macros working
  - Status: ✅ Phase 4 complete - MetadataElicitor refactored, 98.6% code reduction achieved
  - Status: ✅ Infrastructure extracted - shared helper ready for all elicitors

- **CODE_REDUCTION_ANALYSIS.md** - `current` (2025-12-28) **📊 METRICS & RESULTS**
  - Detailed analysis of code reduction from MetadataElicitor refactor
  - Original: 70 lines of manual dialog calls
  - Refactored: 1 line using NarrativeMetadata::elicit()
  - Reduction: 98.6% for core elicitation logic
  - Documents pattern benefits: type safety, maintainability, testability

- **ELICITOR_REFACTOR_GUIDE.md** - `current` (2025-12-28) **📚 IMPLEMENTATION GUIDE**
  - Comprehensive guide for applying paradigm-based refactoring to all elicitors
  - Step-by-step refactoring process with examples
  - Pattern reference for Survey, Select, and Affirm paradigms
  - Migration strategy for ActElicitor, InputElicitor, CarouselElicitor, ValidationElicitor
  - Expected reductions: 60-70% code reduction per elicitor

- **ELICITATION_PARADIGM_REFACTOR.md** - `current` (2025-12-28) **🎯 ARCHITECTURAL VISION**
  - Identifies fundamental design flaw in ElicitationDialog abstraction
  - Paradigm traits (Select/Affirm/Survey) ARE the interaction model
  - Current approach mixes type concerns (String, i64) with interaction patterns
  - Refactor strategy: Use elicitation crate's paradigm system directly
  - Status: 📋 Phase 1 complete - domain types defined

- **ELICITATION_AUDIT.md** - `current` (2025-12-28) **📊 USAGE ANALYSIS**
  - Comprehensive audit of 75 dialog method calls across codebase
  - Categorization: 6 Select, 31 Affirm, 37 Survey-eligible calls
  - High-impact refactors identified (80% code reduction potential)
  - Domain types mapped to paradigms
  - Status: ✅ Audit complete, types created

- **TUI_LAG_FIX_SUMMARY.md** - `current` (2025-12-23) **✅ FIXED**
  - Documents keyboard input lag fix and minimal event loop architecture
  - Problem: 1-3 second delays between keystrokes
  - Root cause: Event loop doing too much (blocking UI thread)
  - Solution: Minimal event loop - ONLY keyboard + rendering, everything else in background
  - Current status: ✅ Keyboard responsive, ⏳ HTTP client integration pending
  - Architecture: UI thread (hot loop) + Background tasks (async) + Channels (communication)

- **TUI_LAG_FIX_IMPLEMENTATION.md** - `current` (2025-12-22) **✅ IMPLEMENTED**
  - Fixes severe keyboard input lag by decoupling rendering from input
  - Root cause: terminal.draw() synchronous I/O blocking event loop
  - Solution: Dirty flag + render at fixed 60fps, input updates state instantly
  - Status: 🧪 Ready for user testing

- **TUI_EVENT_LOOP_REFACTOR.md** - `current` (2025-12-22) **📋 ARCHITECTURAL ANALYSIS**
  - Analysis document explaining the root cause of keyboard lag
  - Documents why tests didn't catch the issue (TestBackend vs real terminal I/O)
  - Architecture options evaluated, decoupled rendering selected
  - Status: 📚 Reference documentation

- **TUI_ARCHITECTURE_CONSOLIDATION.md** - `current` (2025-12-22) **🚨 CRITICAL REFACTOR**
  - Eliminates parallel TUI implementations (botticelli_tui vs botticelli_chat)
  - Establishes trait-based architecture with clear separation of concerns
  - Interface traits → Chat business logic → TUI presentation layer
  - 5-phase plan to consolidate duplicate code and fix tool execution
  - Status: 📋 Strategic design - Phase 2 critical (tool loop broken)

- **TRAIT_INTERFACE_UNIFIED_DESIGN.md** - `current` (2025-12-19) **🎯 ARCHITECTURAL VISION**
  - Clean, unified trait design achieving full provider interoperability
  - Vision: Traits define ALL behaviors, not request fields
  - Tool calling as first-class trait method (`generate_with_tools`)
  - Composable traits: BotticelliDriver, ToolCalling, Streaming, Vision, Embeddings
  - 5-phase migration plan
  - Status: 📋 Strategic design - ready for implementation

- **CHAT_MCP_IMPLEMENTATION_PLAN.md** - `current` (2025-12-20)
  - Strategic plan for Chat as MCP host with multi-provider LLM support
  - Phases 1-3 complete: LLM client, MCP integration, tool execution flow
  - Uses existing ChatSession/ModelSelector fallback architecture
  - Status: 🚧 Active - Phase 4 fallback integration in progress

### Documentation & Guides

- **AI_NARRATIVE_TOML_GUIDE.md** - `ee7d2d3` (2025-11-28)
  - Comprehensive guide for writing narrative TOML files
- **NARRATIVE_TOML_SPEC.md** - `ee7d2d3` (2025-11-28)
  - Technical specification for narrative TOML format
- **CLEAN_SAMPLING_USAGE_GUIDE.md** - `current` (2025-12-14)
  - Usage guide for refactored sampling architecture
- **NARRATIVE_GENERATION_USAGE.md** - `current` (2025-12-08)
  - Usage examples for LLM-driven narrative generation
- **MIGRATION_GUIDE.md** - `current`
  - Guide for migrating between versions

### Setup & Configuration

- **CLAUDE.md** - `current` - Project development guidelines
- **GEMINI.md** - `8937634` (2025-11-17) - Google Gemini API setup
- **POSTGRES.md** - `8937634` (2025-11-17) - PostgreSQL setup
- **DISCORD_SETUP.md** - `8937634` (2025-11-17) - Discord bot setup
- **SOCIAL_MEDIA.md** - `8937634` (2025-11-17) - Social media integration
- **MCP.md** - `current` - MCP user guide and reference

### Testing

- **TESTING_PATTERNS.md** - `718bf35` (2025-11-22) - Testing patterns and best practices
- **DISCORD_COMMAND_TESTING_STRATEGY.md** - `6206b41` (2025-11-22) - Discord command testing

---

## Archived Planning Documents (Dec 2025 Cleanup)

*These documents have been deleted from the workspace but remain in git history.*

### Chat & Configuration (Superseded by CHAT_MCP_IMPLEMENTATION_STRATEGY.md)
- **CHAT_AUTO_START_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_COMPLETE_INTEGRATION.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_COMPLETE_SESSION.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_DEPLOYMENT_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_DEPLOYMENT_INTEGRATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_ENV_INTEGRATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_INTEGRATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_CONFIG_SUMMARY.md** - `f9dafd6` (2025-12-20)
- **CHAT_IMPLEMENTATION_TRACKER.md** - `f9dafd6` (2025-12-20)
- **CHAT_LOCAL_CONFIG_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **CHAT_MCP_ARCHITECTURE_STATUS.md** - `f9dafd6` (2025-12-20)
- **CHAT_MCP_HOST_IMPLEMENTATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **CHAT_MCP_INTEGRATION_FINDINGS.md** - `f9dafd6` (2025-12-20)
- **CHAT_SYSTEM_COMPLETE.md** - `f9dafd6` (2025-12-20)

### LLM Integration Plans (Completed - code in codebase)
- **ANTHROPIC_REQWEST_PLAN.md** - `f9dafd6` (2025-12-20)
- **GROQ_INTEGRATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **HUGGINGFACE_INTEGRATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **OLLAMA_INTEGRATION_PLAN.md** - `f9dafd6` (2025-12-20)

### Narrative Sampling (Completed - superseded by clean architecture)
- **CLEAN_SAMPLING_ARCHITECTURE_DIAGRAM.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_ELICITATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_GENERATION_MCP_PLAN.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_SAMPLING_ARCHITECTURE_ANALYSIS.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_SAMPLING_CLEAN_ARCHITECTURE.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_SAMPLING_IMPLEMENTATION.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_SAMPLING_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **ELICITATION_SYSTEM_COMPLETE.md** - `f9dafd6` (2025-12-20)

### MCP Client Implementation (Completed)
- **MCP_CLIENT_CODE_STANDARDS_AUDIT.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_DESIGN.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_IMPLEMENTATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_INTEGRATION_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_PHASE_B_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_REVISED_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_TODO_COMPLETION_PLAN.md** - `f9dafd6` (2025-12-20)
- **MCP_CLIENT_UNIFIED_ARCHITECTURE_PLAN.md** - `f9dafd6` (2025-12-20)
- **MCP_INTEGRATION_COMPLETION_PLAN.md** - `f9dafd6` (2025-12-20)
- **MCP_TYPE_SYSTEM_STRATEGY.md** - `f9dafd6` (2025-12-20)

### PMCP Migration (Completed)
- **PMCP_COMPLETE_MIGRATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **PMCP_FEATURE_RESTORATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **PMCP_FEATURE_RESTORATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **PMCP_MIGRATION_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **PMCP_SELF_DRIVING_IMPLEMENTATION.md** - `f9dafd6` (2025-12-20)
- **SELF_DRIVING_INTEGRATION_PLAN.md** - `f9dafd6` (2025-12-20)

### TUI Implementation (Completed)
- **TUI_ARCHITECTURE_ANALYSIS.md** - `f9dafd6` (2025-12-20)
- **TUI_ASYNC_UI_UPDATES_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_BINARY_AND_USABILITY_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_COMPOSABLE_ARCHITECTURE_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_COMPREHENSIVE_DESIGN_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **TUI_JUSTFILE_RECIPE_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_MCP_INITIALIZATION_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_MCP_PHASE8_PROGRESS.md** - `f9dafd6` (2025-12-20)
- **TUI_MCP_PHASE8_TASK8_1_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_MCP_SHOWCASE_SUMMARY.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_0_TASK_1_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_0_TASKS_2_3_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_1_TASK_1_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_1_TASK_2_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_1_TASK_3_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_1_TASK_4_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_2_TASK_1_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_2_TASK_2_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_PHASE_2_TASK_3_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TUI_REDESIGN_PLAN.md** - `f9dafd6` (2025-12-20)

### Architecture & Refactoring (Superseded by TRAIT_INTERFACE_UNIFIED_DESIGN.md)
- **ARCHITECTURAL_GAPS_RESOLUTION_PLAN.md** - `f9dafd6` (2025-12-20)
- **ARCHITECTURAL_GAPS_RESOLUTION_PLAN_V2.md** - `f9dafd6` (2025-12-20)
- **TRAIT_INTERFACE_IMPROVEMENT_PLAN.md** - `f9dafd6` (2025-12-20)

### Fallback & Model Selection (Implementation in progress)
- **LLM_FALLBACK_GAP_ANALYSIS.md** - `f9dafd6` (2025-12-20)
- **LLM_FALLBACK_IMPLEMENTATION_PLAN.md** - `f9dafd6` (2025-12-20)
- **MODEL_SELECTION_STRATEGY.md** - `f9dafd6` (2025-12-20)

### Token Counting & Validation (Completed)
- **TOKEN_COUNTING_IMPLEMENTATION.md** - `f9dafd6` (2025-12-20)
- **TOKEN_COUNTING_TRAIT_ANALYSIS.md** - `f9dafd6` (2025-12-20)
- **NARRATIVE_VALIDATOR_DESIGN.md** - `f9dafd6` (2025-12-20)

### User Interaction (Deferred)
- **INTERACTION_INTERFACE_IMPLEMENTATION.md** - `f9dafd6` (2025-12-20)
- **USER_INTERACTION_INTERFACE_PLAN.md** - `f9dafd6` (2025-12-20)
- **USER_INTERACTION_INTERFACE_PLAN_CRITIQUE.md** - `f9dafd6` (2025-12-20)

### Misc Complete Work
- **DEMO_ACTOR_DESIGN.md** - `f9dafd6` (2025-12-20)
- **DISCORD_API_COVERAGE_ANALYSIS.md** - `f9dafd6` (2025-12-20)
- **ERROR_HANDLING_AUDIT.md** - `f9dafd6` (2025-12-20)
- **OBSERVABILITY_METRICS_JAEGER_ISSUE.md** - `f9dafd6` (2025-12-20)
- **OBSERVABILITY_SUMMARY.md** - `f9dafd6` (2025-12-20)
- **PLANNING_TRACKER.md** - `f9dafd6` (2025-12-20)
- **SESSION_2024_12_04_COMPLETE.md** - `f9dafd6` (2025-12-20)
- **TEST_COVERAGE_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **TEST_PERFORMANCE_STRATEGY.md** - `f9dafd6` (2025-12-20)
- **TODO_AUDIT.md** - `f9dafd6` (2025-12-20)

---

## Summary

- **Active Documents**: 15 (architecture, guides, setup, testing)
- **Archived Documents**: 96 (completed work preserved in git history)
- **Total Tracked**: 111 documents

**Last Updated**: 2025-12-20
**Major Cleanup**: Archived 96 completed planning documents
**Active Focus**: Trait-based architecture refactor and Chat MCP host implementation
