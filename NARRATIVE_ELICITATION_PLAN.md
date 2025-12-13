# Narrative Elicitation System - Implementation Plan

## Progress Tracker & Quick Navigation

### Implementation Progress
- [ ] [Dependencies Added](#dependencies)
- [ ] [Step 1: Core Trait Definitions](#step-1-core-trait-definitions)
  - [ ] [1.1 ElicitationDialog trait](#11-dialogrs---elicitationdialog-trait)
  - [ ] [1.2 NarrativeElicitor trait](#12-elicitorrs---narrativeelicitor-trait)
  - [ ] [1.3 PartialNarrative state](#13-partialrs---partialnarrative-state)
  - [ ] [1.4 ElicitationSession orchestrator](#14-sessionrs---elicitationsession-orchestrator)
- [ ] [Step 2: Specific Elicitor Implementations](#step-2-specific-elicitor-implementations)
  - [ ] [2.1 MetadataElicitor](#21-metadatars---metadataelicitor)
  - [ ] [2.2 ActElicitor](#22-actsrs---actelicitor)
  - [ ] [2.3 InputElicitor](#23-inputsrs---inputelicitor)
  - [ ] [2.4 CarouselElicitor](#24-carouselrs---carouselelicitor)
  - [ ] [2.5 ValidationElicitor](#25-validationrs---validationelicitor)
  - [ ] [2.6 Module exports](#26-modrs---module-exports)
- [ ] [Step 3: TUI Integration](#step-3-tui-integration)
- [ ] [Step 4: CommandExecutor Integration](#step-4-commandexecutor-integration)
- [ ] [Step 5: Shared Utilities](#step-5-shared-utilities)
- [ ] [Step 6: Error Handling](#step-6-error-handling)
- [ ] [Step 7: Testing](#step-7-testing)
- [ ] [Step 8: Documentation](#step-8-documentation)
- [ ] [Validation Checklist Complete](#validation-checklist)

### Quick Links

**Planning & Requirements**
- [Overview](#overview)
- [Design Goals](#design-goals)
- [Critical Updates (Post-Critique)](#critical-updates-post-critique)
- [Architecture Summary](#architecture-summary)

**Compliance & Standards**
- [CLAUDE.md Compliance Requirements](#claudemd-compliance-requirements)
  - [Instrumentation](#instrumentation-mandatory)
  - [Derive Policy](#derive-policy-mandatory)
  - [Builder Pattern](#builder-pattern-mandatory)
  - [Error Handling Pattern](#error-handling-pattern-mandatory)
  - [Module Organization](#module-organization-mandatory)
  - [Testing](#testing-mandatory)
  - [TUI Async Integration Strategy](#tui-async-integration-strategy)

**Implementation**
- [Implementation Steps](#implementation-steps)
- [Critical Files](#critical-files)
- [Dependencies](#dependencies)
- [Migration Strategy](#migration-strategy)

**Validation & Success**
- [Validation Checklist](#validation-checklist)
- [Success Criteria](#success-criteria)
- [Next Steps After Implementation](#next-steps-after-implementation)

---

## Overview

Replace the existing MCP `create_narrative` tool with a trait-based, conversational elicitation system that supports the full complexity of narrative construction while remaining UI-agnostic (TUI, Web, Android).

## Design Goals

1. **Conversational/Adaptive**: Not a rigid wizard - users can jump between sections, iterate, and refine
2. **Full Complexity**: Support all narrative features (multimodal inputs, carousels, bot commands, table queries, etc.)
3. **Async-First**: All traits async for compatibility with TUI event loops and web frameworks
4. **Composable**: Small, focused elicitors that combine via traits
5. **UI-Agnostic**: Elicitation logic decoupled from UI through abstraction layer

## Critical Updates (Post-Critique)

This plan has been updated to address critical gaps identified during CLAUDE.md compliance review:

### Dependency Fixes
- **Added**: `botticelli_narrative` and `botticelli_core` dependencies to `Cargo.toml`
- **Reason**: Required for validation, Narrative types, and Input enum

### Coding Standards
- **Added**: Comprehensive CLAUDE.md Compliance section with mandatory requirements
- **Specified**: Exact derive policies for all types
- **Specified**: Builder pattern requirement for PartialNarrative
- **Specified**: Error handling pattern must extend existing ChatError
- **Specified**: All public functions need `#[instrument]` attribute

### Architectural Clarifications
- **Resolved**: TUI async integration challenge with `spawn_blocking` strategy
- **Clarified**: No re-exports from workspace crates (botticelli_narrative, botticelli_core)
- **Clarified**: All tests must be in `tests/` directory, zero `#[cfg(test)]` in source

### Enhanced Validation
- **Expanded**: Validation checklist with specific CLAUDE.md compliance items
- **Added**: Pre-commit verification requirements
- **Added**: Quality gates (just check, check-all, markdownlint)

## Architecture Summary

### Core Components

1. **ElicitationDialog trait** - UI interaction primitives (ask_text, ask_choice, ask_confirmation, show_info, etc.)
2. **NarrativeElicitor trait** - Interface for narrative component elicitors
3. **PartialNarrative** - State container for narrative under construction
4. **ElicitationSession** - Orchestrator managing multiple elicitors and flow control
5. **Specific Elicitors** - MetadataElicitor, ActElicitor, InputElicitor, CarouselElicitor, etc.

### Data Flow

```
User Request
    ↓
ElicitationSession::run(dialog)
    ↓
Elicitors (Metadata → Acts → Inputs → Carousel)
    ↓
PartialNarrative (incremental state + validation)
    ↓
Narrative (complete, validated)
```

## CLAUDE.md Compliance Requirements

All code must strictly adhere to the project's CLAUDE.md guidelines:

### Instrumentation (MANDATORY)

**All public functions and trait methods MUST have `#[instrument]`:**

```rust
#[async_trait]
pub trait ElicitationDialog {
    #[instrument(skip(self))]
    async fn ask_text(&mut self, prompt: &str) -> ChatResult<String>;

    #[instrument(skip(self))]
    async fn ask_confirmation(&mut self, prompt: &str, default: bool) -> ChatResult<bool>;
    // ... all methods need #[instrument]
}

#[async_trait]
pub trait NarrativeElicitor: Send + Sync {
    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> ChatResult<()>;
    // ... all async methods need #[instrument]
}
```

### Derive Policy (MANDATORY)

**Data structures:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_getters::Getters)]
pub struct PartialNarrative {
    // fields...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialAct {
    // fields...
}
```

**Error enums:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ChatErrorKind {
    #[display("Elicitation cancelled by user")]
    Cancelled,

    #[display("Invalid elicitation state: {}", _0)]
    InvalidElicitationState(String),
}
```

**Session enums:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElicitationMode {
    Guided,
    Conversational,
    Quick,
}
```

### Builder Pattern (MANDATORY)

**PartialNarrative MUST use builder:**
```rust
use derive_builder::Builder;

#[derive(Debug, Clone, Builder, Serialize, Deserialize, derive_getters::Getters)]
#[builder(setter(into), default)]
pub struct PartialNarrative {
    name: Option<String>,
    description: Option<String>,
    // ... other fields
}
```

**Usage:**
```rust
let partial = PartialNarrativeBuilder::default()
    .name("my_narrative")
    .description("description")
    .build()
    .expect("Valid partial narrative");
```

### Error Handling Pattern (MANDATORY)

**Follow existing ChatError pattern exactly:**
```rust
// In error.rs - ADD to existing ChatErrorKind enum:
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ChatErrorKind {
    // ... existing variants ...

    /// User cancelled elicitation.
    #[display("Elicitation cancelled by user")]
    Cancelled,

    /// Invalid elicitation state.
    #[display("Invalid elicitation state: {}", _0)]
    InvalidElicitationState(String),

    /// Elicitation error.
    #[display("Elicitation error: {}", _0)]
    ElicitationError(String),
}

// ChatError already has the wrapper with location tracking
// Just add convenience constructors:
impl ChatError {
    #[track_caller]
    pub fn cancelled() -> Self {
        Self::new(ChatErrorKind::Cancelled)
    }

    #[track_caller]
    pub fn elicitation_error(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::ElicitationError(msg.into()))
    }
}
```

**Note**: Error `file` field MUST be `&'static str`, not `String` (already correct in existing code).

### Module Organization (MANDATORY)

**lib.rs - Only mod and pub use:**
```rust
// In botticelli_chat/src/lib.rs
mod elicitation;

pub use elicitation::{
    ElicitationDialog,
    ElicitationSession,
    ElicitationMode,
    PartialNarrative,
    // ... other types
};
```

**NO RE-EXPORTS from other workspace crates:**
```rust
// ❌ FORBIDDEN - Do not re-export from other crates
// pub use botticelli_narrative::Narrative;
// pub use botticelli_core::Input;

// ✅ CORRECT - Users import from source crate
use botticelli_narrative::Narrative;
use botticelli_core::Input;
```

**elicitation/mod.rs - Only mod and pub use:**
```rust
// crates/botticelli_chat/src/elicitation/mod.rs
mod dialog;
mod elicitor;
mod partial;
mod session;
mod metadata;
mod acts;
mod inputs;
mod carousel;
mod validation;
mod utils;

pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use partial::{PartialNarrative, PartialAct};
pub use session::{ElicitationSession, ElicitationMode};
pub use metadata::MetadataElicitor;
// ... etc
```

### Testing (MANDATORY)

**No `#[cfg(test)]` in source files - ALL tests in `tests/` directory:**
```
crates/botticelli_chat/
├── src/
│   └── elicitation/
│       ├── dialog.rs       # NO tests here
│       └── partial.rs      # NO tests here
└── tests/
    ├── elicitation_test.rs           # ✅ Tests here
    └── elicitation_integration_test.rs  # ✅ Tests here
```

### TUI Async Integration Strategy

**Challenge**: Current TUI uses synchronous crossterm event loop, but ElicitationDialog trait is async.

**Solution**: Use `tokio::task::spawn_blocking` wrapper:
```rust
// In tui.rs
#[async_trait]
impl ElicitationDialog for TuiInterface {
    async fn ask_text(&mut self, prompt: &str) -> ChatResult<String> {
        // Wrap synchronous event loop in spawn_blocking
        tokio::task::spawn_blocking(move || {
            // Synchronous crossterm event::read() loop
            loop {
                if let Event::Key(key) = event::read()? {
                    // ... handle input
                }
            }
        })
        .await
        .map_err(|e| ChatError::io_error(e.to_string()))?
    }
}
```

**Alternative**: Keep event loop synchronous, call async session from sync context:
```rust
// In TUI main loop
let runtime = tokio::runtime::Runtime::new()?;
runtime.block_on(async {
    session.run(&mut tui_dialog).await
})?;
```

## Implementation Steps

### Step 1: Core Trait Definitions

**New crate:** `crates/botticelli_chat/src/elicitation/`

#### Files to Create:

**1.1 `dialog.rs` - ElicitationDialog trait**
- Define async trait with UI interaction primitives:
  - `ask_text()` - Free-form text input
  - `ask_confirmation()` - Yes/no questions
  - `ask_choice()` - Select from options
  - `ask_number()` - Numeric input with bounds
  - `ask_file_path()` - File path (platform-specific)
  - `show_info()`, `show_warning()`, `show_error()` - Display messages
  - `show_validation()` - Display validation results
  - `show_progress()` - Show step progress
  - `show_preview()` - Preview current TOML
- Use `async_trait` for trait methods
- Add comprehensive documentation with examples

**1.2 `elicitor.rs` - NarrativeElicitor trait**
- Define async trait for component elicitors:
  - `name()` - Human-readable name
  - `description()` - What this elicitor does
  - `can_run()` - Check prerequisites
  - `elicit()` - Main elicitation method
  - `is_complete()` - Check if aspect is complete
  - `suggest_next()` - Optional guidance
- Trait bounds: `Send + Sync` for async compatibility

**1.3 `partial.rs` - PartialNarrative state**
- Define structure matching full Narrative complexity:
  - Metadata fields (name, description, model, temperature, etc.)
  - TOC (act_order)
  - Acts (HashMap<String, PartialAct>)
  - Resources (media, tables, bots)
- **MUST use `#[derive(Builder)]`** - See CLAUDE.md Compliance section
- **MUST use `derive_getters::Getters`** for field access
- Derive: `Debug, Clone, Builder, Serialize, Deserialize, Getters`
- Implement methods:
  - `has_minimum_required()` - Check basic requirements
  - `validate()` - Use existing `botticelli_narrative::validator`
  - `to_toml()` - Serialize to TOML string
  - `try_into_narrative()` - Convert to complete Narrative
- **All methods MUST have `#[instrument]`**
- Support serde for serialization (save/resume sessions)
- Construction via `PartialNarrativeBuilder::default()`, NOT struct literals

**1.4 `session.rs` - ElicitationSession orchestrator**
- Define ElicitationMode enum: Guided, Conversational, Quick
- Implement orchestration logic:
  - `run_guided()` - Sequential elicitor execution
  - `run_conversational()` - User-driven section selection
  - `run_quick()` - Minimal questions, smart defaults
  - `finalize()` - Final validation and conversion
- Manage elicitor registration and dependencies
- Support partial narrative save/resume

### Step 2: Specific Elicitor Implementations

#### Files to Create:

**2.1 `metadata.rs` - MetadataElicitor**
- Elicit [narrative] section:
  - Name (required, validated format)
  - Description (optional)
  - Default model (choice from common models + custom)
  - Temperature (optional, 0.0-2.0)
  - Max tokens (optional)
  - Budget config (optional, if feature enabled)
- Smart defaults and validation
- Prerequisites: None (can always run first)

**2.2 `acts.rs` - ActElicitor**
- Elicit acts and ordering:
  - Workflow description → auto-extract acts (use logic from MCP tool)
  - Manual count-based specification
  - Import from file
  - Per-act configuration (name, inputs, overrides)
- Delegate to InputElicitor for act inputs
- Prerequisites: Metadata complete

**2.3 `inputs.rs` - InputElicitor**
- Elicit inputs for a single act:
  - Simple text prompt
  - Multimodal (text + image/audio/video/document)
  - Narrative reference
  - Bot command
  - Table query
- Handle all Input enum variants from `botticelli_core`
- Support multiple inputs per act

**2.4 `carousel.rs` - CarouselElicitor**
- Elicit carousel configuration:
  - Scope: Narrative-level or Act-level
  - Iterations (1-1000)
  - Estimated tokens per iteration
  - Continue on error (boolean)
  - Budget multipliers (if feature enabled)
- Prerequisites: Metadata for narrative-level, specific act exists for act-level

**2.5 `validation.rs` - ValidationElicitor**
- Interactive validation and fix suggestions:
  - Display errors with priority (critical/high/medium)
  - Offer to fix common issues automatically
  - Guide user through manual fixes
  - Re-validate after fixes
- Leverage existing `botticelli_narrative::validator`

**2.6 `mod.rs` - Module exports**
- Re-export all public types
- Module organization following project conventions

### Step 3: TUI Integration

#### Files to Modify:

**3.1 `crates/botticelli_chat/src/tui.rs`**
- Implement `ElicitationDialog` for `TuiInterface`:
  - `ask_text()` - Use existing input buffer + event loop
  - `ask_confirmation()` - Parse y/n/yes/no responses
  - `ask_choice()` - Display numbered options, parse selection
  - `ask_number()` - Parse and validate numeric input
  - `ask_file_path()` - Text input (no picker in TUI)
  - `show_*()` methods - Add messages to display
  - `show_preview()` - Display TOML with syntax highlighting (optional)
- Handle async event loop integration
- Add new message rendering for elicitation prompts

### Step 4: CommandExecutor Integration

#### Files to Modify:

**4.1 `crates/botticelli_chat/src/executor.rs`**
- Add new command handler: `handle_create_narrative_interactive()`
- Replace existing `handle_create_narrative()` to use elicitation:
  - Ask user to choose mode (Guided/Conversational/Quick)
  - Create `ElicitationSession` with chosen mode
  - Pass TUI dialog implementation to session
  - Run elicitation and get validated Narrative
  - Save to database or file based on user choice
- Keep MCP integration for backward compatibility (optional)
- Update help text to reflect new elicitation flow

**4.2 `crates/botticelli_chat/src/command.rs`**
- Update `NarrativeCommand::Create` to trigger new elicitation flow
- Consider adding mode parameter (or ask during execution)

### Step 5: Shared Utilities

#### Files to Create:

**5.1 `crates/botticelli_chat/src/elicitation/utils.rs`**
- `extract_acts_from_description()` - AI-based act parsing (from MCP tool)
- `is_valid_narrative_name()` - Name validation
- `format_toml()` - TOML formatting (from MCP tool)
- `add_helpful_comments()` - Inline documentation (from MCP tool)
- Common validation helpers

### Step 6: Error Handling

#### Files to Modify:

**6.1 `crates/botticelli_chat/src/error.rs`**
- **MUST follow existing ChatError pattern** - See CLAUDE.md Compliance section
- Add variants to existing `ChatErrorKind` enum:
  - `Cancelled` - User cancelled elicitation
  - `InvalidElicitationState(String)` - Internal state errors
  - `ElicitationError(String)` - General elicitation errors
- **MUST use `#[display(...)]` on each variant**
- Add convenience constructors to `ChatError` impl:
  - `cancelled()` with `#[track_caller]`
  - `elicitation_error(msg)` with `#[track_caller]`
- **Do NOT create new error wrapper types** - extend existing ChatError

### Step 7: Testing

#### Files to Create:

**7.1 `crates/botticelli_chat/tests/elicitation_test.rs`**
- Unit tests for each elicitor:
  - MetadataElicitor with valid/invalid inputs
  - ActElicitor with different approaches
  - InputElicitor for all input types
  - CarouselElicitor configuration
- Integration tests for ElicitationSession:
  - Guided mode flow
  - Conversational mode navigation
  - Quick mode with defaults
- Mock ElicitationDialog implementation for testing
- Test partial narrative validation

**7.2 `crates/botticelli_chat/tests/elicitation_integration_test.rs`**
- End-to-end tests:
  - Complete narrative creation via elicitation
  - Save and resume partial narratives
  - Validation error handling
  - Mode switching scenarios

### Step 8: Documentation

#### Files to Create/Modify:

**8.1 Update PLANNING_INDEX.md**
- Add entry for this implementation plan
- Mark as complete when done

**8.2 Create examples/elicitation_demo.md**
- Show example flows for each mode
- Document common patterns
- UI-agnostic examples

**8.3 Update crates/botticelli_chat/README.md**
- Document elicitation system architecture
- Show how to implement ElicitationDialog for new platforms
- Link to examples

## Critical Files

### New Files (to create):
- `crates/botticelli_chat/src/elicitation/mod.rs`
- `crates/botticelli_chat/src/elicitation/dialog.rs`
- `crates/botticelli_chat/src/elicitation/elicitor.rs`
- `crates/botticelli_chat/src/elicitation/partial.rs`
- `crates/botticelli_chat/src/elicitation/session.rs`
- `crates/botticelli_chat/src/elicitation/metadata.rs`
- `crates/botticelli_chat/src/elicitation/acts.rs`
- `crates/botticelli_chat/src/elicitation/inputs.rs`
- `crates/botticelli_chat/src/elicitation/carousel.rs`
- `crates/botticelli_chat/src/elicitation/validation.rs`
- `crates/botticelli_chat/src/elicitation/utils.rs`
- `crates/botticelli_chat/tests/elicitation_test.rs`
- `crates/botticelli_chat/tests/elicitation_integration_test.rs`
- `examples/elicitation_demo.md`

### Modified Files:
- `crates/botticelli_chat/src/lib.rs` - Export elicitation module (NO re-exports from other crates)
- `crates/botticelli_chat/src/tui.rs` - Implement ElicitationDialog with async wrapper
- `crates/botticelli_chat/src/executor.rs` - Add elicitation integration
- `crates/botticelli_chat/src/command.rs` - Update Create command (optional)
- `crates/botticelli_chat/src/error.rs` - Add error kinds to existing ChatErrorKind enum
- `crates/botticelli_chat/Cargo.toml` - Add `botticelli_narrative` and `botticelli_core` dependencies
- `PLANNING_INDEX.md` - Add plan entry

### Integration Points:
- `crates/botticelli_narrative/src/validator.rs` - Used by PartialNarrative::validate()
- `crates/botticelli_narrative/src/core.rs` - Target Narrative type
- `crates/botticelli_core/src/input.rs` - Input enum variants
- `crates/botticelli_mcp/src/tools/create_narrative.rs` - Reuse act extraction logic

## Dependencies

Add to `crates/botticelli_chat/Cargo.toml`:
```toml
[dependencies]
async-trait = "0.1"  # Already present
botticelli_narrative = { path = "../botticelli_narrative" }
botticelli_core = { path = "../botticelli_core" }
```

**Critical**: These dependencies are required for:
- `botticelli_narrative::validator` - Used by PartialNarrative validation
- `botticelli_narrative::{Narrative, CarouselConfig}` - Target types
- `botticelli_core::Input` - Input enum variants for InputElicitor

## Migration Strategy

1. **Phase 1**: Implement core traits and PartialNarrative (non-breaking)
2. **Phase 2**: Implement specific elicitors (non-breaking)
3. **Phase 3**: Add TUI integration alongside existing MCP flow
4. **Phase 4**: Update CommandExecutor to use elicitation (breaking change to create command)
5. **Phase 5**: Deprecate MCP create_narrative tool (document migration path)

## Validation Checklist

### CLAUDE.md Compliance (MANDATORY)
- [ ] **All public functions have `#[instrument]`** - No exceptions
- [ ] **All trait methods have `#[instrument(skip(...))]`** - ElicitationDialog and NarrativeElicitor
- [ ] **PartialNarrative uses `#[derive(Builder)]`** - No `new()` method, use builder
- [ ] **All derives specified correctly** - Debug, Clone, Getters, Serialize, Deserialize, etc.
- [ ] **Error handling follows ChatError pattern** - Add to existing enum, use derive_more::Display
- [ ] **Error constructors have `#[track_caller]`** - For location tracking
- [ ] **lib.rs has ONLY mod and pub use** - No type definitions
- [ ] **elicitation/mod.rs has ONLY mod and pub use** - No type definitions
- [ ] **NO re-exports from other workspace crates** - No `pub use botticelli_narrative::*`
- [ ] **All tests in `tests/` directory** - Zero `#[cfg(test)]` in source files
- [ ] **Never use `#[allow]`** - Fix root cause instead
- [ ] **TUI async integration uses spawn_blocking** - Bridge sync/async correctly

### Functionality
- [ ] All elicitors implement NarrativeElicitor trait correctly
- [ ] PartialNarrative validates using existing `botticelli_narrative::validator`
- [ ] All input types from `botticelli_core::Input` are supported
- [ ] Session can save/resume partial narratives
- [ ] Error handling is comprehensive and user-friendly

### Testing
- [ ] Tests cover happy path and error cases
- [ ] Mock ElicitationDialog for unit tests
- [ ] Integration tests for full elicitation flows
- [ ] Tests pass: `just test-package botticelli_chat`

### Quality
- [ ] Passes `just check botticelli_chat` - Zero errors
- [ ] Passes `just check-all botticelli_chat` - Zero warnings, all tests pass
- [ ] Passes `markdownlint-cli2 "**/*.md"` - If markdown changed
- [ ] Documentation is complete and includes examples

## Success Criteria

1. Users can create simple narratives via conversational elicitation
2. Users can create complex narratives with all features (carousels, multimodal, etc.)
3. Elicitation state can be saved and resumed
4. Validation errors are caught early and provide helpful guidance
5. Architecture supports future implementations for Web and Android
6. Code passes all checks: `just check-all`, `markdownlint-cli2`

## Next Steps After Implementation

1. Web implementation of ElicitationDialog (using WebSockets or REST)
2. Android implementation of ElicitationDialog (using JNI bridge)
3. Additional elicitors for advanced features (BudgetElicitor, TemplateElicitor, etc.)
4. AI-assisted elicitation (suggest acts, inputs, models based on description)
5. Collaborative elicitation (multiple users editing same narrative)
