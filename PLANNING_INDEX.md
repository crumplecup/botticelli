# Planning Document Index

This document provides a comprehensive index of all planning and strategy documents in the Botticelli workspace, organized by category with their last commit information.

## About This Index

This index tracks all planning documents in the workspace. When documents are completed or superseded, they are **deleted from the workspace** but remain accessible in git history. The index preserves entries for deleted documents with their last commit hash, allowing easy retrieval via `git show <commit>:<path>`.

**To view a deleted document**: `git show <commit-hash>:<document-path>`

## Active Planning Documents

### Narrative System
- **AI_NARRATIVE_TOML_GUIDE.md** - `ee7d2d3` (2025-11-28)
  - Comprehensive guide for writing narrative TOML files
- **NARRATIVE_TOML_SPEC.md** - `ee7d2d3` (2025-11-28)
  - Technical specification for narrative TOML format

### Discord Integration
- **crates/botticelli_narrative/narratives/discord/BOTTICELLI_CONTEXT.md** - `81c7eab` (2025-11-24)
  - Context document for LLM-generated Discord content
- **DISCORD_COMMAND_TESTING_STRATEGY.md** - `6206b41` (2025-11-22)
  - Testing strategy for Discord bot commands
- **DISCORD_API_COVERAGE_ANALYSIS.md** - `0f817c4` (2025-11-21)
  - Analysis of Discord API coverage and command implementation

### Actor Architecture
- **crates/botticelli_actor/ACTOR_GUIDE.md** - `4df9c44` (2025-11-23)
  - Guide for using the actor system

### Infrastructure & Configuration
- **CHAT_CONFIG_SYSTEM.md** - `current` (2025-12-08)
  - Flexible configuration system for local development and containerized deployment
  - Environment modes (local/container), config precedence, comprehensive tests
  - Status: ✅ COMPLETE - Infrastructure ready, 4 phases remaining
- **CHAT_CONFIG_COMPLETE.md** - `current` (2025-12-08)
  - Complete implementation summary with architecture and usage
  - Status: ✅ Reference document
- **CHAT_CONFIG_SUMMARY.md** - `current` (2025-12-08)
  - Quick reference and usage guide
  - Status: ✅ Reference document
- **CHAT_CONFIG_INTEGRATION_COMPLETE.md** - `current` (2025-12-08)
  - Complete configuration integration with service container and health checks
  - Status: ✅ Reference document
- **CHAT_AUTO_START_COMPLETE.md** - `current` (2025-12-08)
  - Automatic MCP server startup for Local mode development
  - One-command launch with dependency management
  - Status: ✅ COMPLETE - Feature ready
- **CHAT_LOCAL_CONFIG_COMPLETE.md** - `current` (2025-12-09)
  - Complete local development setup with auto-startup and integration tests
  - Phase A (Database) and Phase B (MCP Integration) complete
  - Comprehensive testing with 3/3 integration tests passing
  - Status: ✅ COMPLETE - Phases A & B ready
- **CHAT_IMPLEMENTATION_TRACKER.md** - `current` (2025-12-08)
  - Detailed task tracker for remaining 4 phases (A-E)
  - Time estimates, dependencies, success criteria
  - Status: 🚧 Active tracking document
- **BUDGET_MULTIPLIER_DESIGN.md** - `e0a7ebc` (2025-11-24)
  - Design for API rate limit budget multipliers
- **OPENTELEMETRY_INTEGRATION_ISSUES.md** - `current` (2025-11-30)
  - OpenTelemetry integration analysis and resolution strategy
- **OBSERVABILITY_SETUP.md** - `current` (2025-11-30)
  - Complete guide for setting up observability with Jaeger and Podman
- **QUICK_START_OBSERVABILITY.md** - `current` (2025-11-30)
  - Quick start guide for running full observability stack
- **OBSERVABILITY_SUMMARY.md** - `current` (2025-11-30)
  - Summary of observability stack implementation and setup
- **OBSERVABILITY_METRICS_JAEGER_ISSUE.md** - `current` (2025-11-30)
  - Explanation of why Jaeger alone doesn't support metrics
- **NARRATIVE_COMPOSITION_LOADING_ISSUE.md** - `current` (2025-11-30)
  - Analysis and fix strategy for narrative composition loading
- **PODMAN_CONTAINERIZATION.md** - `current` (2025-11-30)
  - Strategy for containerizing bot-server with Podman
- **NARRATIVE_VALIDATOR_DESIGN.md** - `1118d0c` (2025-12-05)
  - Comprehensive design for narrative TOML validation with actionable error messages
  - Phases 1-5 complete: validator infrastructure, CLI integration, MCP tools, multi-LLM execution
- **NARRATIVE_GENERATION_MCP_PLAN.md** - `dba7a8f` (2025-12-08, Phase 1 complete)
  - Strategy for LLM-driven narrative generation via MCP tools
  - Stateless transformation pattern: create, modify, save narratives
  - Phase 1 complete with tests and documentation
  - See NARRATIVE_GENERATION_USAGE.md for examples

### Testing & Quality
- **TESTING_PATTERNS.md** - `718bf35` (2025-11-22)
  - Testing patterns and best practices
- **crates/botticelli_social/DISCORD_TEST_COVERAGE.md** - `54c9429` (2025-11-22)
  - Discord command test coverage tracking
- **crates/botticelli_social/DISCORD_WRITE_OPERATIONS_TESTING.md** - `6206b41` (2025-11-22)
  - Testing strategy for Discord write operations

### Development Guidelines
- **CLAUDE.md** - `805ffdc` (2025-11-23)
  - Project development guidelines and conventions

## Setup & Configuration Guides

### Backend Setup
- **GEMINI.md** - `8937634` (2025-11-17)
  - Google Gemini API setup and configuration
- **GEMINI_STREAMING.md** - `8937634` (2025-11-17)
  - Streaming support for Gemini API
- **crates/botticelli_server/MISTRALRS_SETUP.md** - `4d1d2a2` (2025-11-19)
  - Local LLM setup with MistralRS
- **crates/botticelli_server/SERVER_GUIDE.md** - `4d1d2a2` (2025-11-19)
  - Server configuration and usage guide
- **crates/botticelli_server/USAGE_GUIDE.md** - `e48b0c1` (2025-11-19)
  - User guide for server operations

### Database & Storage
- **POSTGRES.md** - `8937634` (2025-11-17)
  - PostgreSQL setup and configuration
- **DATABASE_SYNC_STRATEGY.md** - `current` (2025-11-30)
  - Strategy for synchronizing databases between host and containers
- **DISCORD_SCHEMA.md** - `8937634` (2025-11-17)
  - Discord data schema definitions
- **MEDIA_STORAGE.md** - `8937634` (2025-11-17)
  - Media file storage strategy

### Social Media Integration
- **DISCORD_SETUP.md** - `8937634` (2025-11-17)
  - Discord bot setup and permissions
- **SOCIAL_MEDIA.md** - `8937634` (2025-11-17)
  - Social media platform integration overview
- **USAGE_TIERS.md** - `8937634` (2025-11-17)
  - API usage tier management

## Crate-Specific Documentation

- **crates/botticelli/README.md** - `7b606af` (2025-11-17) - Main crate documentation
- **crates/botticelli_actor/README.md** - `d8ccea3` (2025-11-23) - Actor system crate
- **crates/botticelli_core/README.md** - `7b606af` (2025-11-17) - Core types and utilities
- **crates/botticelli_database/README.md** - `7b606af` (2025-11-17) - Database layer
- **crates/botticelli_error/README.md** - `7b606af` (2025-11-17) - Error handling
- **crates/botticelli_interface/README.md** - `7b606af` (2025-11-17) - Trait definitions
- **crates/botticelli_models/README.md** - `7b606af` (2025-11-17) - Data models
- **crates/botticelli_narrative/README.md** - `5dd1a78` (2025-11-19) - Narrative execution engine
- **crates/botticelli_rate_limit/README.md** - `7b606af` (2025-11-17) - Rate limiting
- **crates/botticelli_social/README.md** - `7b606af` (2025-11-17) - Social media APIs
- **crates/botticelli_storage/README.md** - `7b606af` (2025-11-17) - File storage
- **crates/botticelli_tui/README.md** - `7b606af` (2025-11-17) - Terminal UI

## Narrative Examples & Guides

- **crates/botticelli_narrative/narratives/README.md** - `72eda8e` (2025-11-20)
  - Overview of narrative examples
- **crates/botticelli_narrative/narratives/NARRATIVES.md** - `5dd1a78` (2025-11-19)
  - Narrative system documentation
- **crates/botticelli_narrative/narratives/discord/README.md** - `72eda8e` (2025-11-20)
  - Discord-specific narrative examples

## Archived Planning Documents

*These documents have been deleted from the workspace but remain in git history. View with `git show <commit>:<path>`*

### Bot Server & Deployment (Completed)
- **BOT_SERVER_DEPLOYMENT_PLAN.md** - `b9bfa37` (2025-11-28)
  - Comprehensive plan for deploying generation, curation, and posting bots
- **BOT_SERVER_NEXT_STEPS.md** - `3d7386f` (2025-11-28)
  - Next steps for bot server implementation and testing
- **ACTOR_INTEGRATION_PROGRESS.md** - `403803c` (2025-11-28)
  - Progress tracker for actor-based architecture integration
- **ACTOR_SERVER_STRATEGY.md** - `0f5fada` (2025-11-28)
  - Strategy for actor-based server architecture
- **ACTOR_ARCHITECTURE.md** - `4df9c44` (2025-11-23)
  - Overall actor system architecture
- **ACTOR_SERVER_OBSERVABILITY.md** - `b9310b6` (2025-11-27)
  - Observability and monitoring for actor server
- **DISCORD_CONTENT_ACTOR_PLAN.md** - `4df9c44` (2025-11-23)
  - Plan for Discord content generation actors

### JSON & Content Processing (Completed)
- **JSON_EXTRACTION_IMPLEMENTATION.md** - `8d9f720` (2025-11-28)
  - Implementation details for JSON extraction from LLM outputs
- **JSON_EXTRACTION_STRATEGY.md** - `365edc7` (2025-11-28)
  - Strategy for reliable JSON extraction and validation
- **JSON_SCHEMA_MISMATCH_STRATEGY.md** - `087c544` (2025-11-27)
  - Handling schema mismatches between JSON and database tables
- **crates/botticelli_narrative/narratives/discord/JSON_COMPLIANCE_WORKFLOW.md** - `8d9f720` (2025-11-28)
  - Workflow for ensuring JSON compliance in narrative outputs

### Narrative System (Completed)
- **NARRATIVE_COMPOSITION_IMPLEMENTATION.md** - `08594ed` (2025-11-24)
  - Implementation of narrative composition and reuse
- **NARRATIVE_COMPOSITION_ISSUE.md** - `750e86c` (2025-11-24)
  - Issues and solutions for narrative composition
- **MULTI_NARRATIVE_DESIGN.md** - `81c7eab` (2025-11-24)
  - Design for multi-narrative TOML files
- **MULTI_NARRATIVE_IMPLEMENTATION.md** - `81c7eab` (2025-11-24)
  - Implementation of multi-narrative support
- **CAROUSEL_COMPOSITION_STRATEGY.md** - `0f5fada` (2025-11-28)
  - Strategy for carousel-based narrative execution

### Discord Integration (Completed)
- **crates/botticelli_narrative/narratives/discord/DISCORD_POSTING_STRATEGY.md** - `e1cea3a` (2025-11-27)
  - Strategy for automated Discord content posting
- **crates/botticelli_narrative/narratives/discord/ACTOR_INTEGRATION_STRATEGY.md** - `72d8d10` (2025-11-27)
  - Actor integration for Discord content workflows

### Infrastructure & Configuration (Completed)
- **CONNECTION_POOL_INTEGRATION.md** - `86f071e` (2025-11-23)
  - Database connection pool integration strategy
- **CONVERSATION_HISTORY_RETENTION_PLAN.md** - `4bdd0b1` (2025-11-27)
  - Plan for conversation history management
- **OPENTELEMETRY_INTEGRATION_PLAN.md** - `68ba55c` (2025-11-30)
  - Comprehensive OpenTelemetry observability integration plan (COMPLETED)
- **BOT_SERVER_OBSERVABILITY_STRATEGY.md** - `68ba55c` (2025-11-30)
  - Original observability strategy (superseded by OpenTelemetry implementation)

### State Management (Completed)
- **BOT_COMMAND_STATE_INTEGRATION.md** - `2f6af15` (2025-11-22)
  - Integration of bot command state tracking
- **BOT_OUTPUT_STATE_CAPTURE.md** - `5c93418` (2025-11-22)
  - Capturing and managing bot output state

### Development Sessions (Completed)
- **SESSION_SUMMARY.md** - `4df9c44` (2025-11-23)
  - Summary of development sessions and progress

### Bug Reports & Investigations (Resolved)
- **crates/botticelli_narrative/narratives/discord/BUG_TABLE_INPUT_RESPONSE_LOSS.md** - `4bdd0b1` (2025-11-27)
  - Bug report and investigation (resolved)

### Refactoring Notes (Completed)
- **crates/botticelli_core/REFACTOR.md** - `c0a1603` (2025-11-19)
  - Core crate refactoring notes
- **crates/botticelli_narrative/narratives/discord/DISCORD_COMMUNITY_SERVER_PLAN.md** - `7b2772f` (2025-11-20)
  - Early Discord community server planning

## Document Categories Summary

- **Active Planning**: 12 documents (added PODMAN_CONTAINERIZATION.md)
- **Setup & Configuration**: 13 documents (added DATABASE_SYNC_STRATEGY.md)
- **Crate Documentation**: 13 documents
- **Narrative Guides**: 3 documents
- **Archived**: 28 documents

**Total**: 69 markdown documents tracked (41 active, 28 archived)

---

*Last Updated: 2025-11-30*
*OpenTelemetry Integration: COMPLETE (Phases 1-6)*
*Generated automatically - see git log for detailed history*

## MCP Integration (Dec 2024)

### Active Documents
- [MCP.md](./MCP.md) - User guide and reference
- **MCP_INTEGRATION_COMPLETION_PLAN.md** - `current` (2025-12-05)
  - Plan for completing Phases 6-8: Discord tools, observability, and workflows
- **TOKEN_COUNTING_IMPLEMENTATION.md** - `current` (2025-12-05)
  - Implementation plan for TokenCounting trait across all LLM backends
  - Phase 1 complete: trait implemented for Gemini, Claude, Groq, Ollama, HuggingFace

### Archived Documents
- **MCP_INTEGRATION_STRATEGIC_PLAN.md** - `1118d0c` (2025-12-05)
  - Complete 5-phase strategy (Phase 1 MVP complete)
- **MCP_COPILOT_CLI_SETUP.md** - `1118d0c` (2025-12-05)  
  - Setup guide for MCP with GitHub Copilot CLI
- **MCP_MULTI_LLM_IMPLEMENTATION.md** - `1118d0c` (2025-12-05)
  - Multi-backend LLM implementation strategy for MCP server

---

## Session Complete: December 4-5, 2024

**Document:** [SESSION_2024_12_04_COMPLETE.md](./SESSION_2024_12_04_COMPLETE.md)

### Summary
- ✅ HuggingFace integration complete
- ✅ Groq integration complete  
- ✅ MCP Phase 1 MVP complete
- ✅ 8 production commits
- ✅ ~2,700 lines (code + docs)
- ✅ All tests passing

### Next Session Actions
1. Restart Copilot CLI
2. Test MCP integration with natural language queries
3. Validate Phase 1 works end-to-end
4. Document findings
5. Plan Phase 2 (if validated)

**Status:** Ready for testing and validation

- **USER_INTERACTION_INTERFACE_PLAN.md** - `draft` (2025-12-08)
  - Unified trait-based interface for user-Botticelli interactions
  - Multi-platform architecture (TUI, web, mobile)
  - Conversational workflows for narrative creation, bot assignment, scheduling
  - Ready for Phase 1 implementation

- **INTERACTION_INTERFACE_IMPLEMENTATION.md** - `active` (2025-12-08)
  - Reformed implementation plan based on critique
  - 10 clear, actionable steps with code templates
  - CLAUDE.md compliant design
  - 6-hour timeline estimate
  - Ready for Phase 1 implementation

- **DEMO_ACTOR_DESIGN.md** - `current` (2025-12-09)
  - Demo actor system design for automated interaction testing
  - Exercises all MCP tools through scripted prompts
  - Phase 1 complete: core scenarios and executor
  - Status: ✅ Core implementation complete, TUI integration pending

- **TUI_REDESIGN_PLAN.md** - `current` (2025-12-14)
  - Comprehensive TUI redesign for multi-view interface
  - Clean architecture with ratatui: chat, narrative browser, editor views
  - All 7 phases complete: foundation, state, UI components, events, integration, testing, documentation
  - Status: ✅ COMPLETE - 665 lines, zero dead code, all checks pass

- **NARRATIVE_SAMPLING_CLEAN_ARCHITECTURE.md** - `current` (2025-12-14)
  - Clean architecture refactor for narrative sampling and LLM providers
  - Phases 1-4 complete: ConversationSession, LlmSampler trait, SamplingCoordinator, testing
  - Fixed Gemini stop_reason implementation (SAFETY, STOP_SEQUENCE, etc.)
  - Status: ✅ COMPLETE - All providers refactored, tests passing

- **CLEAN_SAMPLING_USAGE_GUIDE.md** - `current` (2025-12-14)
  - Usage guide for refactored sampling architecture
  - Examples for basic generation, tool calling, and narrative workflows
  - Component documentation: LlmProvider, ConversationSession, LlmSampler, SamplingCoordinator
  - Status: ✅ Reference document

- **MCP_TYPE_SYSTEM_STRATEGY.md** - `current` (2025-12-14)
  - Strategy for implementing MCP specification in Rust type system
  - Compile-time verification of protocol compliance
  - Phased approach: core types, validation, integration
  - Status: 📋 Planning document for future implementation

- **PMCP_MIGRATION_STRATEGY.md** - `current` (2025-12-15)
  - Comprehensive migration strategy from mcp-server/mcp-spec to pmcp SDK
  - 4 phases: Server Foundation, Client Migration, Enhanced Features, Testing & Documentation
  - 16x performance improvement, 50x memory reduction targets
  - Battle-tested SDK with OAuth, batching, middleware, WebSocket support
  - 13-day timeline with measurable success criteria
  - Status: 📋 Planning document - ready for implementation

- **MCP_CLIENT_INTEGRATION_STRATEGY.md** - `current` (2025-12-15)
  - Strategy for integrating pmcp Client to connect to external MCP servers
  - Leverages existing pmcp SDK for ecosystem access (filesystem, git, search, etc.)
  - 4 phases: Client Wrapper, Narrative Integration, Enhanced Features, Testing
  - Enables Botticelli as complete platform: MCP server + client + orchestrator
  - 10-day timeline with minimal risk
  - Status: 📋 Planning document - ready for implementation

- **MCP_CLIENT_REVISED_STRATEGY.md** - `current` (2025-12-15)
  - Revised strategy after auditing existing botticelli_mcp_client crate
  - Phase A: Self-driving (existing) - tools for LLM self-discovery
  - Phase B: External servers - connect to ecosystem MCP servers ✅ COMPLETE
  - Phase C: Narrative integration - TOML config for external servers
  - Proper architecture understanding and integration approach
  - Status: ✅ Phase B complete, Phase C ready

- **MCP_CLIENT_PHASE_B_COMPLETE.md** - `current` (2025-12-15)
  - Complete implementation summary for Phase B (External Server Integration)
  - Custom ChildProcessTransport implementing pmcp::Transport
  - Full MCP protocol: initialize, tools/list, tools/call
  - Integration tests with real external servers
  - Ecosystem access: filesystem, git, search, cloud, dev tools
  - Strategic positioning: MCP server + client + orchestrator + multi-model
  - Status: ✅ COMPLETE - Ready for merge to dev
