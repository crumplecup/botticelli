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
- **BUDGET_MULTIPLIER_DESIGN.md** - `e0a7ebc` (2025-11-24)
  - Design for API rate limit budget multipliers
- **OPENTELEMETRY_INTEGRATION_PLAN.md** - `current` (2025-11-29)
  - Comprehensive OpenTelemetry observability integration plan
- **BOT_SERVER_OBSERVABILITY_STRATEGY.md** - `current` (2025-11-28)
  - Original observability strategy (pre-OpenTelemetry research)

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

- **Active Planning**: 11 documents
- **Setup & Configuration**: 11 documents
- **Crate Documentation**: 13 documents
- **Narrative Guides**: 3 documents
- **Archived**: 26 documents

**Total**: 64 markdown documents tracked (38 active, 26 archived)

---

*Last Updated: 2025-11-29*
*Generated automatically - see git log for detailed history*
