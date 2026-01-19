# Elicitation Workspace Expansion Strategy

**Status:** botticelli_core 100% complete, ready for workspace expansion
**Date:** 2026-01-19

## Executive Summary

**elicitation 0.4.1 integration in botticelli_core: COMPLETE SUCCESS** 🎉

- ✅ 24/24 user-facing types derive Elicit
- ✅ serde_json::Value support working
- ✅ chrono::DateTime support working  
- ✅ All blocking issues resolved
- ✅ 100% completionist coverage achieved

**Next goal:** Expand Elicit derives to config types across the workspace (estimated 26+ types).

---

## What We Accomplished in botticelli_core

### Types with Elicit (24 total)

**Conversation & Messages:**
1. Role - System/User/Assistant
2. Input - Multimodal inputs (with serde_json::Value!)
3. Output - Multimodal outputs (with serde_json::Value!)
4. Message - Conversation messages
5. ToolCall - Tool invocations (with serde_json::Value!)
6. ToolDefinition - Tool schemas (with serde_json::Value!)
7. ToolResult - Tool results (with serde_json::Value!)

**Requests & Responses:**
8. GenerateRequest - LLM generation request
9. GenerateResponse - LLM generation response
10. StreamChunk - Streaming chunk
11. TokenUsageData - Token counting

**Enums & Types:**
12. MediaSource - Url/Base64/Binary (tuple variants!)
13. HistoryRetention - Full/Summary/Drop
14. TableFormat - Json/Markdown/Csv
15. StopReason - Why generation stopped
16. FinishReason - Why streaming finished

**Configuration:**
17. BotStats - Bot statistics (with chrono::DateTime!)
18. BotServerConfig - Server config
19. BudgetConfig - Rate limit budget
20. ObservabilityConfig - Observability setup
21. ExporterBackend - Trace exporter (Stdout/Otlp)

**Metadata & Health:**
22. Capabilities - Model capability flags (all bool)
23. HealthStatus - Health status (Healthy/Degraded/Unhealthy)
24. BotState - Bot actor state

### Type Correctly Excluded (1 total)

**ModelMetadata** - Cannot implement Elicitation due to:
- `provider: &'static str` - Compile-time constant
- `max_input_tokens: usize` - Platform-dependent integer

**Why this is good:** Metadata = intrinsic properties, Config = user preferences. Elicitation is for user input, not compile-time constants.

---

## Workspace Expansion Targets

### High Priority: Config-Heavy Crates

#### 1. botticelli_rate_limit (5 config types)

```rust
// All in src/config.rs
pub struct ModelTierConfig { ... }       // Model-specific rate overrides
pub struct TierConfig { ... }            // Tier configuration  
pub struct RateLimitConfig { ... }       // Rate limit config
pub struct ProviderConfig { ... }        // Provider config
pub struct BotticelliConfig { ... }      // Top-level config
```

**Value:** Users can interactively configure rate limits and budgets.

#### 2. botticelli_narrative (4 config types) - STARTED

```rust
// ✅ Already done:
pub struct CarouselConfig { ... }        // Iterative execution (DONE)
pub struct ActConfig { ... }             // Per-act config (DONE)

// TODO:
pub struct TomlActConfig { ... }         // TOML act parser
pub struct ValidationConfig { ... }      // Validation settings
```

**Value:** Interactive narrative creation and configuration.

#### 3. botticelli_chat (5 config types)

```rust
pub struct ChatConfig { ... }            // Main chat config
pub struct ChatAppConfig { ... }         // App-level config
pub struct EnvironmentConfig { ... }     // Environment settings
pub struct McpServerConfig { ... }       // MCP server config
pub struct ConfigBuilder { ... }         // Builder pattern
```

**Value:** Interactive chat app configuration.

#### 4. botticelli_bot (4 config types)

```rust
pub struct BotConfig { ... }             // Main bot config
pub struct GenerationConfig { ... }      // Generation settings
pub struct CurationConfig { ... }        // Content curation
pub struct PostingConfig { ... }         // Posting schedule
```

**Value:** Interactive bot setup and tuning.

#### 5. botticelli_server (2 config types)

```rust
pub struct ServerConfig { ... }          // Server configuration
pub struct DatabaseConfig { ... }        // Database connection
```

**Value:** Interactive server setup.

#### 6. botticelli_mcp_client (3 config types)

```rust
pub struct ExternalServerConfig { ... }  // External MCP servers
pub struct GenerationConfig { ... }      // Generation settings
pub struct RetryConfig { ... }           // Retry policy
```

**Value:** Interactive MCP client configuration.

### Medium Priority: Specialized Crates

#### 7. botticelli_actor (5 config types)

```rust
pub struct ActorCacheConfig { ... }      // Actor cache
pub struct ExecutionConfig { ... }       // Execution settings
pub struct SkillConfig { ... }           // Skill config
pub struct ActorConfig { ... }           // Actor config
pub struct ActorServerConfig { ... }     // Server config
```

#### 8. botticelli_security (2 config types)

```rust
pub struct PermissionConfig { ... }      // Permission settings
pub struct ContentFilterConfig { ... }   // Content filtering
```

### Low Priority: Specialized/Internal

#### 9. botticelli_cache (1 config type)

```rust
pub struct CommandCacheConfig { ... }    // Cache config
```

#### 10. botticelli_models (2 config types)

```rust
// Gemini-specific
pub struct SetupConfig { ... }           // Live API setup
pub struct GenerationConfig { ... }      // Generation config
```

#### 11. botticelli_storage (minimal types)

Mostly traits and impl blocks, few user-facing config types.

#### 12. botticelli_tui (specialized UI)

UI state types, not typically elicited interactively.

---

## Implementation Strategy

### Phase 1: Config Types (High ROI)

**Target:** botticelli_rate_limit, botticelli_chat, botticelli_bot, botticelli_server, botticelli_mcp_client

**Approach:**
1. Add `elicitation = { workspace = true }` to Cargo.toml
2. Add `#[derive(elicitation::Elicit)]` to config structs
3. Verify compilation with `just check <crate>`
4. Test with simple examples

**Estimated effort:** 2-3 hours (straightforward derives)

### Phase 2: Actor & Security Types

**Target:** botticelli_actor, botticelli_security

**Approach:** Same as Phase 1

**Estimated effort:** 1 hour

### Phase 3: Specialized Types

**Target:** botticelli_cache, botticelli_models

**Approach:** Case-by-case evaluation (some may not benefit from Elicit)

**Estimated effort:** 1 hour

---

## Benefits of Workspace Expansion

### 1. Interactive Configuration

Users can generate config files interactively:

```bash
# Instead of:
cp botticelli.example.toml botticelli.toml
vim botticelli.toml  # Manual editing

# Users can:
botticelli config generate
# Walks through interactive prompts for all config values
```

### 2. Type-Safe CLI Tools

```rust
use botticelli_rate_limit::RateLimitConfig;

fn main() -> anyhow::Result<()> {
    let config = RateLimitConfig::elicit()?;
    println!("Generated config: {:#?}", config);
    Ok(())
}
```

### 3. Testing & Debugging

```rust
#[test]
fn test_server_config() -> anyhow::Result<()> {
    // Programmatically generate test configs
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .build()?;
    Ok(())
}
```

### 4. MCP Tool Registration

Types with Elicit can be automatically exposed as MCP tools:

```rust
// Automatically generate tool schemas from Elicit types
register_tool::<RateLimitConfig>("configure_rate_limits");
register_tool::<ChatConfig>("configure_chat");
```

### 5. Narrative Generation

```rust
// Interactive narrative creation
let carousel = CarouselConfig::elicit()?;
let act = ActConfig::elicit()?;
// Generate narrative.toml automatically
```

---

## Technical Considerations

### Types That May Not Work

Some types may have fields that don't implement Elicitation:

1. **&'static str** - Compile-time constants (seen in ModelMetadata)
2. **usize** - Platform-dependent integers
3. **Function pointers** - Not serializable/elicitable
4. **Trait objects** - `dyn Trait` can't be elicited

**Solution:** Skip those fields or use concrete types.

### Feature Gates

Some crates use feature gates extensively. Ensure:
- Elicit derives are behind the same feature gates as their fields
- Feature combinations are tested with `just check-features`

### Import Requirements

All types need trait imports:

```rust
// For enums:
use elicitation::{Prompt, Select};

// For structs:
use elicitation::{Prompt, Survey};
```

The macro will fail without these in scope.

---

## Testing Strategy

### Per-Crate Checklist

For each crate:

1. ✅ Add `elicitation = { workspace = true }` to Cargo.toml
2. ✅ Add derives to config types
3. ✅ Add required trait imports
4. ✅ Run `just check <crate>`
5. ✅ Fix any compilation errors
6. ✅ Test with example (optional)

### Workspace-Level Verification

```bash
# After all crates updated:
just check-all           # Full workspace check
just check-features      # Feature combination testing
just test-package <crate>  # Run tests
```

---

## Success Metrics

### Coverage Goals

- **Phase 1:** 80% of config types (high-value targets)
- **Phase 2:** 90% of config types (including specialized)
- **Phase 3:** 95%+ (remaining applicable types)

### Quality Metrics

- ✅ Zero compilation errors
- ✅ Zero clippy warnings
- ✅ All tests passing
- ✅ Documentation updated

---

## Risks & Mitigation

### Risk: Breaking Changes

**Impact:** Crates depend on each other, changes could cascade

**Mitigation:**
- Work one crate at a time
- Test after each crate
- Commit frequently

### Risk: Feature Gate Complexity

**Impact:** Some types behind features may break with Elicit

**Mitigation:**
- Use same feature gates on derives
- Test with `just check-features`
- Document feature requirements

### Risk: Type Constraints

**Impact:** Some types can't have Elicit (like ModelMetadata)

**Mitigation:**
- Document why types are excluded
- Provide alternatives (like Capabilities for ModelMetadata)
- Accept that not ALL types can/should have Elicit

---

## Conclusion

**elicitation 0.4.1 integration is a complete success** in botticelli_core. The path forward for workspace expansion is clear:

1. **High ROI:** Config types in rate_limit, chat, bot, server, mcp_client
2. **Medium ROI:** Actor and security types
3. **Low ROI:** Specialized internal types

**Estimated total effort:** 4-5 hours for 80% coverage, 6-8 hours for 95%+ coverage.

**Impact:** Transforms botticelli from "TOML-configured" to "interactively configurable" with type-safe CLI tools, MCP tool registration, and narrative generation.

The foundation is solid. Time to expand! 🚀
