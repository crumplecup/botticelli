# Elicitation Dependency Analysis
## External Types Blocking Completionist Elicit Derives

## Executive Summary

To derive `Elicit` on all botticelli_core types, we need `Elicitation` implementations for **2 external types**:

1. ✅ **std types** - Already handled by elicitation crate
2. 🚧 **serde_json::Value** - Plan exists (v0.2.2)
3. 🤔 **chrono::DateTime** - Needs implementation strategy

**Impact:** With these 2 implementations, ~30 botticelli_core types can derive `Elicit`.

---

## Analysis: Types Used in botticelli_core

### Primitive & std Types (Already Supported ✅)

These work out of the box with elicitation 0.2.1:

- **Primitives:** `bool`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`, `usize`, `String`
- **std Collections:** `Vec<T>`, `Option<T>`, `HashMap<K, V>`
- **std Time:** `Duration`

### Botticelli Types (Work with Derives ✅)

Our own types that can derive `Elicit`:

**Simple enums (unit variants):**
- ✅ `Role` - System, User, Assistant
- ✅ `HistoryRetention` - Full, Summary, Drop
- ✅ `TableFormat` - Json, Markdown, Csv
- ✅ `StopReason` - EndTurn, MaxTokens, ToolUse, etc.
- ✅ `ExecutionStatus` - Running, Completed, Failed
- ✅ `BotState` - Starting, Running, Paused, Stopping, Stopped, Failed
- ✅ `FinishReason` - Stop, Length, ToolCalls, ContentFilter, Error

**Tuple variant enums:**
- ✅ `MediaSource` - Url(String), Base64(String), Binary(Vec<u8>)

**Struct variant enums (pending serde_json):**
- 🚧 `Input` - Text(String), Image {...}, ToolCall {...}, etc.
- 🚧 `Output` - Text(String), Json(Value), ToolCalls(Vec<ToolCall>), etc.
- 🚧 `ExporterBackend` - Stdout, Otlp { endpoint: String }
- ✅ `HealthStatus` - Healthy, Degraded {...}, Unhealthy {...}

**Structs:**
- 🔜 `Message` - Fields: `role: Role`, `content: Vec<Input>`
- 🔜 `GenerateRequest` - Fields: messages, max_tokens, temperature, model
- 🔜 `GenerateResponse` - Fields: outputs, stop_reason, usage
- 🔜 `StreamChunk` - Fields: content, is_final, finish_reason
- 🔜 `TokenUsageData` - Fields: input_tokens, output_tokens, total_tokens
- 🔜 `ToolDefinition` - Fields: name, description, input_schema (has Value!)
- 🔜 `ToolResult` - Fields: tool_call_id, content (has Value!), is_error
- 🔜 `ToolCall` - Fields: id, name, arguments (has Value!)
- 🔜 `ModelMetadata` - Fields: provider, model, max_input_tokens, features, etc.
- 🔜 `ModelCapabilities` - Fields: streaming, tool_calling, vision, etc.
- 🔜 `ActExecution` - Fields: act_name, inputs, response, usage, etc.
- 🔜 `NarrativeExecution` - Fields: narrative_name, act_executions, totals
- 🔜 `QueryExecutions` - Fields: narrative_name, status, limit, offset
- 🔜 `ExecutionRecord` - Fields: id, narrative_name, status, act_count, error
- 🔜 `ObservabilityConfig` - Fields: service_name, log_level, exporter, etc.
- 🚧 `BotStats` - Fields: counts, duration, **last_task_at: Option<DateTime>**
- 🔜 `BotServerConfig` - Fields: config_path, graceful_shutdown, shutdown_timeout

---

## Blocking Types Analysis

### 1. serde_json::Value (CRITICAL - Blocks 6 types)

**Used in:**
- `Input::ToolCall { arguments: Value }`
- `Output::Json(Value)`
- `ToolCall { arguments: Value }`
- `ToolDefinition { input_schema: Value }`
- `ToolResult { content: Value }`
- Plus any custom config structs

**Status:** 🚧 Implementation plan complete
- Plan: `/home/erik/repos/elicitation/SERDE_JSON_IMPLEMENTATION_PLAN.md`
- Target: elicitation 0.2.2
- Feature: `serde_json`
- Effort: 3-5 hours

**Impact:**
- **Unblocks:** ~10 core types immediately
- **Enables:** Universal JSON handling in all Rust codebases
- **Ecosystem:** Makes elicitation useful for ANY project with JSON

### 2. chrono::DateTime<Utc> (LOW PRIORITY - Blocks 1 type)

**Used in:**
- `BotStats { last_task_at: Option<DateTime<Utc>> }`

**Status:** 🤔 Needs implementation strategy

**Options:**

**A) Implement in elicitation crate (Feature-gated)**
```rust
#[cfg(feature = "chrono")]
impl Elicitation for chrono::DateTime<chrono::Utc> {
    async fn elicit<C: McpClient>(client: &C) -> Result<Self, ErrorData> {
        // Option 1: Parse ISO 8601 string
        let s = String::prompt("Enter datetime (ISO 8601):").elicit(client).await?;
        DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc)
        
        // Option 2: Elicit components (year, month, day, hour, min, sec)
        let year = i32::prompt("Year:").elicit(client).await?;
        let month = u32::prompt("Month (1-12):").elicit(client).await?;
        // ... etc
    }
}
```

**B) Workaround in botticelli (Skip for now)**
```rust
// Don't derive Elicit on BotStats
// Or make last_task_at an internal field not in Elicit
#[derive(Elicit)]
struct BotStats {
    tasks_completed: u64,
    tasks_failed: u64,
    total_processing_time: Duration,
    
    #[elicit(skip)]  // Future feature
    last_task_at: Option<DateTime<Utc>>,
}
```

**C) Use String internally (Lossy)**
```rust
// Store as ISO 8601 string, convert when needed
pub struct BotStats {
    // ... other fields
    last_task_at_iso: Option<String>,  // Elicitable
}

impl BotStats {
    pub fn last_task_at(&self) -> Option<DateTime<Utc>> {
        self.last_task_at_iso.as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }
}
```

**Recommendation:**
- **Short term:** Option B or C (skip or workaround)
- **Long term:** Option A (chrono feature in elicitation 0.2.3+)
- **Rationale:** Only 1 type blocked, not critical path

---

## Implementation Priority

### Phase 1: serde_json (CRITICAL) ⏰ 3-5 hours

**Blocks:** 6+ types immediately, entire ecosystem long-term

**Plan:** Exists at `/home/erik/repos/elicitation/SERDE_JSON_IMPLEMENTATION_PLAN.md`

**Action:**
1. Implement feature in elicitation crate
2. Release elicitation 0.2.2
3. Update botticelli to use it
4. Derive Elicit on blocked types

**Expected outcome:**
```rust
// All of these work:
#[derive(Elicit)]
pub enum Input {
    ToolCall { arguments: Value },  // ✅
}

#[derive(Elicit)]
pub enum Output {
    Json(Value),  // ✅
}

#[derive(Elicit)]
pub struct ToolCall {
    arguments: Value,  // ✅
}
```

### Phase 2: Complete botticelli_core ⏰ 1-2 hours

With `serde_json::Value` working, add `Elicit` derives to all remaining types.

**Eligible types (~20 structs):**
- Message
- GenerateRequest/Response
- StreamChunk
- TokenUsageData
- ToolDefinition/Result
- ModelMetadata/Capabilities
- ActExecution/NarrativeExecution
- ExecutionRecord/QueryExecutions
- ObservabilityConfig
- BotServerConfig
- (BotStats - skip for now due to DateTime)

### Phase 3: chrono (OPTIONAL) ⏰ 2-3 hours

**Blocks:** 1 type (BotStats)

**Plan:** Feature-gated implementation in elicitation 0.2.3+

**Action:**
1. Design chrono elicitation UX (ISO string vs components)
2. Add `chrono` feature to elicitation
3. Implement `Elicitation` for `DateTime<Utc>`
4. Add tests and docs
5. Release elicitation 0.2.3
6. Enable in botticelli

---

## Completionist Roadmap

### Current State (elicitation 0.2.1)

**Working:**
- ✅ All primitives (i32, String, bool, etc.)
- ✅ std types (Vec, Option, HashMap, Duration)
- ✅ Simple enums (unit variants)
- ✅ Tuple variant enums
- ✅ Struct variant enums
- ✅ Structs (all fields must implement Elicitation)

**Blocked:**
- 🚧 Types with `serde_json::Value` fields
- 🚧 Types with `chrono::DateTime` fields

### After elicitation 0.2.2 (serde_json feature)

**Newly working:**
- ✅ Input enum (all variants)
- ✅ Output enum (all variants)
- ✅ ToolCall, ToolDefinition, ToolResult
- ✅ Any struct/enum with Value fields
- ✅ **~95% of botticelli_core types**

**Still blocked:**
- 🚧 BotStats (has DateTime field)

### After elicitation 0.2.3 (chrono feature)

**Newly working:**
- ✅ BotStats
- ✅ **100% of botticelli_core types**

---

## Dependencies Summary

### Must Have (Critical Path)

1. **serde_json::Value** - Elicitation impl
   - Status: Plan exists, ready to implement
   - Impact: Unblocks 95% of types
   - Timeline: 3-5 hours
   - Release: elicitation 0.2.2

### Nice to Have (Completionist)

2. **chrono::DateTime<Utc>** - Elicitation impl
   - Status: Needs design
   - Impact: Unblocks 1 type (BotStats)
   - Timeline: 2-3 hours
   - Release: elicitation 0.2.3+

### Already Handled ✅

- All primitives: bool, i32, u32, i64, u64, f32, f64, usize, String
- std containers: Vec, Option, HashMap
- std time: Duration

---

## Other Crates Analysis

Beyond `botticelli_core`, other workspace crates also have types. However, **core is the foundation** - if we can derive Elicit there, the pattern extends naturally:

**botticelli_database:**
- Uses core types + diesel types
- Diesel types probably won't need Elicit (internal query builders)

**botticelli_models:**
- Uses core types + HTTP client types
- HTTP client types are internal (reqwest, etc.)

**botticelli_narrative:**
- Uses core types + toml types
- TOML types might benefit from Elicit (config elicitation)

**botticelli_mcp:**
- Uses core types + rmcp types
- rmcp types are internal protocol handling

**Conclusion:** Core is 90%+ of the problem. Other crates mostly use core types or internal implementation types that don't need elicitation.

---

## Testing Strategy

### With serde_json Feature

Add to botticelli_core test:

```rust
#[tokio::test]
async fn test_elicit_tool_call() -> anyhow::Result<()> {
    let client = helpers::mock_client(vec![
        // Select variant: ToolCall
        helpers::Response::Select("ToolCall"),
        // Elicit id
        helpers::Response::Text("call_123"),
        // Elicit name
        helpers::Response::Text("get_weather"),
        // Elicit arguments (serde_json::Value)
        helpers::Response::Select("object"),  // JSON type
        helpers::Response::Bool(true),        // Add field
        helpers::Response::Text("location"),  // Key
        helpers::Response::Select("string"),  // Value type
        helpers::Response::Text("SF"),        // Value
        helpers::Response::Bool(false),       // Done
    ]);
    
    let input = Input::elicit(&client).await?;
    
    match input {
        Input::ToolCall { id, name, arguments } => {
            assert_eq!(id, "call_123");
            assert_eq!(name, "get_weather");
            assert_eq!(arguments, json!({"location": "SF"}));
        }
        _ => panic!("Expected ToolCall"),
    }
    
    Ok(())
}
```

---

## Recommendation

**Phase 1 (This Week):**
1. Implement `serde_json` feature in elicitation (3-5 hours)
2. Release elicitation 0.2.2 to crates.io
3. Update botticelli dependency to 0.2.2
4. Add `Elicit` derives to all core types (1-2 hours)
5. Celebrate 95%+ completionist achievement 🎉

**Phase 2 (Future):**
1. Design chrono elicitation UX
2. Implement chrono feature in elicitation 0.2.3
3. Add Elicit to BotStats
4. Achieve 100% completionist 🏆

**Total effort to 95%:** 4-7 hours
**Total effort to 100%:** 6-10 hours

---

## Conclusion

**The answer:** Only **2 external types** need Elicitation implementations to unlock completionist elicit derives across botticelli:

1. **serde_json::Value** (critical, blocks 95%)
2. **chrono::DateTime** (nice-to-have, blocks 5%)

Everything else either:
- Already works (std types, primitives)
- Is our own type (derive Elicit directly)
- Is internal/implementation detail (doesn't need elicitation)

**The path is clear.** Implement serde_json feature, and we traipse untrammelled across the borrow checker! 🚀
