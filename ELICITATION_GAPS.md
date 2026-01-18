# Elicitation Crate Gap Analysis

## Current Limitation: Enum Variants with Fields

### The Problem

The elicitation crate's `#[derive(Elicit)]` for enums currently only supports **unit variants** (variants without fields). This is explicitly stated in the error message:

```rust
"Elicit derive for enums requires at least one unit variant. 
 Variants with fields are not supported in v0.1.0."
```

### Affected Types in Botticelli

**MediaSource** - Cannot derive Elicit:
```rust
pub enum MediaSource {
    Url(String),           // Tuple variant - NOT SUPPORTED
    Base64(String),        // Tuple variant - NOT SUPPORTED
    Binary(Vec<u8>),       // Tuple variant - NOT SUPPORTED
}
```

**Input** - Cannot derive Elicit:
```rust
pub enum Input {
    Text(String),          // Tuple variant
    Image {                // Struct variant - NOT SUPPORTED
        mime: Option<String>,
        source: MediaSource,
    },
    Document {             // Struct variant - NOT SUPPORTED
        mime: Option<String>,
        source: MediaSource,
        filename: Option<String>,
    },
    Table {                // Struct variant - NOT SUPPORTED
        table_name: String,
        columns: Option<Vec<String>>,
        // ... many fields
    },
    // etc.
}
```

**Output** - Cannot derive Elicit:
```rust
pub enum Output {
    Text(String),          // Tuple variant - NOT SUPPORTED
    ToolCall(ToolCall),    // Tuple variant - NOT SUPPORTED
}
```

**HealthStatus** - Cannot derive Elicit:
```rust
pub enum HealthStatus {
    Healthy,               // Unit variant - OK
    Degraded {             // Struct variant - NOT SUPPORTED
        message: String,
    },
    Unhealthy {            // Struct variant - NOT SUPPORTED
        message: String,
    },
}
```

**ExporterBackend** - Cannot derive Elicit:
```rust
pub enum ExporterBackend {
    Stdout,                // Unit variant - OK
    #[cfg(feature = "otel-otlp")]
    Otlp {                 // Struct variant - NOT SUPPORTED
        endpoint: String,
    },
}
```

### Impact Assessment

**Currently Supported (Unit Variants Only):**
- ✅ Role (System, User, Assistant)
- ✅ HistoryRetention (Full, Summary, Drop)
- ✅ TableFormat (Json, Markdown, Csv)
- ✅ StopReason (EndTurn, MaxTokens, etc.)
- ✅ ExecutionStatus (Running, Completed, Failed)
- ✅ BotState (Starting, Running, Paused, etc.)
- ✅ FinishReason (Stop, Length, StopSequence, etc.)

**Not Supported (Has Field Variants):**
- ❌ MediaSource - tuple variants
- ❌ Input - complex struct variants
- ❌ Output - tuple variants
- ❌ HealthStatus - struct variants
- ❌ ExporterBackend - struct variant

### Why This Limitation Exists

Looking at the derive macro code (`enum_impl.rs:25-46`):

```rust
// Extract only unit variants (no fields)
let unit_variants: Vec<_> = data_enum
    .variants
    .iter()
    .filter(|v| matches!(v.fields, Fields::Unit))
    .collect();

if unit_variants.is_empty() {
    let error = syn::Error::new_spanned(
        name,
        "Elicit derive for enums requires at least one unit variant. \
         Variants with fields are not supported in v0.1.0.",
    );
    return error.to_compile_error().into();
}
```

The macro explicitly filters to unit variants only. The comment says "v0.1.0" suggesting this is a known temporary limitation.

### What's Needed for Support

#### Pattern 1: Tuple Variants → Survey Each Field

For `MediaSource::Url(String)`, the elicitation flow could be:
1. User selects variant: "Url" vs "Base64" vs "Binary"
2. Survey pattern elicits the inner type: `String::elicit()`
3. Construct: `MediaSource::Url(value)`

**Implementation approach:**
```rust
// Pseudo-code for generated impl
impl Elicit for MediaSource {
    async fn elicit(client: &Peer<RoleClient>) -> ElicitResult<Self> {
        // Step 1: Select variant
        let variant = elicit_select(
            client,
            "Select media source type:",
            &["Url", "Base64", "Binary"]
        ).await?;
        
        // Step 2: Elicit fields for chosen variant
        match variant {
            "Url" => {
                let url: String = String::elicit(client).await?;
                Ok(MediaSource::Url(url))
            }
            "Base64" => {
                let data: String = String::elicit(client).await?;
                Ok(MediaSource::Base64(data))
            }
            "Binary" => {
                let data: Vec<u8> = Vec::<u8>::elicit(client).await?;
                Ok(MediaSource::Binary(data))
            }
            _ => unreachable!()
        }
    }
}
```

#### Pattern 2: Struct Variants → Nested Survey

For `Input::Image { mime, source }`, the flow:
1. Select variant: "Text", "Image", "Document", etc.
2. For Image: Survey the named fields
3. Each field uses its own elicitation

**Implementation:**
```rust
match variant {
    "Image" => {
        let mime: Option<String> = Option::<String>::elicit(client).await?;
        let source: MediaSource = MediaSource::elicit(client).await?;
        Ok(Input::Image { mime, source })
    }
}
```

### Workarounds for Now

#### Option 1: Manual Implementation

For critical types, manually implement Elicit:
```rust
#[async_trait]
impl Elicitation for MediaSource {
    async fn elicit(client: &Peer<RoleClient>) -> ElicitResult<Self> {
        // Custom implementation
    }
}
```

#### Option 2: Wrapper Types

Create unit variant enums that map to field variants:
```rust
#[derive(Elicit)]
pub enum MediaSourceType {
    Url,
    Base64,
    Binary,
}

// Then manually construct MediaSource based on selection
```

#### Option 3: Split Complex Enums

Break field-variant enums into separate types:
```rust
#[derive(Elicit)]
pub enum MediaSourceKind { Url, Base64, Binary }

pub struct MediaSourceUrl(pub String);
pub struct MediaSourceBase64(pub String);
pub struct MediaSourceBinary(pub Vec<u8>);
```

But this loses the enum type safety.

### Recommendation: Extend Elicitation Crate

This is a **high-value enhancement** for your crate because:

1. **Common pattern**: Enum variants with fields are extremely common in Rust
2. **Composability**: Makes the derive work with more realistic types
3. **Nested elicitation**: Leverages existing Survey pattern for struct variants
4. **Type safety**: Maintains Rust's sum type semantics

**Implementation plan:**
1. Extend `enum_impl.rs` to handle `Fields::Named` and `Fields::Unnamed`
2. For tuple variants: sequentially elicit each field
3. For struct variants: use Survey pattern on the fields
4. Generate match arms for each variant type
5. Add tests for tuple variants, struct variants, mixed variants

**Effort estimate:** 4-8 hours to implement, test, document

### Alternative: Aggressive Manual Implementation

If enhancing the elicitation crate isn't immediate priority, we can:
1. Use `#[derive(Elicit)]` on all unit-variant enums (7 types done ✅)
2. Manually implement `Elicitation` for the 5 field-variant enums
3. Document which types have manual vs derived impls
4. Add unit tests for manual implementations

**Effort:** 2-3 hours for 5 manual implementations

### Questions for You

1. **Priority:** Do you want to extend elicitation crate now, or proceed with manual impls?
2. **Scope:** Should we support only tuple variants, or also struct variants?
3. **Naming:** For tuple variants, how should we prompt? "Enter value for Url:" or more specific?
4. **Validation:** Should variant field elicitation have validation hooks?

### Impact on Current Migration

**Can proceed with current limitation:**
- 7 simple enums ✅ already have derives
- Focus next on structs (Message, ActConfig, etc.) which DO work with derives
- Defer field-variant enums until decision made

**Does NOT block migration** - we have plenty of elicitable types to work with.

## Summary

The elicitation crate is **excellent for unit-variant enums** (the common Select pattern). The limitation on field variants is explicitly labeled "v0.1.0" suggesting you knew this was MVP.

For Botticelli's needs:
- **Good coverage** for simple selection enums (7/12 enum types)
- **Gap** for sum types with data (5/12 enum types)
- **Workaround** exists (manual Elicitation impl)
- **Enhancement path** is clear and valuable

Your crate works as designed - this is just the next evolution step if you want aggressive elicitation coverage.
