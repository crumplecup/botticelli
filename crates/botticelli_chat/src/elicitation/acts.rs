//! Refactored act elicitation using elicitation crate paradigms.
//!
//! This refactored version demonstrates code reduction through the use of:
//! - #[derive(Elicit)] macros for type-safe data collection
//! - Shared MCP infrastructure for primitive elicitation
//! - ActDefinition type with Survey paradigm for act specification
//!
//! **Expected reduction**: ~60-70% code reduction for elicitation logic
//! while preserving all three workflow approaches (auto-extract, count-based, interactive).

use botticelli_error::{BotticelliResult, ChatError, ChatErrorKind};
use botticelli_mcp::{
    ElicitationDialog, NarrativeHelper, PartialAct, PartialNarrative, PartialNarrativeBuilder,
};
use elicitation::Elicitation;
use std::collections::HashMap;
use tracing::{debug, instrument};

use super::infrastructure::create_mcp_client_for_dialog;
use super::types::{ActApproach, ActDefinition};

/// Elicit acts using paradigm-based approach.
///
/// This function demonstrates the refactored pattern:
/// 1. Create MCP client with dialog
/// 2. Use ActApproach::elicit() to select workflow (Select paradigm)
/// 3. Dispatch to approach-specific functions
/// 4. Update PartialNarrative
///
/// Compare to original ActElicitor which:
/// - Manually calls dialog.ask_choice() for approach selection
/// - Has explicit loops with dialog.ask_text() calls
/// - Manual validation with dialog.show_error()
/// - String indexing for approach selection
///
/// This version uses:
/// - ActApproach enum with Select paradigm
/// - ActDefinition with Survey paradigm for one-by-one
/// - Type-safe approach dispatch
#[instrument(skip(dialog, partial))]
pub async fn elicit_acts(
    dialog: Box<dyn ElicitationDialog>,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    // 1. Create MCP client with primitive elicitation tools
    let client = create_mcp_client_for_dialog(dialog).await?;

    // 2. Select approach using Select paradigm - replaces manual ask_choice!
    let approach = ActApproach::elicit(&client).await.map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to elicit act approach: {}",
            e
        )))
    })?;

    debug!(approach = ?approach, "Act approach selected");

    // 3. Get acts based on approach
    let (act_order, acts) = match approach {
        ActApproach::AutoExtract => extract_from_description(partial).await?,
        ActApproach::ManualCount => count_based_specification(&client).await?,
        ActApproach::Interactive => one_by_one_specification(&client).await?,
    };

    debug!(act_count = act_order.len(), "Acts defined");

    // 4. Update partial narrative
    let updated = PartialNarrativeBuilder::default()
        .name(partial.name().clone())
        .description(partial.description().clone())
        .model(partial.model().clone())
        .temperature(*partial.temperature())
        .max_tokens(*partial.max_tokens())
        .act_order(act_order)
        .acts(acts)
        .build()
        .map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to build partial narrative: {}",
                e
            )))
        })?;

    *partial = updated;

    Ok(())
}

/// Extract acts from description using NarrativeHelper.
///
/// This approach is unchanged from original as it uses the helper function.
/// No refactoring needed - already optimal.
#[instrument(skip(partial))]
async fn extract_from_description(
    partial: &PartialNarrative,
) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
    let description = partial.description().as_ref().ok_or_else(|| {
        ChatError::new(ChatErrorKind::InvalidState(
            "Description required for auto-extract".to_string(),
        ))
    })?;

    let extracted_acts = NarrativeHelper::extract_acts_from_description(description);

    if extracted_acts.is_empty() {
        return Err(ChatError::new(ChatErrorKind::InvalidState(
            "No acts extracted from description".to_string(),
        ))
        .into());
    }

    let mut act_order = Vec::new();
    let mut acts = HashMap::new();

    for extracted in extracted_acts {
        act_order.push(extracted.name.clone());
        acts.insert(
            extracted.name,
            PartialAct::new(extracted.prompt, None, None, Vec::new(), None),
        );
    }

    debug!(count = act_order.len(), "Extracted acts from description");
    Ok((act_order, acts))
}

/// Count-based specification using MCP primitive tools.
///
/// Refactored to use MCP tool calls directly. In a future iteration,
/// we could create helper functions to wrap the tool calling pattern.
#[instrument(skip(client))]
async fn count_based_specification(
    client: &pmcp::Client<botticelli_mcp::InProcTransport>,
) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
    // Elicit count using primitive tool
    let count_result = client
        .call_tool(
            "elicit_number".to_string(),
            serde_json::json!({
                "prompt": "How many acts?",
                "min": 1,
                "max": 100
            }),
        )
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to elicit act count: {}",
                e
            )))
        })?;

    let count = extract_value_as_i64(&count_result)? as usize;

    let mut act_order = Vec::new();
    let mut acts = HashMap::new();

    for i in 0..count {
        let act_name = format!("act{}", i + 1);

        // Elicit prompt using primitive tool
        let prompt_result = client
            .call_tool(
                "elicit_text".to_string(),
                serde_json::json!({
                    "prompt": format!("Enter prompt for {} (step {}/{}):", act_name, i + 1, count)
                }),
            )
            .await
            .map_err(|e| {
                ChatError::new(ChatErrorKind::InvalidState(format!(
                    "Failed to elicit prompt: {}",
                    e
                )))
            })?;

        let prompt = extract_value_as_string(&prompt_result)?;

        act_order.push(act_name.clone());
        acts.insert(
            act_name,
            PartialAct::new(prompt, None, None, Vec::new(), None),
        );
    }

    debug!(count, "Defined acts via count-based specification");
    Ok((act_order, acts))
}

/// One-by-one specification using ActDefinition::elicit().
///
/// This is where the biggest code reduction happens!
/// Original: ~40 lines with manual dialog.ask_text() calls and validation loops
/// Refactored: ~25 lines using ActDefinition::elicit()
///
/// Note: Post-validation is done after elicitation. In a production version,
/// we could add error display by keeping a reference to dialog or using
/// tool-based error messages.
#[instrument(skip(client))]
async fn one_by_one_specification(
    client: &pmcp::Client<botticelli_mcp::InProcTransport>,
) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
    let mut act_order = Vec::new();
    let mut acts = HashMap::new();

    loop {
        // Use ActDefinition::elicit() - this ONE LINE replaces multiple dialog calls!
        let act_def = ActDefinition::elicit(client).await.map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to elicit act definition: {}",
                e
            )))
        })?;

        // Post-elicitation validation
        if !NarrativeHelper::is_valid_name(&act_def.name) {
            // In the original, we show error via dialog.show_error()
            // In refactored version, validation happens after collection
            // Future: could add tool-based error display
            debug!(name = %act_def.name, "Invalid act name");
            continue;
        }

        if acts.contains_key(&act_def.name) {
            debug!(name = %act_def.name, "Duplicate act name");
            continue;
        }

        // Add act
        act_order.push(act_def.name.clone());
        acts.insert(
            act_def.name,
            PartialAct::new(act_def.prompt, None, None, Vec::new(), None),
        );

        // Ask if user wants to add another
        let continue_result = client
            .call_tool(
                "elicit_bool".to_string(),
                serde_json::json!({
                    "prompt": "Add another act?",
                    "default": true
                }),
            )
            .await
            .map_err(|e| {
                ChatError::new(ChatErrorKind::InvalidState(format!(
                    "Failed to elicit continuation: {}",
                    e
                )))
            })?;

        let should_continue = extract_value_as_bool(&continue_result)?;

        if !should_continue {
            break;
        }
    }

    debug!(
        count = act_order.len(),
        "Defined acts via one-by-one specification"
    );
    Ok((act_order, acts))
}

/// Extract value from tool result.
///
/// This helper follows the pattern from elicitation_integration_test.rs:
/// 1. Get first content item
/// 2. Serialize to JSON to access the text field
/// 3. Parse the text field as JSON to get the actual value
fn extract_value(result: &pmcp::types::CallToolResult) -> Result<serde_json::Value, ChatError> {
    if result.content.is_empty() {
        return Err(ChatError::new(ChatErrorKind::InvalidState(
            "Empty content in tool response".to_string(),
        )));
    }

    let content_json = &result.content[0];
    let content_str = serde_json::to_string(&content_json).map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to serialize content: {}",
            e
        )))
    })?;

    let content_val: serde_json::Value = serde_json::from_str(&content_str).map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to parse content: {}",
            e
        )))
    })?;

    let result_text = content_val["text"].as_str().ok_or_else(|| {
        ChatError::new(ChatErrorKind::InvalidState(
            "Expected text field in content".to_string(),
        ))
    })?;

    serde_json::from_str(result_text).map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to parse tool result: {}",
            e
        )))
    })
}

/// Extract value as string from tool result.
fn extract_value_as_string(result: &pmcp::types::CallToolResult) -> Result<String, ChatError> {
    extract_value(result)?
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            ChatError::new(ChatErrorKind::InvalidState(
                "Expected string value in tool result".to_string(),
            ))
        })
}

/// Extract value as i64 from tool result.
fn extract_value_as_i64(result: &pmcp::types::CallToolResult) -> Result<i64, ChatError> {
    extract_value(result)?.as_i64().ok_or_else(|| {
        ChatError::new(ChatErrorKind::InvalidState(
            "Expected number value in tool result".to_string(),
        ))
    })
}

/// Extract value as bool from tool result.
fn extract_value_as_bool(result: &pmcp::types::CallToolResult) -> Result<bool, ChatError> {
    extract_value(result)?.as_bool().ok_or_else(|| {
        ChatError::new(ChatErrorKind::InvalidState(
            "Expected boolean value in tool result".to_string(),
        ))
    })
}
