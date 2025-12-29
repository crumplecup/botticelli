# Elicitation Transport Bridge Implementation

## Overview

This document provides a concrete implementation plan for bridging the `elicitation` crate with botticelli's `ElicitationDialog` trait using a custom MCP transport.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│ Botticelli Narrative Elicitation               │
│                                                 │
│  ┌──────────────────────────────────┐          │
│  │ NarrativeMetadata::elicit()      │          │
│  └──────────────────────────────────┘          │
│              │ (calls)                          │
│              ▼                                  │
│  ┌──────────────────────────────────┐          │
│  │ pmcp::Client<DialogTransport>    │          │
│  └──────────────────────────────────┘          │
│              │ (MCP request)                    │
│              ▼                                  │
│  ┌──────────────────────────────────┐          │
│  │ DialogTransport                  │          │
│  │  - Implements pmcp::Transport    │          │
│  │  - Bridges to ElicitationDialog  │          │
│  └──────────────────────────────────┘          │
│              │ (translates to)                  │
│              ▼                                  │
│  ┌──────────────────────────────────┐          │
│  │ ElicitationDialog                │          │
│  │  - TuiDialog / WebDialog         │          │
│  └──────────────────────────────────┘          │
└─────────────────────────────────────────────────┘
```

---

## Implementation

### Step 1: Add Dependencies

```toml
# botticelli_chat/Cargo.toml
[dependencies]
elicitation = "0.1"
pmcp = "1.4"
async-trait = "0.1"
```

### Step 2: Create Transport Bridge

```rust
// botticelli_chat/src/elicitation/transport.rs

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_mcp::ElicitationDialog;
use pmcp::shared::transport::{Request, Response, Transport};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Custom MCP transport that bridges to ElicitationDialog.
///
/// This allows elicitation crate to work with botticelli's
/// existing dialog abstraction without modification.
pub struct DialogTransport {
    dialog: Arc<Mutex<Box<dyn ElicitationDialog>>>,
}

impl DialogTransport {
    /// Create new transport wrapping a dialog.
    pub fn new(dialog: Box<dyn ElicitationDialog>) -> Self {
        Self {
            dialog: Arc::new(Mutex::new(dialog)),
        }
    }

    /// Helper to extract string parameter.
    fn get_prompt(params: &Value) -> Result<String, TransportError> {
        params
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| TransportError::missing_param("prompt"))
    }

    /// Helper to extract number parameters.
    fn get_range(params: &Value) -> Result<(i64, i64), TransportError> {
        let min = params
            .get("min")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| TransportError::missing_param("min"))?;

        let max = params
            .get("max")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| TransportError::missing_param("max"))?;

        Ok((min, max))
    }

    /// Helper to extract options array.
    fn get_options(params: &Value) -> Result<Vec<String>, TransportError> {
        params
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .ok_or_else(|| TransportError::missing_param("options"))
    }
}

#[async_trait]
impl Transport for DialogTransport {
    type Error = TransportError;

    async fn send(&mut self, request: Request) -> Result<Response, Self::Error> {
        let mut dialog = self.dialog.lock().await;

        match request.method.as_str() {
            // Text elicitation
            "elicit_text" => {
                let prompt = Self::get_prompt(&request.params)?;
                let text = dialog
                    .ask_text(&prompt)
                    .await
                    .map_err(TransportError::dialog)?;

                Ok(Response::success(serde_json::json!({ "value": text })))
            }

            // Number elicitation
            "elicit_number" => {
                let prompt = Self::get_prompt(&request.params)?;
                let (min, max) = Self::get_range(&request.params)?;

                let num = dialog
                    .ask_number(&prompt, min, max)
                    .await
                    .map_err(TransportError::dialog)?;

                Ok(Response::success(serde_json::json!({ "value": num })))
            }

            // Boolean elicitation
            "elicit_bool" => {
                let prompt = Self::get_prompt(&request.params)?;
                let default = request
                    .params
                    .get("default")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let confirmed = dialog
                    .ask_confirmation(&prompt, default)
                    .await
                    .map_err(TransportError::dialog)?;

                Ok(Response::success(
                    serde_json::json!({ "value": confirmed }),
                ))
            }

            // Select elicitation
            "elicit_select" => {
                let prompt = Self::get_prompt(&request.params)?;
                let options = Self::get_options(&request.params)?;

                // Convert to &str array for dialog API
                let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();

                let index = dialog
                    .ask_choice(&prompt, &option_refs)
                    .await
                    .map_err(TransportError::dialog)?;

                let selected = options
                    .get(index)
                    .ok_or_else(|| TransportError::invalid_index(index, options.len()))?;

                Ok(Response::success(serde_json::json!({ "value": selected })))
            }

            // Unsupported method
            method => Err(TransportError::unsupported_method(method)),
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        // No cleanup needed for dialog transport
        Ok(())
    }
}

/// Transport errors.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum TransportError {
    #[display("Missing parameter: {}", _0)]
    MissingParameter(String),

    #[display("Dialog error: {}", _0)]
    DialogError(String),

    #[display("Unsupported method: {}", _0)]
    UnsupportedMethod(String),

    #[display("Invalid index {} (max {})", index, max)]
    InvalidIndex { index: usize, max: usize },
}

impl TransportError {
    fn missing_param(name: &str) -> Self {
        Self::MissingParameter(name.to_string())
    }

    fn dialog(err: impl std::fmt::Display) -> Self {
        Self::DialogError(err.to_string())
    }

    fn unsupported_method(method: &str) -> Self {
        Self::UnsupportedMethod(method.to_string())
    }

    fn invalid_index(index: usize, max: usize) -> Self {
        Self::InvalidIndex { index, max }
    }
}
```

### Step 3: Define Narrative Types with Elicitation

```rust
// botticelli_chat/src/elicitation/types.rs

use elicitation::Elicit;

/// Narrative metadata elicited via derive macro.
#[derive(Debug, Clone, Elicit)]
#[prompt("Let's configure your narrative:")]
pub struct NarrativeMetadata {
    #[prompt("Narrative name:")]
    pub name: String,

    #[prompt("Brief description:")]
    pub description: String,

    #[prompt("Default model (e.g., gemini-2.0-flash-thinking-exp):")]
    pub default_model: Option<String>,

    #[prompt("Default temperature (0.0-1.0):")]
    pub default_temperature: Option<f64>,
}

/// Narrative act definition.
#[derive(Debug, Clone, Elicit)]
pub struct Act {
    #[prompt("Act name (identifier):")]
    pub name: String,

    #[prompt("Act prompt:")]
    pub prompt: String,

    #[prompt("Model override (optional):")]
    pub model: Option<String>,

    #[prompt("Temperature override (optional):")]
    pub temperature: Option<f64>,
}

/// Approach for defining acts.
#[derive(Debug, Clone, Copy, Elicit)]
#[prompt("How would you like to define acts?")]
pub enum ActApproach {
    #[prompt("Auto-extract from description")]
    AutoExtract,

    #[prompt("Manual specification")]
    Manual,

    #[prompt("Enter acts one-by-one")]
    Interactive,
}
```

### Step 4: Integrate in Elicitor

```rust
// botticelli_chat/src/elicitation/metadata.rs

use crate::elicitation::transport::DialogTransport;
use crate::elicitation::types::NarrativeMetadata;
use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative};
use elicitation::Elicitation;

/// Elicits narrative metadata using elicitation crate.
pub struct MetadataElicitor;

#[async_trait]
impl NarrativeElicitor for MetadataElicitor {
    fn name(&self) -> &str {
        "Metadata"
    }

    fn description(&self) -> &str {
        "Gather narrative name, description, and defaults"
    }

    fn can_run(&self, _partial: &PartialNarrative) -> bool {
        true // Always can run
    }

    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> BotticelliResult<()> {
        // Create transport bridge
        let transport = DialogTransport::new(Box::new(dialog));
        let client = pmcp::Client::new(transport);

        // Use elicitation crate to gather metadata
        let metadata = NarrativeMetadata::elicit(&client)
            .await
            .map_err(|e| {
                ChatError::new(ChatErrorKind::Elicitation(e.to_string()))
            })?;

        // Update partial narrative
        partial.set_name(metadata.name);
        partial.set_description(metadata.description);

        if let Some(model) = metadata.default_model {
            partial.set_default_model(model);
        }

        if let Some(temp) = metadata.default_temperature {
            partial.set_default_temperature(temp);
        }

        Ok(())
    }

    fn is_complete(&self, partial: &PartialNarrative) -> bool {
        partial.name().is_some() && partial.description().is_some()
    }
}
```

### Step 5: Testing

```rust
// botticelli_chat/tests/elicitation_bridge_test.rs

use botticelli_chat::elicitation::{DialogTransport, NarrativeMetadata};
use botticelli_mcp::ElicitationDialog;
use elicitation::Elicitation;

/// Mock dialog for testing.
struct MockDialog {
    responses: Vec<String>,
    index: usize,
}

#[async_trait::async_trait]
impl ElicitationDialog for MockDialog {
    async fn ask_text(&mut self, _prompt: &str) -> BotticelliResult<String> {
        let response = self.responses[self.index].clone();
        self.index += 1;
        Ok(response)
    }

    // ... implement other methods
}

#[tokio::test]
async fn test_metadata_elicitation() {
    let dialog = MockDialog {
        responses: vec![
            "test_narrative".to_string(),
            "A test narrative".to_string(),
            "gemini-2.0-flash-thinking-exp".to_string(),
            "0.7".to_string(),
        ],
        index: 0,
    };

    let transport = DialogTransport::new(Box::new(dialog));
    let client = pmcp::Client::new(transport);

    let metadata = NarrativeMetadata::elicit(&client).await.unwrap();

    assert_eq!(metadata.name, "test_narrative");
    assert_eq!(metadata.description, "A test narrative");
    assert_eq!(
        metadata.default_model,
        Some("gemini-2.0-flash-thinking-exp".to_string())
    );
    assert_eq!(metadata.default_temperature, Some(0.7));
}
```

---

## Benefits

### Code Reduction

**Before (manual elicitation):**
```rust
// ~50 lines of manual prompting, validation, error handling
async fn elicit(&self, dialog: &mut dyn ElicitationDialog, ...) {
    let name = dialog.ask_text("Narrative name:").await?;
    // validate name...
    
    let desc = dialog.ask_text("Description:").await?;
    // validate description...
    
    // ... repeat for each field
}
```

**After (derived elicitation):**
```rust
// ~10 lines - derive macro handles everything
#[derive(Elicit)]
#[prompt("Configure narrative:")]
struct NarrativeMetadata {
    #[prompt("Name:")]
    name: String,
    #[prompt("Description:")]
    description: String,
}

let metadata = NarrativeMetadata::elicit(&client).await?;
```

### Type Safety

- ✅ **Compile-time guarantees** - can't forget fields
- ✅ **Automatic validation** - type system enforces constraints
- ✅ **Composable** - `Vec<Act>`, `Option<Model>` work automatically

### Maintainability

- ✅ **Single source of truth** - type definition IS the elicitation spec
- ✅ **Less duplication** - no separate validation logic
- ✅ **Easier refactoring** - change type, elicitation updates automatically

---

## Migration Strategy

### Phase 1: Pilot (Week 1)
1. Implement `DialogTransport`
2. Test with simple types (`String`, `i32`, `bool`)
3. Validate MCP protocol compatibility

### Phase 2: Simple Elicitors (Week 2-3)
1. Refactor `MetadataElicitor` to use derived elicitation
2. Measure code reduction and correctness improvements
3. Document patterns and best practices

### Phase 3: Complex Elicitors (Week 4-6)
1. Refactor `ActElicitor` with hybrid approach:
   - Use elicitation for data collection
   - Keep custom logic for conditional workflows
2. Update `InputElicitor` and `CarouselElicitor`
3. Full integration testing

### Phase 4: Production (Week 7+)
1. Add HTTP transport support for web deployment
2. Performance optimization
3. Documentation and examples
4. Consider contributing patterns back to elicitation crate

---

## Conclusion

The transport-agnostic design of the elicitation crate makes integration **straightforward**. By implementing a custom `DialogTransport`, botticelli can leverage elicitation's type-safe derive macros while preserving its existing UI abstractions.

This approach:
- ✅ **Reduces code** through derive macros
- ✅ **Improves type safety** through compile-time checks
- ✅ **Preserves flexibility** through transport abstraction
- ✅ **Enables gradual migration** through hybrid approach

**Next step:** Implement `DialogTransport` and pilot test with `MetadataElicitor`.
