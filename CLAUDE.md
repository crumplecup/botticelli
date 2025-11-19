# Claude Project Instructions

## Workflow

- Use a planning document (.md file) to break the current task into steps and discuss the implementation, so the developer can review the implementation plan prior to writing the actual code. Blocks of code with comments are nice, but be sure to couch these blocks with a lot of interpretive context, for consumption by a human reader.
- For each step in the planning document:
  - After generating new code and correcting any cargo check errors and warnings:
    1. Run cargo test and clear **all** errors, including any pre-existing failures.
    2. Run cargo clippy and clear **all** warnings, including any pre-existing warnings.
    3. Run cargo test --doc and ensure all doctests pass.
    4. Commit the changes to git using best practices for code auditing.
    5. Push the changes to their respective github branch.
- After each step is complete, update the planning document, so it can serve as a user guide when all the tracked tasks are complete.
- Avoid running cargo clean often, to take advantage of incremental compilation during development.

### Critical Rule: Fix Everything

- **NEVER ignore test failures, clippy warnings, or errors because they seem unrelated to your current work.**
- **ALWAYS fix ALL issues before committing**, even if they appear unrelated.
- Pre-existing failures must be fixed before your changes are committed.
- The codebase must always be in a clean state: all tests passing, zero clippy warnings, zero errors, all doctests compiling.
- If you discover an unrelated issue:
  1. Fix it immediately as part of your current work, OR
  2. Create a separate commit to fix it before your main changes
- Rationale: "Unrelated" issues may actually be dependencies, side effects, or test environment problems that affect your work. Leaving them unfixed creates technical debt and obscures real issues.

### Why This Matters: Common Pitfalls

**Example 1: Export changes causing doctest failures**

- You add a new type to crate-level exports
- Existing doctests use old module-path imports (`use crate::module::Type`)
- These appear "unrelated" but are actually incomplete work from your change
- **Fix:** Update all doctests to use crate-level imports when you change exports

**Example 2: Name conflicts from new exports**

- You export all types from a module at crate root
- A type name conflicts with an existing export (e.g., `ToolCall` from two modules)
- Tests or doctests that depend on the old import break
- **Fix:** Rename types to be unique before exporting (e.g., `LiveToolCall`)

**Example 3: Missing feature gates**

- You enable a feature for your work
- Existing tests depend on that feature but lack `#![cfg(feature = "...")]`
- Without the feature, compilation fails for other users
- **Fix:** Add appropriate feature gates to all affected tests

**Example 4: Incomplete examples in doctests**

- You add a required field to a struct
- Doctest examples don't include the new field
- Doctests fail to compile even though "your code works"
- **Fix:** Update all doctests that construct the modified struct

### Verification Checklist Before Committing

Run these commands and ensure ALL pass with zero errors/warnings:

```bash
# 1. Check compilation (no features needed for basic check)
cargo check

# 2. Run LOCAL tests only (fast, no API keys required)
cargo test --lib --tests

# 3. Run doctests
cargo test --doc

# 4. Run clippy
cargo clippy --all-targets

# 5. For markdown changes
markdownlint-cli2 "**/*.md" "#target" "#node_modules"
```

If any command fails, **fix it before committing**. No exceptions.

### API Testing (Optional)

API tests consume rate-limited resources and require API keys. Only run these:
1. When explicitly requested by the user
2. Before merging to another branch
3. For targeted integration testing

```bash
# Requires GEMINI_API_KEY environment variable
cargo test --features gemini,api

# Run all API tests (expensive!)
cargo test --all-features
```

## Linting

- When running any linter (e.g. clippy or markdownlint), rather than deny all warnings, let them complete so you can fix them all in a single pass.
- After editing a markdown file, run `markdownlint-cli2` (not `markdownlint`) and either fix the error or add an exception, as appropriate in the context.
- Do not run cargo clippy or cargo test after changes to markdown files, as they don't affect the Rust code.

## API structure

- In lib.rs, export the visibility of all types at the root level with pub use statements.
  - Keep the mod statements private so there is only one way for users to import the type.
  - In modules, import types from the crate level with use crate::{type1, type2} statements.

## Derive Policies

- Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible.
- Use derive_more to derive Display, FromStr, From, Deref, DerefMut, AsRef, and AsMut when appropriate.
- For enums with no fields, use strum to derive EnumIter.

### Exception: Error Types

Error types follow different derive policies due to their unique semantics:

**ErrorKind enums** (specific error conditions):
- MUST derive: `Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display`
- Rationale: Full derives enable comparison, sorting, and collection operations for error handling

**Wrapper error structs** (error + location tracking):
- MUST derive: `Debug, Clone, derive_more::Display, derive_more::Error`
- DO NOT derive: `PartialEq, Eq, Hash, PartialOrd, Ord`
- Rationale: Location tracking makes comparison confusing (same error at different lines would be unequal)
- Follows `std::io::Error` precedent (also not PartialEq/Hash)

**Examples:**
```rust
// ErrorKind enum - full derives for comparison/sorting
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum StorageErrorKind {
    #[display("Media not found: {}", _0)]
    NotFound(String),
}

// Wrapper struct - minimal derives, location tracking
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Storage Error: {} at line {} in {}", kind, line, file)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub line: u32,
    pub file: &'static str,
}
```

**CRITICAL AUDIT REQUIREMENT:**
When auditing error types, you MUST check:
1. ✅ ErrorKind enums use `derive_more::Display` (NOT manual `impl Display`)
2. ✅ Wrapper structs use `derive_more::Display` and `derive_more::Error` (NOT manual impls)
3. ✅ No `impl std::fmt::Display for ErrorKind` blocks exist
4. ✅ No `impl std::error::Error for WrapperError` blocks exist

**If you find manual implementations:**
- Replace them with derive_more macros immediately
- This is NOT optional - it's a codebase standard violation
- Update CLAUDE.md if the guidance was insufficient

## Serialization

- Derive `Serialize` and `Deserialize` for types that need to be persisted or transmitted (project state, configuration, etc.).
- Use `#[serde(skip)]` for fields that should not be serialized (runtime state, caches, UI state, texture handles).
- Use `#[serde(default)]` for fields that should use their `Default` value when missing during deserialization.
- Use `#[serde(default = "function_name")]` to specify a custom default function for a field.
- Use `#[serde(rename = "name")]` when the serialized field name should differ from the Rust field name.
- Group related `#[serde(skip)]` attributes with comments explaining why they're not serialized (e.g., "// Runtime state (not serialized)").
- For complex serialization needs, implement custom `Serialize`/`Deserialize` instead of using derives.

## Feature Flags

- Use `#[cfg(feature = "feature-name")]` to conditionally compile code based on features.
- Document feature-gated public APIs with a note in the documentation: `/// Available with the`feature-name`feature.`
- Available features:
  - `backend-eframe` - eframe/wgpu rendering backend (enabled by default)
  - `text-detection` - OpenCV-based text detection
  - `logo-detection` - OpenCV-based logo detection
  - `ocr` - Tesseract-based OCR text extraction
  - `dev` - Enables all optional features for development
- When adding new feature-gated code, ensure the crate still compiles with only default features.
- Use `cargo check --no-default-features` to verify the crate works without optional features.
- Use `cargo check --all-features` to verify all features compile together.

## Dependency Versions

In Cargo.toml:

- If the crate is >=1.0, use just the major version number "x".
- If >=0.1.0, use the major and minor "x.y".
- If <0.1.0, use the full "x.y.z".
- Before testing, run `cargo update` to update Cargo.lock with the latest compatible versions.

## Documentation

- Use `///` for item documentation (functions, structs, enums, fields, methods).
- Use `//!` for module-level documentation at the top of files.
- All public types, functions, and methods must have documentation (enforced by `#![warn(missing_docs)]`).
- Document:
  - **What** the item does (concise first line)
  - **Why** it exists or when to use it (for non-obvious cases)
  - **Parameters and returns** for functions (when not obvious from types)
  - **Examples** for complex APIs or non-obvious usage
  - **Errors** that can be returned (for Result-returning functions)
- Keep documentation concise but informative - avoid stating the obvious from the signature.

## Logging and Tracing

**MANDATORY:** Comprehensive tracing instrumentation is a baseline requirement for all code, not a nice-to-have feature.

### Core Principles

- Use the `tracing` crate for all logging (never `println!` in library code).
- **Every public function MUST have tracing instrumentation** - this is non-negotiable.
- Observability is critical for debugging, performance monitoring, audit trails, and error tracking.
- Missing instrumentation is a defect that must be caught in audits and fixed before merging.

### Instrumentation Requirements

**All public functions must:**
1. Use `#[instrument]` macro for automatic span creation
2. Skip large parameters with `skip(connection, data)` to avoid log bloat
3. Include relevant fields for context (IDs, counts, status values)
4. Emit debug/info/warn/error events at key decision points
5. Log SQL queries at debug level for database operations
6. Track errors with full context before returning

**Example:**
```rust
#[instrument(skip(conn), fields(table_name, limit))]
pub fn list_content(
    conn: &mut PgConnection,
    table_name: &str,
    limit: i64,
) -> DatabaseResult<Vec<ContentRow>> {
    debug!("Querying content table");
    // ... implementation ...
    if let Err(e) = result {
        error!("Failed to query content: {}", e);
        return Err(e.into());
    }
    debug!(count = rows.len(), "Retrieved content rows");
    Ok(rows)
}
```

### Log Levels

Choose appropriate log levels:

- `trace!()` - Very detailed, fine-grained information (loop iterations, individual calculations)
- `debug!()` - General debugging information (function entry/exit, state changes, SQL queries)
- `info!()` - Important runtime information (initialization, major events, table creation)
- `warn!()` - Warnings about unusual but recoverable conditions (missing columns, deprecated usage)
- `error!()` - Errors that should be investigated (query failures, connection errors)

### Structured Logging

- Use structured logging with fields: `debug!(count = items.len(), "Processing items")`
- Use `?` prefix for Debug formatting in field values: `debug!(value = ?self.field())`
- Use `%` prefix for Display formatting: `info!(table = %table_name, "Creating table")`
- Skip large data structures in spans: `#[instrument(skip(connection, large_json))]`

### Span Naming Convention

- Pattern: `module_name.function_name`
- Examples: `database.establish_connection`, `content_management.list_content`
- Consistent naming enables filtering and tracing request flows

### Audit Checklist

When auditing code for tracing compliance:
- ✅ Every public function has `#[instrument]`
- ✅ Span fields include relevant context (IDs, counts, names)
- ✅ Large structures (connections, JSON, schemas) are skipped
- ✅ Key operations emit debug/info events
- ✅ Errors emit error events with context before returning
- ✅ SQL queries logged at debug level
- ✅ Span names follow `module.function` convention

### Binary Applications

- Binary applications can use `println!` for user-facing output
- Use `tracing` for all diagnostics, debugging, and operational logging
- Configure tracing subscriber in main() with appropriate filtering

## Testing

- **Centralized test location**: Do not place `#[cfg(test)] mod tests` blocks in source files. All tests must be in the `tests/` directory.
- **Test file naming**: Name test files descriptively after what they test: `{module}_{component}_test.rs`
  - Examples: `storage_filesystem_test.rs`, `narrative_in_memory_repository_test.rs`, `rate_limit_tiers_test.rs`
- **Test organization**: Group related tests in the same file, use clear test function names that describe what is being tested.
- **Import patterns**: Import from crate-level exports (`use botticelli::Type`) not module paths (`use botticelli::module::Type`)
- **Test independence**: Each test should be self-contained and not depend on other tests.
- **Use test utilities**: Create helper functions within test files to reduce duplication (e.g., `create_test_execution()`).

### API Rate Limit Conservation in Tests

- Tests that make API calls should minimize ALL rate limit consumption (TPM, RPM, TPD, RPD).
- Design tests to use the **minimum necessary** to validate behavior:
  - **Tokens**: Use minimal prompts and low max_tokens (e.g., 10)
  - **Requests**: Use fewest API calls possible (e.g., 1-3 requests, not 20+)
  - **Time**: Keep test duration short to avoid extended quota usage
- Mark API-consuming tests with `#[cfg_attr(not(feature = "api"), ignore)]` to require explicit opt-in
  - Run with: `cargo test --features gemini,api` (or other provider + api)
  - The `api` feature is an empty marker flag (`api = []`) that gates tests consuming API tokens
- **Do NOT use `#[ignore]` for API tests** - `#[ignore]` is reserved for:
  - Tests for features not yet implemented
  - Broken tests that need fixing
  - Tests temporarily disabled during refactoring
- Follow existing patterns in the codebase (see `gemini_streaming_test.rs` for examples)
- If extensive testing is needed, consider:
  - Mocking API responses instead of real calls
  - Creating separate "expensive" test suite with clear warnings
  - Using local test doubles or fake implementations
- Use environment variables to manage API keys securely during testing

## Error Handling

### Critical Rule: ALWAYS Use derive_more for Display and Error

**MANDATORY:** All error types MUST use `derive_more::Display` and `derive_more::Error`. Manual implementations are NOT allowed.

**Why this is critical:**
- Manual Display/Error implementations are boilerplate code (reduces 200+ lines)
- More declarative - error messages visible directly in attributes
- Easier to maintain - change one attribute vs entire impl block
- Consistent patterns across all error types
- Prevents audit failures and technical debt

**Audit requirement:** When auditing code, ALWAYS check for manual `impl std::fmt::Display` and `impl std::error::Error` - these are violations.

### Error Type Patterns

Use unique error types for different sources to create encapsulation around error conditions for easier isolation.

#### Pattern 1: Simple Error Structs (message + location)

For errors that just need a message and location tracking:

```rust
/// HTTP error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("HTTP Error: {} at line {} in {}", message, line, file)]
pub struct HttpError {
    pub message: String,
    pub line: u32,
    pub file: &'static str,
}

impl HttpError {
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}
```

**Key requirements:**
- MUST use `derive_more::Display` with `#[display(...)]` attribute
- MUST use `derive_more::Error`
- MUST use `#[track_caller]` on constructor
- Error struct `file` fields MUST use `&'static str` (not `String`)

#### Pattern 2: ErrorKind Enum (specific error conditions)

For errors with multiple specific conditions:

```rust
/// Specific storage error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum StorageErrorKind {
    /// Media not found
    #[display("Media not found: {}", _0)]
    NotFound(String),
    
    /// Permission denied
    #[display("Permission denied: {}", _0)]
    PermissionDenied(String),
    
    /// Hash mismatch with named fields
    #[display("Content hash mismatch: expected {}, got {}", expected, actual)]
    HashMismatch {
        expected: String,
        actual: String,
    },
}
```

**Key requirements:**
- MUST use `derive_more::Display`
- MUST have `#[display(...)]` attribute on EVERY variant
- Use `_0`, `_1` for tuple variant fields
- Use field names for struct variant fields
- ErrorKind enums do NOT need derive_more::Error (wrapper handles that)

#### Pattern 3: Wrapper Error Struct (ErrorKind + location)

Wraps an ErrorKind enum with location tracking:

```rust
/// Storage error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Storage Error: {} at line {} in {}", kind, line, file)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl StorageError {
    #[track_caller]
    pub fn new(kind: StorageErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}
```

**Key requirements:**
- MUST use `derive_more::Display` with format including kind
- MUST use `derive_more::Error`
- MUST use `#[track_caller]` on constructor

#### Pattern 4: Crate-Level Error Aggregation

Top-level error that aggregates all module errors:

```rust
/// Crate-level error enum.
#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum CrateErrorKind {
    #[from(HttpError)]
    Http(HttpError),
    
    #[from(StorageError)]
    Storage(StorageError),
    
    // ... other variants
}

/// Crate-level error wrapper.
#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("Crate Error: {}", _0)]
pub struct CrateError(Box<CrateErrorKind>);

impl CrateError {
    pub fn new(kind: CrateErrorKind) -> Self {
        Self(Box::new(kind))
    }
    
    pub fn kind(&self) -> &CrateErrorKind {
        &self.0
    }
}

// Generic blanket From implementation
impl<T> From<T> for CrateError
where
    T: Into<CrateErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}
```

**Key requirements:**
- MUST use `derive_more::From` on CrateErrorKind
- MUST use explicit `#[from(ErrorType)]` on each variant
- MUST use `derive_more::Display` on both kinds and wrapper
- MUST use `derive_more::Error` on both kinds and wrapper
- CrateErrorKind Display forwards to inner errors (automatic with derive_more)
- CrateError Display wraps with context

### External Error Conversions

For external error types (e.g., `reqwest::Error`, `serde_json::Error`):

```rust
impl From<std::io::Error> for CrateErrorKind {
    fn from(err: std::io::Error) -> Self {
        // Wrap external error with location tracking
        CrateErrorKind::Storage(StorageError::new(
            StorageErrorKind::Io(err.to_string())
        ))
    }
}
```

### Return Type Guidelines

- If a function returns a single unique error type, use that type
- If a function can return multiple error types, use the crate-level error type
- Use `?` operator for automatic conversions via From implementations

### Audit Checklist for Error Handling

When reviewing or auditing code, verify:

1. ✅ NO manual `impl std::fmt::Display` for error types
2. ✅ NO manual `impl std::error::Error` for error types
3. ✅ ALL error structs use `derive_more::Display` with `#[display(...)]`
4. ✅ ALL error structs use `derive_more::Error`
5. ✅ ALL ErrorKind enum variants have `#[display(...)]` attributes
6. ✅ ALL error constructors use `#[track_caller]`
7. ✅ ALL crate-level error enums use `derive_more::From`
8. ✅ ALL from attributes are explicit: `#[from(ErrorType)]`

**If you find manual Display/Error implementations during audit:** Flag as critical issue requiring immediate refactoring.

### Reference Implementation

See `crates/botticelli_error` for a complete, production-ready reference implementation of all error patterns using derive_more.

## Module Organization

### Module Structure

- When a module file exceeds ~500-1000 lines, consider splitting it into a module directory with focused submodules organized by responsibility (e.g., core, io, tools, rendering).
- Create a mod.rs file to re-export the public API and keep internal organization private.
- Only put mod and export statements in the mod.rs file, not types, traits or impl blocks.

### Visibility and Export Patterns

**Module declarations:**

- Use private `mod` declarations (not `pub mod`) in both lib.rs and module mod.rs files
- Keep internal module structure hidden from external users

```rust
// src/lib.rs or src/mymodule/mod.rs
mod error;           // Private module
mod models;          // Private module
mod internal_helper; // Private module
```

**Module-level exports (mod.rs):**

- Re-export public types from submodules using `pub use`
- This creates the public API for the module

```rust
// src/mymodule/mod.rs
mod error;
mod models;
mod helper;

pub use error::{MyError, MyErrorKind, MyResult};
pub use models::{Model, NewModel, ModelRow};
// helper module stays private, not exported
```

**Crate-level exports (lib.rs):**

- Re-export ALL public types from all modules at the crate root
- This ensures a single, consistent import path throughout the codebase

```rust
// src/lib.rs
mod mymodule;

pub use mymodule::{
    Model, MyError, MyErrorKind, MyResult, NewModel, ModelRow,
};
```

### Import Patterns

**For crate-level types (exported from lib.rs):**

- Always use `use crate::{Type1, Type2}` syntax
- Never use module paths like `crate::module::Type`
- Never use `super::` paths
- Never use wildcard imports like `use module::*`

```rust
// ✅ GOOD: Import from crate root
use crate::{Model, MyError, MyResult};

// ❌ BAD: Module path imports
use crate::mymodule::Model;

// ❌ BAD: Super paths
use super::models::Model;

// ❌ BAD: Wildcard imports
use crate::mymodule::*;
```

**For internal module helpers (not exported at crate level):**

- Use explicit module paths: `use crate::module::helper::function`
- For schema tables or module-private items: `use crate::module::schema::table_name`

```rust
// ✅ GOOD: Internal helper functions
use crate::database::schema::{users, posts};
use crate::database::conversions::{row_to_model, model_to_row};
```

### Complete Example

```rust
// src/database/mod.rs
mod error;
mod models;
mod conversions;  // Internal helpers
mod schema;       // Diesel schema

pub use error::{DatabaseError, DatabaseErrorKind, DatabaseResult};
pub use models::{User, NewUser, UserRow};

// src/lib.rs
mod database;

pub use database::{
    DatabaseError, DatabaseErrorKind, DatabaseResult,
    User, NewUser, UserRow,
};

// src/database/conversions.rs
use crate::{User, UserRow, DatabaseResult};  // Crate-level types
use crate::database::schema::users;          // Internal schema

pub fn row_to_user(row: UserRow) -> DatabaseResult<User> {
    // ...
}

// src/database/repository.rs
use crate::{User, UserRow, DatabaseResult};  // Crate-level types
use crate::database::conversions::row_to_user;  // Internal helper
use crate::database::schema::users;             // Internal schema
```

### Benefits

This pattern provides:

1. **Single import path** - All types imported as `use crate::{Type}`
2. **No ambiguity** - Only one way to import each type
3. **Clean public API** - Internal module structure is hidden
4. **Easier refactoring** - Module reorganization doesn't break imports
5. **Better IDE support** - Auto-completion works consistently

### Cross-Module Communication

- Add helper methods (setters, mut accessors) to core structs for clean cross-module communication instead of directly accessing fields.

## Workspace Organization

When working with Cargo workspaces, each crate must follow the same organizational principles as a standalone crate.

### lib.rs Structure in Workspace Crates

**Critical Rule:** `lib.rs` should ONLY contain `mod` declarations and `pub use` exports, never type definitions, trait definitions, or impl blocks.

```rust
// ❌ BAD: Types defined in lib.rs
// src/lib.rs
pub struct MyType {
    field: String,
}

pub enum MyEnum {
    Variant1,
    Variant2,
}

// ✅ GOOD: Only mod and pub use statements
// src/lib.rs
//! Crate documentation goes here.

mod types;
mod enums;

pub use types::MyType;
pub use enums::MyEnum;
```

### Organizing Small Crates

Even small crates (100-200 lines) should separate concerns into modules:

```
crates/my_crate/src/
├── lib.rs              # Only mod declarations and pub use exports
├── role.rs             # Role-related types
├── input.rs            # Input types
├── output.rs           # Output types
└── request.rs          # Request/Response types
```

**lib.rs structure:**

```rust
//! Crate-level documentation describing the crate's purpose.

mod role;
mod input;
mod output;
mod request;

pub use role::Role;
pub use input::Input;
pub use output::{Output, ToolCall};
pub use request::{Request, Response};
```

### Module Responsibilities

Each module should have a single, clear responsibility:

- **Single type per module** (simple case) - One enum or struct
- **Related types per module** (common case) - Types that work together (e.g., `Output` enum + `ToolCall` struct)
- **Shared dependencies** - Types used by multiple other modules (e.g., `MediaSource` used by both `Input` and `Output`)

### Import Patterns in Workspace Crates

The same import rules apply within each workspace crate:

```rust
// In any module within the crate
use crate::{Type1, Type2};  // ✅ Import from crate root

// NOT these:
use crate::module::Type1;   // ❌ Never use module paths
use super::Type1;            // ❌ Never use super paths
```

### Cross-Crate Dependencies

**CRITICAL RULE: NO RE-EXPORTS ACROSS WORKSPACE CRATES**

In a workspace, **DO NOT re-export types from one crate into another crate's public API**. This creates multiple import paths for the same type, violating the "single import path" principle.

#### ❌ WRONG: Re-exporting dependency types

```rust
// crates/botticelli_database/src/lib.rs
pub use botticelli_error::{DatabaseError, DatabaseErrorKind};  // ❌ DON'T DO THIS

// Now users can import the same type two ways:
use botticelli_error::DatabaseError;     // Original source
use botticelli_database::DatabaseError;  // Re-exported (ambiguous!)
```

#### ✅ CORRECT: Import dependency types directly

```rust
// crates/botticelli_database/src/lib.rs
// NO re-exports of dependency types

// Users of botticelli_database import what they need:
// In their code:
use botticelli_database::NarrativeRepository;  // Database's own types
use botticelli_error::DatabaseError;           // Error types from error crate
```

#### Rationale

**Problem with re-exports:**
- Creates ambiguity: `use crate_a::Type` vs `use crate_b::Type` (same type!)
- Breaks "single import path" principle
- Makes refactoring difficult (breaking changes when removing re-exports)
- IDE confusion (multiple auto-import options)
- Unclear ownership (which crate "owns" the type?)

**Solution:**
- Each type has exactly ONE canonical import path
- Users import from the type's home crate
- Transitive dependencies are explicit in user code

#### Type Aliases for Convenience

If you need a crate-specific Result type, use a type alias (not re-export):

```rust
// crates/botticelli_database/src/lib.rs
use botticelli_error::DatabaseError;

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;  // ✅ Type alias OK
```

Users get the convenience:
```rust
use botticelli_database::DatabaseResult;  // Convenience alias
use botticelli_error::DatabaseError;      // Actual error type
```

#### Exception: Foundation Crates

Only the top-level binary/library crate (e.g., `botticelli`) may re-export types from workspace dependencies for user convenience:

```rust
// crates/botticelli/src/lib.rs (top-level public API)
pub use botticelli_core::{Role, Input, Output};
pub use botticelli_error::{BotticelliError, BotticelliResult};
pub use botticelli_interface::{GenerationBackend, NarrativeRepository};

// This is the ONLY crate that re-exports.
// Internal workspace crates NEVER re-export from each other.
```

### Refactoring Checklist for Existing Crates

When cleaning up a crate with types in lib.rs:

1. **Identify type groups** - Group related types by responsibility
2. **Create module files** - One file per logical group
3. **Move types** - Cut types from lib.rs, paste into module files
4. **Add imports** - Each module imports from `crate::{...}`
5. **Update lib.rs** - Replace type definitions with `mod` and `pub use`
6. **Verify** - Run `cargo check`, `cargo test`, `cargo clippy`
7. **Commit** - Use conventional commit message (e.g., `refactor(crate): organize types into modules`)

### Example: Refactoring a Small Core Crate

**Before:**

```rust
// crates/my_core/src/lib.rs (146 lines)
pub enum Role { System, User, Assistant }
pub struct Message { pub role: Role, pub content: String }
pub struct Request { pub messages: Vec<Message> }
pub struct Response { pub output: String }
```

**After:**

```rust
// crates/my_core/src/lib.rs (12 lines)
mod role;
mod message;
mod request;

pub use role::Role;
pub use message::Message;
pub use request::{Request, Response};

// crates/my_core/src/role.rs
pub enum Role { System, User, Assistant }

// crates/my_core/src/message.rs
use crate::Role;

pub struct Message {
    pub role: Role,
    pub content: String,
}

// crates/my_core/src/request.rs
use crate::Message;

pub struct Request {
    pub messages: Vec<Message>,
}

pub struct Response {
    pub output: String,
}
```

### Benefits for Workspaces

1. **Consistency** - All crates follow the same organizational pattern
2. **Discoverability** - Easy to find types across multiple crates
3. **Maintainability** - Changes to one crate don't require understanding others' internal structure
4. **Scalability** - Easy to grow crates without restructuring
5. **Onboarding** - New contributors learn one pattern that applies everywhere

## Common Refactoring Patterns

- **State Machine Extraction**: When multiple boolean flags represent mutually exclusive states, extract them into an enum state machine to prevent invalid state combinations.
- **Borrow Checker**: When encountering borrow checker errors with simultaneous immutable and mutable borrows, extract needed values before taking mutable references (e.g., `let value = *self.field(); /* then mutably borrow */`).

## Unsafe

- Use the forbid unsafe lint at the top level of lib.rs to prevent unsafe code.
