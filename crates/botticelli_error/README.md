# botticelli_error

Foundation error types for the Botticelli ecosystem.

## Overview

This crate provides the foundational error types used throughout the Botticelli workspace. It defines error enums and wrapper structs with automatic source location tracking for better debugging.

## Features

- **Location Tracking**: Automatically captures file and line number where errors are created using `#[track_caller]`
- **Modular Design**: Separate error types for different subsystems (HTTP, JSON, Database, Narrative, etc.)
- **Conversion Traits**: Automatic `From` implementations for common external error types
- **Feature Flags**: Optional database error conversions behind `database` feature

## Error Types

### Core Error Types

- `HttpError` - HTTP/network errors (wraps `reqwest` errors)
- `JsonError` - JSON serialization/deserialization errors
- `BackendError` - Generic backend operation errors
- `NotImplementedError` - Placeholder for unimplemented features

### Subsystem Errors

- `DatabaseError` / `DatabaseErrorKind` - Database operations
- `NarrativeError` / `NarrativeErrorKind` - Narrative execution
- `GeminiError` / `GeminiErrorKind` - Gemini API client
- `StorageError` / `StorageErrorKind` - Content storage
- `TuiError` / `TuiErrorKind` - Terminal UI

### Main Error Enum

- `BotticelliError` / `BotticelliErrorKind` - Top-level error aggregation
- `BotticelliResult<T>` - Convenience type alias

## Usage

```rust
use botticelli_error::{NarrativeError, NarrativeErrorKind, BotticelliResult};

// Automatic location tracking
#[track_caller]
fn load_narrative(path: &str) -> Result<Narrative, NarrativeError> {
    // Error captures file and line automatically
    Err(NarrativeError::new(
        NarrativeErrorKind::FileRead(path.to_string())
    ))
}

// Automatic conversion to top-level error
fn process() -> BotticelliResult<()> {
    let narrative = load_narrative("missing.toml")?; // NarrativeError converts automatically
    Ok(())
}
```

## Feature Flags

### `database` (optional)

Enables conversion from diesel and serde_json errors to `DatabaseError`:

```toml
[dependencies]
botticelli_error = { version = "0.2", features = ["database"] }
```

When enabled, provides:
- `From<diesel::result::Error> for DatabaseError`
- `From<diesel::ConnectionError> for DatabaseError`
- `From<serde_json::Error> for DatabaseError`

## Design Philosophy

### Location Tracking Pattern

All error constructors use `#[track_caller]` to automatically capture the file and line where the error originated:

```rust
#[derive(Debug, Clone)]
pub struct DatabaseError {
    pub kind: DatabaseErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl DatabaseError {
    #[track_caller]
    pub fn new(kind: DatabaseErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}
```

### Error Kind Pattern

Separate the error variants (kind) from location tracking (wrapper):

```rust
// Enumeration of error cases
pub enum DatabaseErrorKind {
    Connection(String),
    Query(String),
    NotFound,
}

// Wrapper with location
pub struct DatabaseError {
    pub kind: DatabaseErrorKind,
    pub line: u32,
    pub file: &'static str,
}
```

### Automatic Conversions

Use `derive_more::From` to generate conversion implementations:

```rust
#[derive(Debug, derive_more::From)]
pub enum BotticelliErrorKind {
    #[from(HttpError)]
    Http(HttpError),
    #[from(DatabaseError)]
    Database(DatabaseError),
    // ... other variants
}
```

## Dependencies

- `derive_more` - Derive macro for Display, From, etc.
- `derive-new` - Derive constructors
- `diesel` (optional) - Database error conversions
- `serde_json` (optional) - JSON error conversions

## Version

Current version: 0.2.0

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
