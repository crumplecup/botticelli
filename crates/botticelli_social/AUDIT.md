# botticelli_social Audit Report

Date: 2025-11-19

## Summary

Auditing `botticelli_social` against CLAUDE.md standards. The crate contains Discord integration code with ~3200 lines.

## Critical Issues

### 1. ❌ Public Module Declaration (lib.rs:20, discord/mod.rs:53)
- **Issue**: Using `pub mod discord` in lib.rs and `pub mod models` in discord/mod.rs
- **Policy**: "Use private `mod` declarations (not `pub mod`) in both lib.rs and module mod.rs files"
- **Fix**: Change to private `mod` declarations and use `pub use` for re-exports

### 2. ❌ Wildcard Imports (Multiple Files)
- **Issue**: `use diesel::prelude::*` in models files and repository.rs
- **Issue**: `use super::*` in test modules
- **Policy**: "Never use wildcard imports like `use module::*`"
- **Fix**: Replace with explicit imports

### 3. ❌ Missing Tracing Instrumentation
- **Issue**: No `#[instrument]` attributes on any public functions
- **Policy**: "Public functions should use #[instrument] macro for automatic entry/exit logging"
- **Fix**: Add `#[instrument]` to all public functions in client.rs, repository.rs, handler.rs

### 4. ❌ Error Type Not Using derive_more
- **Issue**: DiscordErrorKind has manual Display implementation (error.rs:53-77)
- **Policy**: "Use derive_more to derive Display and Error when appropriate"
- **Fix**: Use `#[derive(derive_more::Display)]` with `#[display(fmt = "...")]` attributes

### 5. ❌ Missing Derives on ErrorKind
- **Issue**: DiscordErrorKind only derives Debug, Clone, PartialEq
- **Policy**: "Error enums should derive: Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash"
- **Fix**: Add missing Eq, PartialOrd, Ord, Hash derives

### 6. ❌ Error Not Using derive_more
- **Issue**: DiscordError has manual Display and Error implementations
- **Policy**: "Use #[derive(derive_more::Display, derive_more::Error)] for error wrapper structs"
- **Fix**: Replace manual impls with derives

## High Priority Issues

### 7. ⚠️ Missing EnumIter on Fieldless Enum
- **Issue**: ChannelType enum (models/channel.rs) is fieldless but doesn't derive EnumIter
- **Policy**: "For enums with no fields, use strum to derive EnumIter"
- **Fix**: Add `#[derive(strum::EnumIter)]` to ChannelType

### 8. ⚠️ Inconsistent Tracing
- **Issue**: Some functions use `tracing::info!` but no structured logging with fields or spans
- **Policy**: "Use structured logging with fields: `debug!(count = items.len(), \"Processing items\")`"
- **Fix**: Add structured fields to all tracing calls

### 9. ⚠️ Missing Documentation on Some Public Items
- **Issue**: Some public functions lack comprehensive documentation
- **Policy**: "All public types, functions, and methods must have documentation"
- **Fix**: Add missing documentation, especially for error variants

## Medium Priority Issues

### 10. 📋 Sparse Tracing Coverage
- **Issue**: Only minimal `info!` calls in client.rs, no tracing in repository operations
- **Policy**: "Use appropriate log levels throughout (trace, debug, info, warn, error)"
- **Fix**: Add comprehensive tracing:
  - `debug!` for function entry/exit (or use `#[instrument]`)
  - `debug!` with fields for database operations (guild_id, channel_id, etc.)
  - `warn!` for unusual conditions
  - `error!` for error paths

### 11. 📋 Missing Feature Documentation
- **Issue**: Public API doesn't document Discord feature requirement consistently
- **Policy**: "Document feature-gated public APIs with a note in the documentation"
- **Fix**: Add "Available with the `discord` feature" to all relevant public items

### 12. 📋 Test Organization
- **Issue**: Test modules use wildcard imports (`use super::*`)
- **Policy**: Tests should use explicit imports from crate-level exports
- **Fix**: Update test imports to `use botticelli_social::discord::{Type1, Type2}`

## Low Priority Issues

### 13. 💡 Commented Out Code
- **Issue**: Processors and commands modules are commented out (discord/mod.rs:54-59, 74-78)
- **Fix**: Either implement and enable, or remove if not needed

### 14. 💡 TODO Comments
- **Issue**: Multiple TODO comments in mod.rs
- **Fix**: Track in separate planning document or issue tracker

## Compliance Checklist

- [ ] All `pub mod` changed to private `mod` with `pub use` re-exports
- [ ] All wildcard imports replaced with explicit imports
- [ ] All public functions instrumented with `#[instrument]`
- [ ] DiscordErrorKind uses derive_more::Display
- [ ] DiscordErrorKind has all required derives (Eq, PartialOrd, Ord, Hash)
- [ ] DiscordError uses derive_more::Display and derive_more::Error
- [ ] ChannelType derives EnumIter
- [ ] Comprehensive tracing with structured fields in repository
- [ ] Feature requirements documented on all public items
- [ ] Test imports use crate-level exports

## Files Requiring Changes

1. `src/lib.rs` - Change `pub mod` to `mod`
2. `src/discord/mod.rs` - Change `pub mod models` to `mod models`, export types
3. `src/discord/error.rs` - Use derive_more for Display/Error, add missing derives
4. `src/discord/client.rs` - Add #[instrument], structured tracing
5. `src/discord/repository.rs` - Replace wildcard imports, add #[instrument], add comprehensive tracing
6. `src/discord/handler.rs` - Add #[instrument]
7. `src/discord/models/channel.rs` - Add EnumIter to ChannelType, replace wildcard import
8. `src/discord/models/*.rs` - Replace wildcard imports in all model files
9. `src/discord/conversions.rs` - Replace wildcard imports in tests
10. `src/discord/json_models.rs` - Replace wildcard imports in tests

## Estimated Effort

- Critical fixes: ~2 hours
- High priority: ~1 hour
- Medium priority: ~2 hours
- Total: ~5 hours of focused refactoring
