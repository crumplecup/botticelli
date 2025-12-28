# Elicitation Method Audit

## Summary Statistics

- **ask_choice**: 6 calls → **Select** paradigm (finite options)
- **ask_confirmation**: 31 calls → **Affirm** paradigm (yes/no)
- **ask_text**: 28 calls → Text input (Survey fields or standalone)
- **ask_number**: 10 calls → Numeric input (Survey fields or standalone)

Total: **75 dialog method calls** across the codebase

---

## Categorization by Paradigm

### 1. Select Paradigm (Finite Choice)

#### acts.rs:62 - Act Definition Approach
```rust
// CURRENT
let approach = dialog.ask_choice(
    "How would you like to define acts?",
    &["Auto-extract from description", "Manual count-based specification", "Enter acts one-by-one"]
).await?;

// REFACTOR TO
#[derive(Debug, Clone, Copy, Elicit)]
#[prompt("How would you like to define acts?")]
enum ActApproach {
    #[label("Auto-extract from description")]
    AutoExtract,
    #[label("Manual count-based specification")]
    ManualCount,
    #[label("Enter acts one-by-one")]
    Interactive,
}
```

#### metadata.rs:88 - Default Model Selection
```rust
// CURRENT
let index = dialog.ask_choice("Select default model:", model_options).await?;

// REFACTOR TO
// Use existing model enum or create ModelChoice
```

#### inputs.rs:76 - Input Type Selection
```rust
// CURRENT
let input_type = dialog.ask_choice("Select input type:", input_type_options).await?;

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
enum InputType {
    Text,
    Image,
    Audio,
    Video,
    Document,
    Command,
    Database,
    NarrativeCall,
}
```

#### inputs.rs:209 - URL vs Base64
```rust
// CURRENT
let source = dialog.ask_choice("Source type:", &["URL", "Base64 data"]).await?;

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
enum MediaSource {
    #[label("URL")]
    Url,
    #[label("Base64 data")]
    Base64,
}
```

#### inputs.rs:372 - Output Format
```rust
// CURRENT
let format = dialog.ask_choice("Select output format:", format_options).await?;

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
enum OutputFormat {
    Json,
    Csv,
    // ... etc
}
```

#### inputs.rs:455 - History Retention Mode
```rust
// CURRENT
let mode = dialog.ask_choice("History retention mode:", retention_options).await?;

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
enum HistoryRetentionMode {
    KeepAll,
    KeepLast,
    Clear,
}
```

---

### 2. Survey Paradigm (Multi-Field Forms)

#### Metadata (metadata.rs)
```rust
// CURRENT: 10+ separate ask_* calls
let name = dialog.ask_text("Enter narrative name...").await?;
let description = dialog.ask_text("Enter narrative description...").await?;
let has_model = dialog.ask_confirmation("Set a default model?", false).await?;
// ... many more

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
#[prompt("Configure narrative metadata")]
struct NarrativeMetadata {
    #[prompt("Enter narrative name (alphanumeric and underscores):")]
    name: String,

    #[prompt("Enter narrative description (what does this workflow do?):")]
    description: String,

    #[prompt("Default model (optional):")]
    default_model: Option<String>,

    #[prompt("Default temperature (0.0-2.0, optional):")]
    #[validate(range(0.0..=2.0))]
    default_temperature: Option<f64>,

    #[prompt("Default max tokens (optional):")]
    #[validate(range(1..=1000000))]
    default_max_tokens: Option<i64>,
}
```

**Impact**: ~10 dialog calls → 1 Survey elicitation

#### Carousel Configuration (carousel.rs)
```rust
// CURRENT: 3 separate calls
let iterations = dialog.ask_number("Number of iterations (1-1000):", 1, 1000).await?;
let tokens = dialog.ask_number("Estimated tokens per iteration:", 100, 1000000).await?;
let continue_on_error = dialog.ask_confirmation("Continue execution if an iteration fails?", false).await?;

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
#[prompt("Configure carousel settings")]
struct CarouselConfig {
    #[prompt("Number of iterations:")]
    #[validate(range(1..=1000))]
    iterations: i64,

    #[prompt("Estimated tokens per iteration:")]
    #[validate(range(100..=1000000))]
    estimated_tokens: i64,

    #[prompt("Continue execution if an iteration fails?")]
    continue_on_error: bool,
}
```

**Impact**: 3 dialog calls → 1 Survey elicitation

#### Database Input Configuration (inputs.rs:310-395)
```rust
// CURRENT: 15+ separate calls
let table_name = dialog.ask_text("Enter table name:").await?;
let has_columns = dialog.ask_confirmation("Specify columns?", false).await?;
// ... many more

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
#[prompt("Configure database input")]
struct DatabaseInputConfig {
    #[prompt("Enter table name:")]
    table_name: String,

    #[prompt("Column names (comma-separated, optional):")]
    columns: Option<String>,

    #[prompt("WHERE clause (without WHERE, optional):")]
    where_clause: Option<String>,

    #[prompt("Maximum rows (optional):")]
    #[validate(range(1..=10000))]
    limit: Option<i64>,

    #[prompt("Offset (optional):")]
    offset: Option<i64>,

    #[prompt("ORDER BY clause (without ORDER BY, optional):")]
    order_by: Option<String>,

    #[prompt("Alias for {{alias}} interpolation (optional):")]
    alias: Option<String>,

    #[prompt("Output format:")]
    format: OutputFormat,  // Uses Select

    #[prompt("Random sample rows?")]
    random_sample: bool,

    #[prompt("Sample size (if random):")]
    #[validate(range(1..=1000))]
    sample_size: Option<i64>,

    #[prompt("Destructive read (pull and delete)?")]
    destructive: bool,
}
```

**Impact**: 15+ dialog calls → 1 Survey elicitation

#### Command Input Configuration (inputs.rs:248-294)
```rust
// CURRENT: 7+ separate calls
let platform = dialog.ask_text("Enter platform...").await?;
let command = dialog.ask_text("Enter command...").await?;
// ... more

// REFACTOR TO
#[derive(Debug, Clone, Elicit)]
#[prompt("Configure command input")]
struct CommandInputConfig {
    #[prompt("Enter platform (e.g., discord, slack):")]
    platform: String,

    #[prompt("Enter command (e.g., server.get_stats):")]
    command: String,

    // Arguments could be a nested Survey or separate step
    #[prompt("Command arguments (JSON object, optional):")]
    arguments: Option<String>,

    #[prompt("Is this command required (halt on failure)?")]
    required: bool,

    #[prompt("Cache duration in seconds (0 to disable):")]
    #[validate(range(0..=86400))]
    cache_duration: i64,
}
```

**Impact**: 7+ dialog calls → 1 Survey elicitation

---

### 3. Affirm Paradigm (Yes/No Confirmations)

Most of these are simple boolean decisions that can remain as `bool::elicit()` calls:

- validation.rs:128,279 - Auto-fix confirmations
- metadata.rs:75,103,117 - "Set a default X?" confirmations
- acts.rs:151,245 - "Use these acts?", "Add another act?"
- inputs.rs:56,100,125,142,159,176,186,259,284,313,324,334,341,349,362,382,392,425 - Various yes/no questions
- carousel.rs:61,127,178 - Carousel enablement

**Pattern**: These can use `elicitation::bool::Affirm` with custom prompts

---

### 4. Text/Number Input (Standalone or Survey Fields)

#### Standalone Text Inputs
- acts.rs:188,217,236 - Act names and prompts (sequential, context-dependent)
- inputs.rs:264,270 - Dynamic argument collection (loop-based)

These are **context-dependent** and may need to remain as individual calls due to:
- Sequential dependency (next prompt depends on previous answer)
- Dynamic loops (unknown number of inputs)
- Validation/error handling between steps

#### Standalone Number Inputs
- acts.rs:180 - "How many acts?" (determines loop count)
- inputs.rs:335,342,385 - Conditional numeric inputs

These are also **context-dependent** and control flow.

---

## Refactor Strategy

### Phase 1: Low-Hanging Fruit (High Impact, Low Risk)

1. **Create domain enums for Select paradigm** (6 occurrences)
   - ActApproach
   - InputType
   - MediaSource
   - OutputFormat
   - HistoryRetentionMode

2. **Create Survey structs for multi-field forms** (3 major forms)
   - NarrativeMetadata (~10 calls → 1)
   - CarouselConfig (3 calls → 1)
   - DatabaseInputConfig (15+ calls → 1)
   - CommandInputConfig (7+ calls → 1)

**Impact**: ~35 dialog calls eliminated → ~6 elicit calls

### Phase 2: Affirm Conversion (Medium Impact)

3. **Convert simple confirmations to bool::elicit()**
   - Most of the 31 ask_confirmation calls
   - Use custom prompts via elicitation

**Impact**: Standardized boolean elicitation

### Phase 3: Complex Flows (Low Priority)

4. **Handle context-dependent flows**
   - Act creation loops (depends on count)
   - Dynamic argument collection
   - Sequential prompts with validation

These may need custom orchestration or remain as-is.

---

## Example Conversion

### Before (metadata.rs - Current)
```rust
async fn elicit_metadata(
    &self,
    dialog: &mut dyn ElicitationDialog,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    // 10+ dialog.ask_* calls
    let name = dialog.ask_text("Enter narrative name...").await?;
    let description = dialog.ask_text("Enter narrative description...").await?;
    let has_model = dialog.ask_confirmation("Set a default model?", false).await?;
    let default_model = if has_model {
        let model_options = &["gemini-1.5-flash", "claude-3-5-sonnet-20241022", "Custom"];
        let choice = dialog.ask_choice("Select default model:", model_options).await?;
        if choice == 2 {
            Some(dialog.ask_text("Enter model name:").await?)
        } else {
            Some(model_options[choice].to_string())
        }
    } else {
        None
    };
    // ... more prompts

    partial.set_name(name);
    partial.set_description(description);
    partial.set_default_model(default_model);
    // ... more setters

    Ok(())
}
```

### After (With Elicitation)
```rust
async fn elicit_metadata<T: Transport>(
    &self,
    client: &pmcp::Client<T>,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    // ONE LINE - Survey paradigm handles everything
    let metadata = NarrativeMetadata::elicit(client).await?;

    partial.set_name(metadata.name);
    partial.set_description(metadata.description);
    partial.set_default_model(metadata.default_model);
    partial.set_default_temperature(metadata.default_temperature);
    partial.set_default_max_tokens(metadata.default_max_tokens);

    Ok(())
}
```

**Code reduction**: 30+ lines → 8 lines
**Maintainability**: Type-safe, compile-time checked, self-documenting

---

## Next Steps

1. ✅ **Audit complete**
2. → **Create domain type definitions** (enums and structs with #[derive(Elicit)])
3. → **Implement Transport bridge** (DialogTransport to connect elicitation to ElicitationDialog)
4. → **Refactor elicitors one by one** (start with metadata - highest impact)
5. → **Remove/deprecate ElicitationDialog trait** (once all conversions complete)

---

## Benefits Summary

- **Code reduction**: ~75 dialog calls → ~15 elicit calls (80% reduction)
- **Type safety**: Compile-time checking, exhaustive match enforcement
- **Self-documenting**: Domain types describe the interaction model
- **Composable**: Survey fields can use Select/Affirm types
- **Testable**: Mock Transport instead of entire dialog trait
- **Paradigm-explicit**: Clear separation of Select/Affirm/Survey patterns
