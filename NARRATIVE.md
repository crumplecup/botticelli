# Narrative System Implementation Plan

## Overview

Implement a narrative execution system that reads multi-act prompts from TOML files and executes them in sequence against LLM APIs. This enables automated multi-step content generation workflows.

## Main Goals

1. Parse narrative TOML files with metadata, table of contents, and acts
2. Execute prompts in the order specified by the table of contents
3. Pass context between acts (each act can reference previous outputs)
4. Store the complete narrative execution history in the database
5. Provide a CLI interface to run narratives

## Example Use Case

The `narrations/mint.toml` file defines a three-act narrative for generating social media content:
- Act 1: Generate initial content
- Act 2: Critique the content
- Act 3: Improve based on critique

## Implementation Steps

### Step 1: Define Narrative Data Structures ✓ COMPLETE

**Completed:**
- Created `src/narrative/` module with proper organization:
  - `core.rs` - Data structures and parsing logic
  - `error.rs` - Error types following project conventions
  - `mod.rs` - Module exports only
- Implemented `Narrative` struct with:
  - `NarrativeMetadata` - name and description
  - `NarrativeToc` - ordered list of act names
  - `acts: HashMap<String, String>` - map of act names to prompts
- Added TOML parsing with `toml` crate
- Implemented `FromStr` trait for idiomatic parsing
- Added comprehensive validation:
  - Non-empty table of contents
  - All acts referenced in toc exist
  - No empty prompts
- Error handling with `NarrativeError` and `NarrativeErrorKind`
  - Integrated into crate-level `BoticelliError`
  - Uses `derive_new` for clean construction
  - Uses `derive_more::Display` for formatting
- Full compliance with CLAUDE.md guidelines:
  - Proper derives: `Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`
  - Module organization with types in core.rs
  - Exported at crate level in lib.rs
- Tests: 5 passing tests covering validation and parsing

### Step 2: Create Narrative Parser ✓ COMPLETE

**Completed:**
- Parser integrated into `src/narrative/core.rs`
- `Narrative::from_file()` - loads from TOML file path
- `FromStr` trait implementation - parses from TOML string
- Comprehensive error handling:
  - File I/O errors (`FileRead`)
  - TOML parse errors (`TomlParse`)
  - Validation errors (`EmptyToc`, `MissingAct`, `EmptyPrompt`)
- Unit tests in `tests/narrative_test.rs`:
  - Loads `narrations/mint.toml` successfully
  - Validates empty toc rejection
  - Validates missing act detection
  - Validates empty prompt detection
  - Validates well-formed narratives

### Step 3: Implement Narrative Executor ✓ COMPLETE

**Completed:**
- Created `NarrativeExecutor<D: BoticelliDriver>` in `src/narrative/executor.rs`
- Implemented sequential act processing:
  - Builds `GenerateRequest` with conversation history
  - Calls LLM API using `BoticelliDriver::generate()`
  - Extracts text responses from `Output` enum
  - Maintains alternating User/Assistant message history
- Context passing strategy: conversation history approach
  - Each act sees all previous outputs as conversation context
  - Enables multi-step workflows (generate → critique → improve)
- Data structures:
  - `ActExecution` - stores prompt, response, metadata for single act
  - `NarrativeExecution` - aggregates complete execution with all acts
- Proper derives: `Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`
- Error handling with `BoticelliResult`
- Exported types at crate level in `lib.rs`
- Tests: 4 passing tests in `tests/narrative_executor_test.rs`
  - Mock driver for deterministic testing
  - Tests cover: single/multiple acts, context passing, driver access
  - Context tracking test validates conversation history growth

### Step 4: Database Schema for Narrative Executions

- Create table `narrative_executions` to store execution metadata
  - `id` - unique identifier
  - `narrative_name` - which narrative was run
  - `started_at` - timestamp
  - `completed_at` - timestamp (nullable)
  - `status` - (running, completed, failed)
- Create table `narrative_act_outputs` to store individual act results
  - `id` - unique identifier
  - `execution_id` - foreign key to narrative_executions
  - `act_name` - which act this is from
  - `prompt` - the prompt that was sent
  - `response` - the LLM response
  - `sequence_number` - order in the execution
  - `created_at` - timestamp
- Add Diesel migrations
- Create Rust models for these tables

### Step 5: CLI Interface

- Use `clap` crate to define command-line arguments
- Add CLI command to run narratives (e.g., `--narrative narrations/mint.toml`)
- Add option to specify which LLM backend to use
- Display progress as acts execute
- Show final results and where they're stored

### Step 6: Testing and Documentation

- Write integration tests that run a test narrative end-to-end
- Add example narratives to demonstrate capabilities
- Document the TOML format specification
- Update README with narrative usage examples

## Open Questions

1. **Context Passing**: Should each act see all previous outputs, or only the immediate predecessor?
2. **Streaming**: Should we support streaming outputs for narrative execution?
3. **Error Handling**: If act 2 fails, should we store partial results or rollback?
4. **Variables**: Should we support variable substitution in prompts (e.g., `${act1.response}`)?
5. **Multiple Models**: Should different acts be able to use different LLM models?

## Dependencies

- ✓ **Added**: `toml = "0.8"` - TOML parsing for narrative files
- ✓ **Added**: `clap = "4"` - CLI argument parsing
- ✓ **Added**: `derive-new = "0.7"` - Clean error construction
- Existing: `serde` for deserialization (already in project)
- Existing: `derive_more` for Display/Error derives
- Existing: Database infrastructure (Diesel) - for future steps
- Existing: BoticelliDriver trait for LLM calls - for future steps
