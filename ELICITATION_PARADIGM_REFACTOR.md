# Elicitation Paradigm Refactor

## The Core Insight

**I misunderstood the elicitation crate architecture.** The paradigm traits (`Select`, `Affirm`, `Survey`, `Authorize`) are **not just metadata** - they **ARE the interaction model** that describes HOW the user communicates with the LLM.

This is orthogonal to type validation and represents a fundamental vocabulary for conversational UIs.

---

## Current Botticelli Approach (Wrong Abstraction)

### Current: Method-Based Dialog

```rust
trait ElicitationDialog {
    async fn ask_text(&mut self, prompt: &str) -> BotticelliResult<String>;
    async fn ask_confirmation(&mut self, prompt: &str, default: bool) -> BotticelliResult<bool>;
    async fn ask_choice(&mut self, prompt: &str, options: &[&str]) -> BotticelliResult<usize>;
    async fn ask_number(&mut self, prompt: &str, min: i64, max: i64) -> BotticelliResult<i64>;
}
```

**Problems:**
- ❌ **Method proliferation** - need new method for each interaction type
- ❌ **Type-specific** - `ask_number` is about types, not interaction patterns
- ❌ **Not composable** - can't describe "select from list of numbers"
- ❌ **Missing abstraction** - no way to express "this is a selection pattern"

---

## Elicitation Crate Approach (Correct Abstraction)

### Paradigm Traits: The Interaction Vocabulary

```rust
/// Select: Choose ONE from FINITE options (dropdown/radio)
trait Select: Prompt {
    fn options() -> &'static [Self];
    fn labels() -> &'static [&'static str];
    fn from_label(label: &str) -> Option<Self>;
}

/// Affirm: Binary yes/no (checkbox/confirmation)
trait Affirm: Prompt {}

/// Survey: Multi-field form (wizard/questionnaire)
trait Survey: Prompt {
    fn fields() -> &'static [FieldInfo];
}

/// Authorize: Permission with policies (permission dialog)
trait Authorize: Prompt {
    fn policies() -> &'static [Self];
}
```

**Key insight:** These traits describe **interaction patterns**, not type categories.

---

## The Paradigm = Interaction Pattern Model

### Example: Selecting a Priority

**Wrong (type-focused):**
```rust
// Botticelli current approach
let options = &["Low", "Medium", "High"];
let index = dialog.ask_choice("Choose priority", options).await?;
let priority = match index {
    0 => Priority::Low,
    1 => Priority::Medium,
    2 => Priority::High,
    _ => unreachable!(),
};
```

**Right (paradigm-focused):**
```rust
// Elicitation approach
#[derive(Elicit)]
enum Priority {
    Low,
    Medium,
    High,
}

// Auto-generates:
// impl Select for Priority { ... }
// impl Elicit for Priority { ... }

let priority = Priority::elicit(&client).await?;
```

**What happens under the hood:**
1. Compiler sees `Priority` implements `Select`
2. `elicit()` dispatches to **Select-specific MCP tool** (`elicit_select`)
3. MCP tool uses `Select::labels()` to present options
4. MCP tool uses `Select::from_label()` to parse response
5. Type safety guaranteed at compile time

---

## Why This Matters: Interaction Patterns Are Universal

### Paradigm examples across types:

**Select** (finite choice):
- Enum variants (`Priority::Low`, `Priority::High`)
- File paths from list
- Model names from available models
- Act ordering strategy

**Affirm** (yes/no):
- Boolean flags (`continue_on_error`)
- Confirmation prompts ("Save changes?")
- Feature toggles ("Enable logging?")

**Survey** (multi-field):
- Struct fields (`NarrativeMetadata { name, description, ... }`)
- Configuration objects
- Wizard steps

**Authorize** (permission):
- Tool usage permissions
- Scope grants
- Policy selections

---

## How Botticelli Should Be Refactored

### Step 1: Replace ElicitationDialog with Paradigm-Based Tools

**Current (method-based):**
```rust
trait ElicitationDialog {
    async fn ask_text(&mut self, prompt: &str) -> BotticelliResult<String>;
    async fn ask_choice(&mut self, prompt: &str, options: &[&str]) -> BotticelliResult<usize>;
    // ...
}
```

**Refactored (paradigm-based):**
```rust
// Dialog becomes a paradigm executor
trait ParadigmDialog {
    /// Execute Select paradigm (finite choice)
    async fn select<T: Select>(&mut self) -> BotticelliResult<T>;
    
    /// Execute Affirm paradigm (yes/no)
    async fn affirm<T: Affirm>(&mut self) -> BotticelliResult<T>;
    
    /// Execute Survey paradigm (multi-field)
    async fn survey<T: Survey>(&mut self) -> BotticelliResult<T>;
}
```

**Even better - just use elicitation directly:**
```rust
// No custom dialog trait needed!
// Use elicitation::Elicitation with custom transport

let priority = Priority::elicit(&client).await?;
let confirmed = bool::elicit(&client).await?;
let metadata = NarrativeMetadata::elicit(&client).await?;
```

---

## Concrete Refactor Example

### Before: Manual Choice Handling

```rust
// botticelli_chat/src/elicitation/acts.rs (current)
async fn elicit(&self, dialog: &mut dyn ElicitationDialog, ...) {
    let approach_options = &[
        "Auto-extract from description",
        "Manual count-based specification",
        "Enter acts one-by-one",
    ];

    let approach = dialog
        .ask_choice("How would you like to define acts?", approach_options)
        .await?;

    match approach {
        0 => self.extract_from_description(dialog, partial).await?,
        1 => self.count_based_specification(dialog).await?,
        2 => self.one_by_one_specification(dialog).await?,
        _ => return Err(...),
    }
}
```

### After: Select Paradigm

```rust
// Define enum with Select paradigm
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

// Use it
async fn elicit(&self, client: &pmcp::Client<T>, ...) {
    let approach = ActApproach::elicit(client).await?;

    match approach {
        ActApproach::AutoExtract => self.extract_from_description(client, partial).await?,
        ActApproach::ManualCount => self.count_based_specification(client).await?,
        ActApproach::Interactive => self.one_by_one_specification(client).await?,
    }
}
```

**Benefits:**
- ✅ **Type-safe** - can't pass wrong index
- ✅ **Self-documenting** - enum variants are the options
- ✅ **Compile-time checked** - exhaustive match required
- ✅ **Paradigm-explicit** - clearly a Select interaction

---

## The MCP Tools Should Be Paradigm-Based

### Current Botticelli MCP Tools (Wrong)

```rust
// Separate tool per use case (too specific)
"create_elicitation_session"
"elicit_metadata"
"elicit_act"
"elicit_carousel"
```

### Elicitation Crate MCP Tools (Right)

```rust
// One tool per PARADIGM (reusable)
"elicit_select"   // For any Select type
"elicit_bool"     // For any Affirm type (specialized for bool)
"elicit_text"     // For string input
"elicit_number"   // For numeric input
// Survey is composite - uses above tools for each field
```

**Key insight:** Botticelli's MCP tools are **domain-specific workflows**, not **interaction primitives**.

---

## Architecture: Paradigms as Foundation

```
┌─────────────────────────────────────────────────────────┐
│ Botticelli Narrative Domain Layer                      │
│                                                         │
│  ┌───────────────────────────────────────────┐         │
│  │ ElicitationSession (Orchestrator)         │         │
│  │  - Workflow state machine                 │         │
│  │  - Domain validation                      │         │
│  │  - TOML generation                        │         │
│  └───────────────────────────────────────────┘         │
│              │                                          │
│              ▼                                          │
│  ┌───────────────────────────────────────────┐         │
│  │ Domain Types (Paradigm-Annotated)         │         │
│  │                                            │         │
│  │  #[derive(Elicit)]                        │         │
│  │  enum ActApproach { ... }  ← Select       │         │
│  │                                            │         │
│  │  #[derive(Elicit)]                        │         │
│  │  struct NarrativeMetadata { ... } ← Survey│         │
│  │                                            │         │
│  │  continue: bool  ← Affirm                 │         │
│  └───────────────────────────────────────────┘         │
│              │                                          │
│              ▼                                          │
├─────────────────────────────────────────────────────────┤
│ Elicitation Paradigm Layer (Generic)                   │
│                                                         │
│  ┌───────────────────────────────────────────┐         │
│  │ Select, Affirm, Survey, Authorize         │         │
│  │  - Interaction patterns                   │         │
│  │  - Type-agnostic                          │         │
│  │  - Composable                             │         │
│  └───────────────────────────────────────────┘         │
│              │                                          │
│              ▼                                          │
│  ┌───────────────────────────────────────────┐         │
│  │ MCP Tools (Paradigm-Specific)             │         │
│  │  - elicit_select                          │         │
│  │  - elicit_bool                            │         │
│  │  - elicit_text                            │         │
│  │  - elicit_number                          │         │
│  └───────────────────────────────────────────┘         │
│              │                                          │
│              ▼                                          │
│  ┌───────────────────────────────────────────┐         │
│  │ Transport Layer (pluggable)               │         │
│  │  - StdioTransport (Claude)                │         │
│  │  - HttpTransport (Web)                    │         │
│  │  - DialogTransport (TUI)                  │         │
│  └───────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────┘
```

---

## What This Means for Botticelli

### Current Problems

1. **ElicitationDialog mixes concerns:**
   - `ask_text()` → input type (String)
   - `ask_number()` → input type (i64)
   - `ask_choice()` → interaction pattern (Select)
   - `ask_confirmation()` → interaction pattern (Affirm)

2. **MCP tools too domain-specific:**
   - `elicit_metadata` → Should be Survey paradigm
   - `elicit_act` → Should be Survey paradigm
   - Manual field-by-field prompting → Should be automatic via Survey

3. **No paradigm vocabulary:**
   - Can't express "this is a selection"
   - Can't leverage paradigm-based tooling
   - Can't compose patterns

### Correct Refactor

1. **Replace ElicitationDialog with elicitation traits:**
   ```rust
   // OLD: Custom dialog methods
   dialog.ask_choice("Choose approach", &["Auto", "Manual"]).await?
   
   // NEW: Paradigm-based elicitation
   ActApproach::elicit(&client).await?
   ```

2. **Annotate domain types with paradigms:**
   ```rust
   #[derive(Elicit)]  // Auto-implements Select
   enum ActApproach { AutoExtract, Manual, Interactive }
   
   #[derive(Elicit)]  // Auto-implements Survey
   struct NarrativeMetadata {
       name: String,
       description: String,
       default_model: Option<String>,
   }
   ```

3. **Use elicitation MCP tools directly:**
   - No custom `elicit_metadata` tool
   - Just use `elicit_select`, `elicit_text`, etc.
   - Survey auto-composes the field-level tools

---

## Implementation Plan

### Phase 1: Understand Current Usage

1. Audit all `ask_*` method calls in botticelli
2. Categorize by paradigm:
   - `ask_choice` → mostly Select
   - `ask_confirmation` → Affirm
   - `ask_text` → depends on context
   - `ask_number` → depends on context

### Phase 2: Define Domain Types with Paradigms

1. Convert enums to `#[derive(Elicit)]` with Select
2. Convert structs to `#[derive(Elicit)]` with Survey
3. Use `bool` for Affirm patterns

### Phase 3: Replace Dialog with Elicitation

1. Remove custom `ask_*` methods
2. Use `T::elicit(&client)` directly
3. Implement `DialogTransport` to bridge to UI

### Phase 4: Remove Domain-Specific MCP Tools

1. Delete `elicit_metadata`, `elicit_act`, etc.
2. Use elicitation's paradigm tools
3. Keep only workflow orchestration tools

---

## Example: Complete Refactor

### Before (Current Botticelli)

```rust
// Manual prompting
async fn elicit_metadata(
    &self,
    dialog: &mut dyn ElicitationDialog,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    let name = dialog.ask_text("Narrative name:").await?;
    let description = dialog.ask_text("Description:").await?;
    
    let has_model = dialog
        .ask_confirmation("Specify default model?", false)
        .await?;
    
    let default_model = if has_model {
        Some(dialog.ask_text("Model name:").await?)
    } else {
        None
    };
    
    // ... more manual prompting
    
    partial.set_name(name);
    partial.set_description(description);
    partial.set_default_model(default_model);
    
    Ok(())
}
```

### After (With Elicitation)

```rust
// Define type with Survey paradigm
#[derive(Debug, Clone, Elicit)]
struct NarrativeMetadata {
    #[prompt("Narrative name:")]
    name: String,
    
    #[prompt("Brief description:")]
    description: String,
    
    #[prompt("Default model (optional):")]
    default_model: Option<String>,
    
    #[prompt("Default temperature (optional):")]
    default_temperature: Option<f64>,
}

// Use it
async fn elicit_metadata<T: Transport>(
    &self,
    client: &pmcp::Client<T>,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    // ONE LINE - Survey paradigm handles everything
    let metadata = NarrativeMetadata::elicit(client).await?;
    
    partial.set_metadata(metadata);
    Ok(())
}
```

**Code reduction: 30+ lines → 3 lines**

---

## Key Takeaways

1. **Paradigms ARE the interaction model** - not just metadata
2. **ElicitationDialog is the wrong abstraction** - it mixes types and patterns
3. **MCP tools should be paradigm-based** - not domain-specific
4. **Domain types should declare their paradigm** - via derive macros
5. **Elicitation crate provides the RIGHT foundation** - use it directly

## Next Steps

1. ✅ **Accept this analysis**
2. ✅ **Audit botticelli for paradigm patterns** (Select, Affirm, Survey usage)
3. ✅ **Define narrative types with proper paradigm annotations**
4. ✅ **Implement DialogTransport for UI bridge**
5. ✅ **Refactor elicitors to use elicitation traits**
6. ✅ **Remove domain-specific MCP tools**

**Bottom line:** Botticelli should **use elicitation's paradigm system directly**, not create a parallel abstraction. The paradigm traits ARE the correct way to model conversational UI.
