# Core Crate Refactoring Plan

## Problem

The `botticelli_core/src/lib.rs` file contains all type definitions, violating the principle that `lib.rs` should only contain `mod` and `pub use` statements (similar to `mod.rs`).

## Current Structure (146 lines in lib.rs)

All types are defined directly in `lib.rs`:
- `Role` enum
- `Input` enum (multimodal input types)
- `MediaSource` enum
- `Output` enum (multimodal output types)
- `ToolCall` struct
- `Message` struct
- `GenerateRequest` struct
- `GenerateResponse` struct

## Proposed Module Organization

Split into focused modules by responsibility:

```
crates/botticelli_core/src/
├── lib.rs              # Only mod declarations and pub use exports
├── role.rs             # Role enum
├── media.rs            # MediaSource enum
├── input.rs            # Input enum
├── output.rs           # Output enum + ToolCall struct
├── message.rs          # Message struct
└── request.rs          # GenerateRequest and GenerateResponse
```

### Module Responsibilities

**`role.rs`**
- `Role` enum (System, User, Assistant)
- Simple, standalone type with no dependencies

**`media.rs`**
- `MediaSource` enum (Url, Base64, Binary)
- Shared by both Input and Output
- Should be imported by input.rs and output.rs

**`input.rs`**
- `Input` enum (Text, Image, Audio, Video, Document)
- Imports: `MediaSource` from `crate::MediaSource`

**`output.rs`**
- `Output` enum (Text, Image, Audio, Video, Embedding, Json, ToolCalls)
- `ToolCall` struct (logically belongs with Output since it's only used in Output::ToolCalls)
- Imports: `MediaSource` from `crate::MediaSource` (for future use)

**`message.rs`**
- `Message` struct
- Imports: `Role`, `Input` from crate level

**`request.rs`**
- `GenerateRequest` struct
- `GenerateResponse` struct
- Imports: `Message`, `Output` from crate level

**`lib.rs`** (final state)
```rust
//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

mod role;
mod media;
mod input;
mod output;
mod message;
mod request;

pub use role::Role;
pub use media::MediaSource;
pub use input::Input;
pub use output::{Output, ToolCall};
pub use message::Message;
pub use request::{GenerateRequest, GenerateResponse};
```

## Implementation Steps

### Step 1: Create module files
1. Create `role.rs` with `Role` enum
2. Create `media.rs` with `MediaSource` enum
3. Create `input.rs` with `Input` enum
4. Create `output.rs` with `Output` enum and `ToolCall` struct
5. Create `message.rs` with `Message` struct
6. Create `request.rs` with `GenerateRequest` and `GenerateResponse` structs

### Step 2: Update lib.rs
- Remove all type definitions
- Add private `mod` declarations
- Add `pub use` statements to export all public types at crate level

### Step 3: Verify imports
- Each module should import types using `use crate::{Type}` syntax
- Never use module paths like `use crate::module::Type`

### Step 4: Test and commit
1. Run `cargo check --all-features`
2. Run `cargo test --all-features`
3. Run `cargo clippy --all-features --all-targets`
4. Commit with message: "refactor(core): move types from lib.rs to focused modules"

## Benefits

1. **Cleaner organization** - Each module has a single, clear responsibility
2. **Easier navigation** - Types are grouped logically by function
3. **Better scalability** - Easy to add new types to appropriate modules
4. **Consistent pattern** - Matches the established pattern for other crates
5. **Single import path** - All types imported via `use crate::{Type}`

## Notes

- This is a pure refactoring with no functional changes
- All types remain publicly exported at the crate root
- Import paths for users of this crate remain unchanged
- This sets a pattern for other crates in the workspace
