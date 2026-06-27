# Bug: `elicitation_derive` generates `Select` impl incompatible with `Select` trait

## Status

Latent compile-time error. Affects `elicitation_derive` ≥ 0.11.0 (at least through 0.11.1).
Triggered the first time a struct field uses the `style = "..."` form of `#[prompt]`.

---

## Background: the two `#[prompt]` forms

The `Elicit` derive macro recognises two syntaxes on struct fields:

```rust
// Form A — plain prompt (goes to field.default_prompt)
#[prompt("Human-readable question text")]
pub name: String,

// Form B — styled prompt (goes to field.styled_prompts["Human"])
#[prompt("Human-readable question text", style = "Human")]
pub name: String,
```

**Form B is the only form that causes custom prompt text to appear in a live
[`TuiCommunicator`] session** (see *Secondary issue* below). Form A is correctly
parsed by the macro but the text is silently dropped from the live elicitation
flow.

---

## Primary issue: generated `Select` impl doesn't match the `Select` trait

### What the macro generates

When **any** field in a struct carries Form B, `generate_elicit_impl_styled`
(`elicitation_derive/src/struct_impl.rs:1176`) is chosen instead of
`generate_elicit_impl_simple`. It emits a private style-selection enum plus a
`Select` impl:

```rust
// elicitation_derive-0.11.1/src/struct_impl.rs  lines 1459-1473
impl elicitation::Select for #style_enum_name {
    fn options() -> &'static [Self] {          // ← &'static slice
        &[#(Self::#style_variants),*]
    }

    fn labels() -> &'static [&'static str] {   // ← &'static slice of &'static str
        &[#(#style_labels),*]
    }

    fn from_label(label: &str) -> Option<Self> { ... }
}
```

The same signatures appear in `elicitation_derive-0.11.0/src/struct_impl.rs`
lines 1407–1411 — identical in both patch versions.

### What the trait declares

```rust
// elicitation-0.11.1/src/paradigm.rs  lines 54-79
pub trait Select: Prompt + Sized {
    fn options() -> Vec<Self>;       // ← Vec, not &'static [...]
    fn labels() -> Vec<String>;      // ← Vec<String>, not &'static [&'static str]
    fn from_label(label: &str) -> Option<Self>;
}
```

Same signatures in `elicitation-0.11.0/src/paradigm.rs` lines 59, 65.

### The mismatch

| Method    | Trait signature (library)   | Generated impl (derive macro) |
|-----------|-----------------------------|-------------------------------|
| `options` | `-> Vec<Self>`              | `-> &'static [Self]`          |
| `labels`  | `-> Vec<String>`            | `-> &'static [&'static str]`  |

Rust rejects this with:

```
error[E0053]: method `options` has an incompatible type for trait
  --> crates/botticelli_bot/src/generator.rs:25:60
   |
   |   expected `Vec<BotConfigGeneratorElicitStyle>`,
   |      found `&'static [BotConfigGeneratorElicitStyle]`

error[E0053]: method `labels` has an incompatible type for trait
   |   expected `Vec<std::string::String>`,
   |      found `&'static [&'static str]`
```

### Why it hasn't surfaced until now

Every existing struct using `Elicit` in this codebase (`NarrativeGenerator`,
`NarrativeActSpec`, `NarrativeCarouselSpec`, `PostingJitter`, `UserBotConfig`,
…) uses **Form A only** — plain `#[prompt("...")]`. That routes through
`generate_elicit_impl_simple`, which never emits a `Select` impl and avoids
the mismatch entirely. The bug is latent until the first struct tries Form B.

---

## Secondary issue: Form A prompt text is silently ignored in live elicitation

Even if the primary bug is fixed, there is a separate design gap with Form A.

In `generate_elicit_impl_simple`
(`elicitation_derive-0.11.1/src/struct_impl.rs:1026-1036`), each field's
elicitation statement is:

```rust
quote! {
    tracing::debug!(field = #name_str, "Eliciting field");
    let #name = <#ty>::elicit(communicator).await?;
}
```

The `field.default_prompt` captured from `#[prompt("...")]` is **never used
here**. It ends up only in `Prompt::prompt()` (metadata / `prompt_tree()`). For
`String` fields, `<String>::elicit(communicator)` is called, which:

1. Calls `communicator.style_or_elicit::<String>()` to retrieve `StringStyle`.
2. If no style is pre-set, **elicits the style** from the user
   (`"Select elicitation style:\nOptions: human, agent"`).
3. Uses the style's hard-coded default: `"Please provide a text value:"` (Human)
   or `"Value?"` (Agent).

The field's `#[prompt("What should we call this bot?")]` text is never passed to
`send_prompt`. The user sees only the style-default text.

This is the root cause of the "Value?" complaint during the `UserBotConfig`
wizard: the communicator had no style pre-set, so `String::elicit` elicited the
style (user typed "agent"), then all subsequent fields showed `"Value?"`.

---

## Proposed fixes

### Fix 1 — `elicitation_derive`: align generated `Select` with trait

**Option A — change the generated code to return `Vec`:**

```rust
// struct_impl.rs  generate_elicit_impl_styled
impl elicitation::Select for #style_enum_name {
    fn options() -> Vec<Self> {
        vec![#(Self::#style_variants),*]     // Vec instead of &'static slice
    }

    fn labels() -> Vec<String> {
        vec![#(#style_labels.to_string()),*]  // Vec<String> instead of &'static [&'static str]
    }

    fn from_label(label: &str) -> Option<Self> { ... }
}
```

**Option B (preferred) — change the `Select` trait to use slices:**

Returning owned `Vec` from a static-dispatch trait method allocates on every
call. The options and labels are always compile-time constants; a `&'static [...]`
is more appropriate:

```rust
// paradigm.rs
pub trait Select: Prompt + Sized {
    fn options() -> &'static [Self];          // zero-cost, always constant
    fn labels() -> &'static [&'static str];   // zero-cost, always constant
    fn from_label(label: &str) -> Option<Self>;
}
```

All existing hand-written `Select` impls (`StringStyle`, enums, etc.) would
need updating, but that's a one-time migration and the right long-term API.

Also update any call sites that currently iterate `labels()` as `Vec<String>`
(e.g. the `elicit_select` tool argument builder) to work with `&'static [&'static str]`.

### Fix 2 — `generate_elicit_impl_simple`: pass `default_prompt` to `send_prompt`

For structs where all fields use Form A (plain `#[prompt]`), the simple impl
should use the field's prompt text directly instead of delegating to the type's
`elicit()`:

```rust
// Proposed replacement for the Form A field statement
let #name = if let Some(prompt) = #field_default_prompt {
    // Use the field's declared prompt text directly
    let response = communicator.send_prompt(prompt).await?;
    response.trim().parse::<#field_ty>()
        .map_err(|e| elicitation::ElicitError::new(
            elicitation::ElicitErrorKind::ParseError(format!("{}", e))
        ))?
} else {
    <#field_ty>::elicit(communicator).await?
};
```

For `String` fields specifically, no `parse()` is needed — `send_prompt`
returns `String` directly. For integers/bools/floats the parse is needed.
For complex types (no `default_prompt`, or non-primitive), fall through to
`<#field_ty>::elicit(communicator)` as today.

This closes the gap where `#[prompt("What should we call this bot?")]` on a
`String` field is visible in code but invisible to the user at runtime.

---

## Workaround (current botticelli state)

Until the library is fixed:

1. `BotConfigGenerator` uses Form A `#[prompt]` on each field (correct pattern
   even if text isn't shown at runtime).
2. The controller pre-sets `StringStyle::Human` before calling `elicit`:

   ```rust
   use elicitation::{ElicitCommunicator, Elicitation};
   let comm = elicit_ratatui::TuiCommunicator::new()
       .with_style::<String, elicitation::StringStyle>(elicitation::StringStyle::Human);
   match BotConfigGenerator::elicit(&comm).await { ... }
   ```

   This suppresses the style-picker question and ensures "Please provide a text
   value:" rather than "Value?" for each field. The field-specific story text
   from `#[prompt]` will appear once Fix 2 is applied.
