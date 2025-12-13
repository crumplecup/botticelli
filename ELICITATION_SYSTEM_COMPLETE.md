# Narrative Elicitation System - Implementation Complete

**Status**: ✅ Production Ready  
**Date**: 2024-12-13  
**Implementation**: Steps 1-4 Complete

## Overview

A trait-based, conversational system for creating narrative TOML files interactively through guided dialogs. Platform-agnostic architecture supports multiple UIs (TUI, Web, Android).

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────┐
│         ElicitationDialog (Trait)               │
│  Platform-agnostic UI abstraction               │
│  • ask_text(), ask_confirmation(), ask_choice() │
│  • show_info(), show_warning(), show_error()    │
└─────────────────────────────────────────────────┘
                     ▲
                     │ implements
                     │
        ┌────────────┴─────────────┐
        │                          │
┌───────────────────┐    ┌──────────────────┐
│ TuiElicitationDialog│    │ Future: WebDialog │
│   (ratatui)         │    │   AndroidDialog   │
└───────────────────┘    └──────────────────┘

┌─────────────────────────────────────────────────┐
│      NarrativeElicitor (Trait)                  │
│  Component-specific elicitation                 │
│  • can_run() - prerequisite checking            │
│  • elicit() - gather information                │
│  • is_complete() - validation                   │
└─────────────────────────────────────────────────┘
                     ▲
                     │ implements
        ┌────────────┼────────────┐
        │            │            │
┌──────────────┐ ┌─────────┐ ┌─────────┐
│ MetadataElicitor│ ActElicitor│ Future... │
└──────────────┘ └─────────┘ └─────────┘

┌─────────────────────────────────────────────────┐
│        ElicitationSession                       │
│  Orchestrates elicitation flow                  │
│  • Manages elicitor sequence                    │
│  • Prerequisite checking                        │
│  • Retry/skip error handling                    │
│  • Progress tracking                            │
│  • Validation integration                       │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│        PartialNarrative                         │
│  State container (builder pattern)              │
│  • Optional fields during construction          │
│  • TOML generation                              │
│  • Validation integration                       │
│  • Conversion to complete Narrative             │
└─────────────────────────────────────────────────┘
```

## Implementation Status

### ✅ Step 1: Core Trait Definitions
- `ElicitationDialog` - UI abstraction (12 methods)
- `NarrativeElicitor` - Elicitor interface (5 methods)
- `PartialNarrative` - State container with builder
- `PartialAct` - Act definition helper

**Files:**
- `src/elicitation/dialog.rs`
- `src/elicitation/elicitor.rs`
- `src/elicitation/partial.rs`

### ✅ Step 2: Specific Elicitors

**MetadataElicitor** - Gathers narrative metadata
- Name (validated)
- Description
- Optional: default model, temperature, max_tokens
- Interactive model selection menu
- Prerequisites: None

**ActElicitor** - Defines workflow acts
- Three approaches:
  1. Auto-extract from description (uses NarrativeHelper)
  2. Count-based (specify N acts, enter prompts)
  3. One-by-one interactive entry
- Validates act names
- Prevents duplicates
- Prerequisites: Metadata complete

**Files:**
- `src/elicitation/metadata.rs`
- `src/elicitation/acts.rs`

### ✅ Step 3: Session Orchestrator

**ElicitationSession** - Manages complete flow
- Sequential elicitor execution
- Prerequisite checking via `can_run()`
- Retry/skip logic for failures
- Progress reporting (current/total)
- Preview and validation methods
- Finalization to complete Narrative

**Features:**
- `run()` - Execute all elicitors
- `step()` - Manual step-by-step control
- `preview()` - Show generated TOML
- `validate()` - Run validation checks
- `finalize()` - Convert to Narrative

**File:** `src/elicitation/session.rs`

### ✅ Step 4: TUI Implementation

**TuiElicitationDialog** - Terminal interface
- Ratatui-based interactive UI
- Color-coded messages (ℹ ⚠ ✖ ▶ 👁)
- Input validation with retry loops
- Backspace and Ctrl+C support
- Auto cleanup on drop

**Features:**
- Split layout (messages + input)
- Message history with scrolling
- Progress indicators
- TOML preview rendering
- Feature-gated behind "tui"

**File:** `src/elicitation/tui_dialog.rs`

## Usage Examples

### Basic Usage

```rust
use botticelli_chat::{
    TuiElicitationDialog, ElicitationSession,
    MetadataElicitor, ActElicitor,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> ChatResult<()> {
    // Create dialog
    let mut dialog = TuiElicitationDialog::new()?;
    
    // Setup elicitors
    let elicitors = vec![
        Arc::new(MetadataElicitor::new()),
        Arc::new(ActElicitor::new()),
    ];
    
    // Run session
    let mut session = ElicitationSession::new(elicitors);
    session.run(&mut dialog).await?;
    
    // Preview
    session.preview(&mut dialog).await?;
    
    // Validate
    session.validate(&mut dialog).await?;
    
    // Finalize
    let narrative = session.finalize()?;
    
    // Save to file
    std::fs::write("narrative.toml", narrative.to_toml()?)?;
    
    Ok(())
}
```

### Step-by-Step Control

```rust
let mut session = ElicitationSession::new(elicitors);

// Run one step at a time
while session.step(&mut dialog).await? {
    println!("Progress: {:?}", session.progress());
}

if session.has_minimum_required() {
    let narrative = session.finalize()?;
}
```

### Custom Elicitor

```rust
use async_trait::async_trait;

pub struct InputElicitor;

#[async_trait]
impl NarrativeElicitor for InputElicitor {
    fn name(&self) -> &str {
        "Inputs"
    }
    
    fn description(&self) -> &str {
        "Define narrative inputs and outputs"
    }
    
    fn can_run(&self, partial: &PartialNarrative) -> bool {
        // Requires metadata and acts
        partial.name().is_some() && !partial.acts().is_empty()
    }
    
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> ChatResult<()> {
        // Implementation...
        Ok(())
    }
    
    fn is_complete(&self, partial: &PartialNarrative) -> bool {
        // Check if inputs defined...
        true
    }
}
```

## Key Design Patterns

### 1. Helper Type Pattern
Utilities grouped under `NarrativeHelper` from `botticelli_mcp`:
```rust
use botticelli_mcp::NarrativeHelper;

let acts = NarrativeHelper::extract_acts_from_description(desc);
if NarrativeHelper::is_valid_name(name) { ... }
let toml = NarrativeHelper::escape_toml_string(content);
```

### 2. Builder Pattern
All state updates use builder:
```rust
let updated = PartialNarrativeBuilder::default()
    .name(name)
    .description(description)
    .acts(acts)
    .build()?;
```

### 3. Trait-Based Abstraction
Dialog trait enables platform independence:
- TUI implementation for terminals
- Future: Web implementation for browsers
- Future: Android native UI

### 4. Resilient Execution
Session handles failures gracefully:
- Prerequisite checking before each step
- Retry/skip options on failure
- Progress tracking
- Partial state preservation

## Integration Points

### Reuses Existing Infrastructure
- `botticelli_mcp::NarrativeHelper` - Shared utilities
- `botticelli_narrative::Narrative` - Final output type
- `botticelli_narrative::validator` - Validation
- `ChatError` - Error handling

### Feature Gates
- `tui` - TUI implementation (ratatui)
- `cli` - CLI startup sequence
- Future: `web`, `android`

## Testing Strategy

### Unit Tests
Test individual elicitors with mock dialog:
```rust
struct MockDialog {
    responses: Vec<String>,
    // ...
}

#[async_trait]
impl ElicitationDialog for MockDialog {
    async fn ask_text(&mut self, _: &str) -> ChatResult<String> {
        Ok(self.responses.remove(0))
    }
    // ...
}
```

### Integration Tests
Test complete flows:
```rust
#[tokio::test]
async fn test_full_elicitation_flow() {
    let mut dialog = MockDialog::new();
    dialog.add_response("test_narrative");
    dialog.add_response("A test workflow");
    // ...
    
    let mut session = ElicitationSession::new(elicitors);
    session.run(&mut dialog).await.unwrap();
    
    assert!(session.is_complete());
    assert!(session.has_minimum_required());
}
```

## Future Extensions

### Additional Elicitors (Step 5)
- **InputElicitor** - Define narrative inputs/outputs
- **CarouselElicitor** - Configure carousel mode
- **ValidationElicitor** - Human-in-loop TOML validation
- **RefinementElicitor** - Iterative improvement

### Platform Implementations
- **WebDialog** - Browser-based UI (async WASM)
- **AndroidDialog** - Native mobile UI

### Advanced Features
- Save/restore session state
- Template-based generation
- Import from existing TOML
- Diff/merge for modifications
- Undo/redo support

## Quality Metrics

✅ **Compilation**: Zero warnings across all features  
✅ **Clippy**: Passes with `-D warnings`  
✅ **Feature Gates**: All combinations verified  
✅ **CLAUDE.md**: Full compliance  
✅ **Instrumentation**: All public functions traced  
✅ **Documentation**: Complete inline docs  

## File Structure

```
crates/botticelli_chat/src/elicitation/
├── mod.rs                 # Module exports
├── dialog.rs              # ElicitationDialog trait
├── elicitor.rs            # NarrativeElicitor trait
├── partial.rs             # PartialNarrative state
├── session.rs             # ElicitationSession orchestrator
├── metadata.rs            # MetadataElicitor
├── acts.rs                # ActElicitor
└── tui_dialog.rs          # TUI implementation
```

## Dependencies

**Core:**
- `async-trait` - Async trait support
- `derive_builder` - Builder pattern
- `derive-getters` - Field accessors
- `serde` - Serialization

**TUI:**
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal control

**Internal:**
- `botticelli_mcp` - NarrativeHelper utilities
- `botticelli_narrative` - Narrative types
- `botticelli_error` - Error handling

## Performance Considerations

- **Minimal Cloning**: Messages cloned only for rendering
- **Lazy Validation**: Only when explicitly requested
- **Efficient TOML Gen**: Direct string building
- **No Blocking**: All I/O is async

## Summary

The elicitation system provides a production-ready, extensible foundation for interactive narrative creation. The trait-based architecture ensures platform independence while maintaining type safety and user-friendly error handling. Integration with existing Botticelli infrastructure (NarrativeHelper, validation) ensures consistency and code reuse.

**Status**: Ready for production use and further extension.
