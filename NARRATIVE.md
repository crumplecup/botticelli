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
  - `ActExecution` - stores inputs, model, temperature, max_tokens, response, and metadata
  - `NarrativeExecution` - aggregates complete execution with all acts
- Proper derives: `Debug, Clone, PartialEq, Serialize, Deserialize`
- Error handling with `BoticelliResult`
- Exported types at crate level in `lib.rs`
- Tests: 6 passing tests in `tests/narrative_executor_test.rs`
  - Mock driver for deterministic testing
  - Tests cover: single/multiple acts, context passing, driver access
  - Context tracking test validates conversation history growth
  - Trait abstraction test with in-memory provider
  - Multimodal configuration test

### Step 3.5: Architecture Improvements ✓ COMPLETE

**NarrativeProvider Trait Abstraction:**
- Created `NarrativeProvider` trait in `src/narrative/provider.rs`
- Decouples executor from TOML configuration format
- Trait methods:
  - `name()` - Get narrative identifier
  - `act_names()` - Get ordered act list
  - `get_act_config()` - Retrieve act configuration
- Benefits:
  - Format flexibility (easy to add YAML, JSON, database sources)
  - Better testability (simple mock implementations)
  - Reduced coupling (config changes don't ripple through executor)
- `Narrative` struct implements `NarrativeProvider`
- `NarrativeExecutor::execute()` is generic over `NarrativeProvider`

**Multimodal and Per-Act Configuration:**
- Created `ActConfig` struct for flexible act configuration:
  - `inputs: Vec<Input>` - Supports text, images, audio, video, documents
  - `model: Option<String>` - Per-act model override
  - `temperature: Option<f32>` - Per-act temperature override
  - `max_tokens: Option<u32>` - Per-act max_tokens override
- Builder pattern methods:
  - `ActConfig::from_text()` - Simple text constructor
  - `ActConfig::from_inputs()` - Multimodal constructor
  - `.with_model()`, `.with_temperature()`, `.with_max_tokens()` - Fluent API
- Executor applies per-act overrides to `GenerateRequest`
- `ActExecution` stores full configuration used for each act
- Tests demonstrate:
  - Per-act model selection (GPT-4, Claude, Gemini)
  - Per-act temperature/max_tokens overrides
  - Multimodal inputs (text + image in single act)

**TOML Specification Design:**
- Created `NARRATIVE_TOML_SPEC.md` - Complete specification
- Created `narrations/showcase.toml` - Comprehensive example
- Format features:
  - Backward compatible simple text: `act = "text"`
  - Structured acts with `[[acts.act_name.input]]` array-of-tables
  - Native TOML syntax (idiomatic, readable)
  - Multiple input types per act
  - Source flexibility: `url`, `base64`, `file` fields
  - Per-act configuration overrides
- Example narrative demonstrates:
  - 8 acts with different configurations
  - Vision (text+image), audio transcription, video analysis
  - Document review, creative brainstorming, technical synthesis
  - Different models per act (GPT-4, Claude, Gemini, Whisper)

### Step 3.6: TOML Parsing Implementation ✓ COMPLETE

**Multimodal TOML Deserialization:**
- Created `src/narrative/toml.rs` - TOML parsing layer (168 lines)
  - `TomlNarrative`, `TomlToc`, `TomlNarration` - Root structures
  - `TomlAct` enum - Supports Simple(String) and Structured(TomlActConfig)
  - `TomlActConfig` - Structured act with input array and optional overrides
  - `TomlInput` - Input type with flexible source detection
  - Source detection logic: checks for `url`, `base64`, or `file` field
  - Conversion methods: `to_input()`, `to_act_config()`
- Updated `Narrative` structure:
  - Changed `acts` from `HashMap<String, String>` to `HashMap<String, ActConfig>`
  - Updated validation for multimodal inputs (checks inputs.is_empty())
  - Rewrote `FromStr` to parse via intermediate `TomlNarrative`
  - Smart error handling (distinguishes empty prompts from parse errors)
- Features:
  - Supports mixed simple and structured acts in same file
  - Backward compatible with simple text format (mint.toml)
  - Validates empty text prompts during parsing
  - Proper error messages with act name context
  - File source support (reads file into Binary MediaSource)

**Testing:**
- Added `test_multimodal_toml_parsing` - Comprehensive inline TOML test
  - Tests simple text acts
  - Tests vision acts (text + image with model/temp/max_tokens)
  - Tests mixed media acts (text + image + document)
  - Verifies source types, MIME types, filenames
- Added `test_load_showcase_narrative` - Real file parsing test
  - Successfully parses narrations/showcase.toml
  - Verifies all 8 acts with different input types
  - Tests audio, video, document parsing
- All 13 tests passing (7 narrative + 6 executor)
- Zero clippy warnings

**TOML Parsing Now Fully Functional:**
The complete pipeline works end-to-end:
TOML file → Parse → ActConfig → NarrativeExecutor → LLM API

### Step 4: Database Schema for Narrative Executions

**Schema Design Goals:**
- Store complete execution history with full reproducibility
- Preserve multimodal input configurations
- Track per-act model selection and parameters
- Handle media sources (URL, Base64, Binary) efficiently
- Enable querying and analysis of execution patterns

**Proposed Tables:**

1. **`narrative_executions`** - Top-level execution tracking
   - `id` (SERIAL PRIMARY KEY) - Unique identifier
   - `narrative_name` (TEXT NOT NULL) - Which narrative was run
   - `narrative_description` (TEXT) - Description from narrative metadata
   - `started_at` (TIMESTAMP NOT NULL) - Execution start time
   - `completed_at` (TIMESTAMP) - Execution completion time (NULL if running)
   - `status` (TEXT NOT NULL) - Execution status: 'running', 'completed', 'failed'
   - `error_message` (TEXT) - Error details if status='failed'
   - `created_at` (TIMESTAMP DEFAULT NOW())

2. **`act_executions`** - Individual act execution results
   - `id` (SERIAL PRIMARY KEY) - Unique identifier
   - `execution_id` (INTEGER NOT NULL REFERENCES narrative_executions(id) ON DELETE CASCADE)
   - `act_name` (TEXT NOT NULL) - Name of the act from narrative
   - `sequence_number` (INTEGER NOT NULL) - Order in execution (0-indexed)
   - `model` (TEXT) - Model used (if overridden from default)
   - `temperature` (REAL) - Temperature parameter (if overridden)
   - `max_tokens` (INTEGER) - Max tokens parameter (if overridden)
   - `response` (TEXT NOT NULL) - LLM text response
   - `created_at` (TIMESTAMP DEFAULT NOW())
   - INDEX on (execution_id, sequence_number)

3. **`act_inputs`** - Multimodal inputs for each act
   - `id` (SERIAL PRIMARY KEY) - Unique identifier
   - `act_execution_id` (INTEGER NOT NULL REFERENCES act_executions(id) ON DELETE CASCADE)
   - `input_order` (INTEGER NOT NULL) - Position in inputs array (0-indexed)
   - `input_type` (TEXT NOT NULL) - 'text', 'image', 'audio', 'video', 'document'
   - `text_content` (TEXT) - For Text inputs (NULL for media types)
   - `mime_type` (TEXT) - MIME type for media inputs
   - `source_type` (TEXT) - 'url', 'base64', 'binary' (NULL for text)
   - `source_url` (TEXT) - URL source (if source_type='url')
   - `source_base64` (TEXT) - Base64 content (if source_type='base64')
   - `source_binary` (BYTEA) - Binary content (if source_type='binary')
   - `source_size_bytes` (BIGINT) - Size of binary/base64 data (for monitoring)
   - `content_hash` (TEXT) - SHA256 hash of content (for future deduplication)
   - `filename` (TEXT) - Optional filename (for Document inputs)
   - `created_at` (TIMESTAMP DEFAULT NOW())
   - INDEX on (act_execution_id, input_order)
   - INDEX on (content_hash) - For future deduplication queries

**Implementation Tasks:**
- [ ] Create Diesel migration for all three tables with proper constraints
- [ ] Create Diesel schema in `src/schema.rs`
- [ ] Create DB models in `src/db/` module:
  - `models.rs` - Diesel models (`NarrativeExecutionRow`, `ActExecutionRow`, `ActInputRow`)
  - `operations.rs` - Database operations (save, load, query)
  - `conversions.rs` - Convert between domain types and DB models
- [ ] Implement `save_execution()`:
  - Insert into narrative_executions
  - For each act: insert into act_executions
  - For each input: insert into act_inputs
  - Wrap in transaction for atomicity
- [ ] Implement `load_execution(id)`:
  - Join across all three tables
  - Reconstruct NarrativeExecution with all ActExecutions and Inputs
- [ ] Implement `list_executions()` with filtering:
  - Filter by narrative_name
  - Filter by date range
  - Filter by status
  - Pagination support
- [ ] Add unit tests using in-memory SQLite for CI
- [ ] Add integration tests with PostgreSQL (optional, requires DB)

**Design Decisions:**

1. **Binary Media Storage**: ✓ RESOLVED
   - Store everything in PostgreSQL using Diesel
   - URLs stored as TEXT (source_url)
   - Base64 stored as TEXT (source_base64)
   - Binary data stored as BYTEA (source_binary)
   - Focus on text and images (video support deferred to future work)
   - Trade-off: Simplicity and ACID guarantees over performance at massive scale

2. **Supported Media Types (Initial Implementation)**:
   - ✓ Text inputs (primary use case)
   - ✓ Image inputs (PNG, JPEG, WebP, GIF)
   - ✓ Audio inputs (MP3, WAV, OGG) - stored but not primary focus
   - ✓ Document inputs (PDF, TXT, etc.)
   - ⏸️ Video inputs - **deferred to future work** (large file sizes)

**Open Design Questions:**

3. **Media Deduplication**: Should we deduplicate media by content hash?
   - Would save space for repeated images across executions
   - Adds complexity to deletion and reference tracking
   - **Recommendation**: Add `content_hash` column now, implement deduplication later if needed

4. **Retention Policy**: How long to keep execution history?
   - Automatic cleanup of old executions?
   - Archive completed executions to separate table?
   - **Recommendation**: Start without automatic cleanup, add retention policies based on actual usage

5. **Conversation History**: Should we store intermediate conversation state?
   - Currently only storing final response per act
   - Could store full Message history for debugging
   - **Recommendation**: Store only final responses initially, add Message history if debugging requires it

6. **Performance Considerations**:
   - Should we lazy-load binary media when querying executions?
   - **Recommendation**: Start with eager loading, add lazy loading if performance becomes an issue

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

## Resolved Design Decisions

1. **Context Passing** ✓ RESOLVED: Each act sees all previous outputs via conversation history.
   - Implemented using alternating User/Assistant messages
   - More flexible than immediate predecessor only
   - Enables complex multi-step workflows

2. **Multiple Models** ✓ RESOLVED: Yes, per-act model selection supported.
   - Implemented via `ActConfig.model` optional override
   - Act 1 can use GPT-4, Act 2 can use Claude, etc.
   - Enables using best model for each task type

3. **Multimodal Inputs** ✓ RESOLVED: Fully supported via `ActConfig.inputs: Vec<Input>`.
   - Acts can combine text, images, audio, video, documents
   - Flexible source types: URL, base64, file paths
   - TOML spec designed for all input types

4. **Configuration Format** ✓ RESOLVED: Trait-based abstraction with TOML implementation.
   - `NarrativeProvider` trait decouples format from execution
   - TOML spec uses idiomatic array-of-tables syntax
   - Easy to add YAML, JSON, or database sources later

## Open Questions

1. **Streaming**: Should we support streaming outputs for narrative execution?
   - Would require streaming version of executor
   - Could show incremental progress during long generations

2. **Error Handling**: If act 2 fails, should we store partial results or rollback?
   - Current: propagates error immediately
   - Could add retry logic, partial saving, or checkpoint/resume

3. **Variables**: Should we support variable substitution in prompts (e.g., `${act1.response}`)?
   - Current: entire conversation history available
   - Explicit variables could enable more precise references

4. **Parallelization**: Should we support parallel act execution for independent acts?
   - Current: strictly sequential
   - Could add DAG-based execution for independent branches

## Dependencies

- ✓ **Added**: `toml = "0.8"` - TOML parsing for narrative files
- ✓ **Added**: `clap = "4"` - CLI argument parsing
- ✓ **Added**: `derive-new = "0.7"` - Clean error construction
- Existing: `serde` for deserialization (already in project)
- Existing: `derive_more` for Display/Error derives
- Existing: Database infrastructure (Diesel) - for future steps
- Existing: BoticelliDriver trait for LLM calls - integrated ✓

## Current Implementation Status

### ✅ Completed (Fully Functional)
- Core data structures (`Narrative`, `NarrativeMetadata`, `NarrativeToc`)
- TOML parsing (both simple and multimodal formats)
  - Simple text acts: `act = "text"`
  - Structured acts with `[[acts.name.input]]` arrays
  - Mixed formats in same file
  - Source detection (url/base64/file)
- Narrative executor with conversation history
- Trait-based architecture (`NarrativeProvider`, `ActConfig`)
- Multimodal input support (text, image, audio, video, document)
- Per-act configuration (model, temperature, max_tokens)
- Comprehensive test suite (13 tests passing)
- TOML specification document
- Example narratives (mint.toml, showcase.toml)
- **End-to-end functional**: TOML → Parse → Execute → LLM

### 🚧 Next Implementation Tasks

**Near-term (Database Integration):**
1. Database schema (Step 4)
2. Diesel migrations
3. Models for narrative_executions and narrative_act_outputs
4. Save/load execution history

**Future (CLI and Advanced Features):**
1. CLI interface (Step 5)
2. Streaming support
3. Checkpoint/resume for long narratives
4. Variable substitution
5. Parallel execution for independent acts

## Files and Locations

**Core Implementation:**
- `src/narrative/core.rs` - Data structures and Narrative implementation
- `src/narrative/provider.rs` - NarrativeProvider trait and ActConfig
- `src/narrative/executor.rs` - NarrativeExecutor implementation
- `src/narrative/toml.rs` - TOML deserialization layer
- `src/narrative/error.rs` - Error types
- `src/narrative/mod.rs` - Module exports

**Tests:**
- `tests/narrative_test.rs` - Parser and validation tests (7 tests)
  - Simple text parsing
  - Validation tests (empty toc, missing act, empty prompt)
  - Multimodal TOML parsing
  - showcase.toml parsing
- `tests/narrative_executor_test.rs` - Executor tests (6 tests)
  - Simple/multiple act execution
  - Context passing verification
  - Trait abstraction
  - Multimodal configuration

**Documentation:**
- `NARRATIVE.md` - This file (implementation plan)
- `NARRATIVE_TOML_SPEC.md` - Complete TOML format specification

**Examples:**
- `narrations/mint.toml` - Simple text-only narrative
- `narrations/showcase.toml` - Comprehensive multimodal example
