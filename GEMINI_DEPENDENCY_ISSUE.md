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

1. [ ] Test `gemini_rust` implementation
   - Verify correct model naming (especially gemini-2.5-flash-lite)
   - Test streaming support (critical for our use case)
   - Verify function calling compatibility
   - Compare API surface with our current wrapper
2. [ ] If gemini_rust unsuitable, test `gemini_rs`
3. [ ] Create migration plan for chosen library
4. [ ] File issue on google-generative-ai-rs repo about model naming bug
5. [ ] Update CLAUDE.md with dependency maintenance best practices

## Temporary Workaround

Until migration, document the model naming issue and warn users:

```rust
// WORKAROUND: google-generative-ai-rs has outdated model mappings
// gemini-2.5-flash-lite incorrectly routes to gemini-2.0-flash-lite
// This is a known issue - see GEMINI_DEPENDENCY_ISSUE.md
```
