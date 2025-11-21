# Gemini Dependency Issue

## Problem

The `google-generative-ai-rs` dependency (avastmick/google-generative-ai-rs) is **archived** and no longer maintained. This causes issues with model naming:

- Our code requests `gemini-2.0-flash-lite`
- The library's Model enum maps this to `Gemini20FlashLite`
- The API actually needs `gemini-2.0-flash-exp-0827` or similar current model names
- Result: API calls route to wrong/outdated model versions

## Current Workaround

We're using the library's outdated model enum values, which means:
- `gemini-2.5-flash` works (maps correctly)
- `gemini-2.5-flash-lite` routes incorrectly to `gemini-2.0-flash-lite`

## Investigation

Repository: https://github.com/avastmick/google-generative-ai-rs
Status: **ARCHIVED** (as of investigation date)
Open issues: Issue #28 discusses outdated model versions but was never resolved

## Alternatives (Updated 2025)

### Actively Maintained Crates on crates.io:

1. **`gemini_rust`** - Comprehensive Gemini 2.0 API client
   - Domain-organized (generation, embedding, batch, files, safety, tools)
   - Supports text, images, audio generation
   - Function calling and tool integration
   - Recent updates for Gemini 2.0

2. **`gemini_rs`** - Full-featured client
   - Text generation, chat, function calling
   - Safety/content filtering, system instructions
   - Async-ready, well-documented
   - Active maintenance on crates.io

3. **`gemini-client-rs`** (adriftdev) - Convenient client
   - Text generation, function calling
   - Grounding (Google Search capabilities)
   - Modern error handling, modular
   - Active GitHub repository

4. **`gemini-ai`** - Framework-agnostic client
   - Function calling, streaming
   - Community-driven, open to contributions
   - Discussed on Rust forums (late 2024/early 2025)

5. **Fork and maintain** - Fork google-generative-ai-rs and fix ourselves

6. **Direct REST API** - Build our own thin client

## Recommendation

**Primary: Evaluate `gemini_rust`**
- Explicitly supports Gemini 2.0 (most current API version)
- Comprehensive feature set matching our needs
- Shows active development
- Domain-organized structure (easier to navigate)
- Likely has correct model naming

**Fallback: `gemini_rs`**
- Well-documented with good examples
- Active maintenance
- Covers core use cases
- Simpler API if gemini_rust is over-engineered

## Action Items

1. [✅] Test `gemini_rust` implementation
   - ✅ Migrated to gemini-rust 1.5.0
   - ⚠️ **Bug Found**: `Model::Gemini25FlashLite` maps to "gemini-2.0-flash-lite" instead of "gemini-2.5-flash-lite"
   - ✅ Workaround implemented using `Model::Custom("models/gemini-2.5-flash-lite")`
   - ✅ Streaming support working
   - ✅ Function calling compatible
2. [✅] File issue on gemini-rust repository about model naming bug
   - Repository: https://github.com/flachesis/gemini-rust
   - ✅ Searched for existing issues (none found for this specific bug)
   - ✅ Filed issue #49: https://github.com/flachesis/gemini-rust/issues/49
3. [ ] Monitor upstream for fix
4. [ ] Remove workaround when fixed upstream

## Current Status (Updated 2025-11-21)

**Migration Complete**: We've migrated to `gemini-rust 1.5.0`, but discovered a bug in that library.

### Bug Details

- **Affected**: `Model::Gemini25FlashLite` enum variant
- **Expected**: Maps to `"models/gemini-2.5-flash-lite"`
- **Actual**: Maps to `"models/gemini-2.0-flash-lite"`
- **Impact**: Requests route to older 2.0 model instead of current 2.5 model

### Workaround Implemented

In `crates/botticelli_models/src/gemini/client.rs` (line 212):

```rust
// NOTE: gemini-rust 1.5's Model::Gemini25FlashLite incorrectly maps to "gemini-2.0-flash-lite"
// Use Custom variant to get correct 2.5 version until upstream is fixed
"gemini-2.5-flash-lite" => Model::Custom("models/gemini-2.5-flash-lite".to_string()),
```

This workaround ensures the correct model is used while we wait for an upstream fix.
