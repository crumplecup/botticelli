# botticelli_storage Audit Report

## CLAUDE.md Compliance Check

### ✅ Compliant Areas

1. **Module structure**: Only 2 files, simple structure is appropriate
2. **Documentation**: Good module-level and item documentation
3. **Tracing**: Has tracing in filesystem.rs operations
4. **Testing**: Tests in tests/ directory

### ❌ Non-Compliant Issues

#### Critical Issues

1. **lib.rs contains types, traits, impls** (VIOLATION)
   - `MediaStorage` trait (lines 48-122)
   - `MediaMetadata` struct (lines 124-139)
   - `MediaReference` struct (lines 141-161)
   - `MediaType` enum (lines 163-172)
   - Multiple impl blocks for MediaType (lines 174-202)
   - **ACTION**: Move all types/traits/impls to separate modules

2. **Manual Display/FromStr implementations** (VIOLATION)
   - MediaType has manual Display (lines 198-202) and FromStr (lines 185-196)
   - **ACTION**: Use derive_more::Display and derive_more::FromStr

#### Medium Priority

3. **Missing derives on MediaType enum**
   - Should derive PartialOrd, Ord per CLAUDE.md
   - Should derive strum::EnumIter (fieldless enum)
   - **ACTION**: Add missing derives

4. **Missing derives on MediaMetadata struct**
   - Should derive PartialEq, Eq, Hash if possible
   - **ACTION**: Evaluate and add derives

5. **Missing derives on MediaReference struct**
   - Should derive PartialOrd, Ord, Hash if possible
   - **ACTION**: Evaluate and add derives

6. **Missing #[instrument] on public functions**
   - FileSystemStorage::new (line 58) missing #[instrument]
   - **ACTION**: Add #[instrument] to public functions

7. **Module organization suggestion**
   - Create src/media.rs for MediaType, MediaMetadata, MediaReference
   - Create src/storage.rs for MediaStorage trait
   - Keep filesystem.rs as-is
   - lib.rs becomes mod + pub use only

#### Low Priority

8. **Dependency version in Cargo.toml**
   - Verify dependency versions follow CLAUDE.md policy

## Recommended Refactoring Plan

### Step 1: Create new modules

```rust
// src/media.rs - Media type definitions
use derive_more::{Display, FromStr};
use strum::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, FromStr, EnumIter)]
#[display(fmt = "{}", "self.as_str()")]
pub enum MediaType {
    #[display(fmt = "image")]
    Image,
    #[display(fmt = "audio")]
    Audio,
    #[display(fmt = "video")]
    Video,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaMetadata { /* fields */ }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaReference { /* fields */ }

// src/storage.rs - Storage trait
use crate::{MediaMetadata, MediaReference};

#[async_trait::async_trait]
pub trait MediaStorage: Send + Sync { /* methods */ }
```

### Step 2: Update lib.rs

```rust
//! Content-addressable media storage for Botticelli.
//! [module docs...]

mod filesystem;
mod media;
mod storage;

pub use filesystem::FileSystemStorage;
pub use media::{MediaMetadata, MediaReference, MediaType};
pub use storage::MediaStorage;
```

### Step 3: Update filesystem.rs imports

```rust
use crate::{MediaMetadata, MediaReference, MediaStorage, MediaType};
```

### Step 4: Add #[instrument] to FileSystemStorage::new

## Summary

**Total Issues**: 8 (2 critical, 5 medium, 1 low)

The main violations are having types/traits/impls in lib.rs and manual implementations that should use derive_more. The crate is otherwise well-structured.
