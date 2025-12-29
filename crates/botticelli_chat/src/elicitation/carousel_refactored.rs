//! Refactored carousel elicitation using elicitation crate paradigms.
//!
//! This refactored version demonstrates code reduction through the use of:
//! - #[derive(Elicit)] on CarouselConfig type for automatic form elicitation
//! - Shared MCP infrastructure for primitive elicitation
//! - Single `.elicit()` call replacing multiple manual dialog calls
//!
//! **Original**: ~80 lines with manual dialog.ask_*() calls
//! **Refactored**: ~60 lines with single CarouselConfig::elicit() call
//! **Expected reduction**: ~25% overall, ~67% for elicitation logic

use botticelli_error::{BotticelliResult, ChatError, ChatErrorKind};
use botticelli_mcp::{ElicitationDialog, PartialNarrative, PartialNarrativeBuilder};
use elicitation::Elicitation;
use tracing::{debug, instrument};

use super::infrastructure::create_mcp_client_for_dialog;

/// Elicit carousel configuration using paradigm-based approach.
///
/// This function demonstrates the refactored pattern:
/// 1. Create MCP client with dialog
/// 2. Ask if user wants to enable carousel (using elicit_bool primitive)
/// 3. Use CarouselConfig::elicit() for automatic form collection
/// 4. Convert types and update PartialNarrative
///
/// Compare to original which:
/// - Manually calls dialog.ask_confirmation() for enable check
/// - Manually calls dialog.ask_number() for iterations
/// - Manually calls dialog.ask_number() for estimated_tokens
/// - Manually calls dialog.ask_confirmation() for continue_on_error
/// - Manual CarouselConfig::new() construction
///
/// This version uses:
/// - Primitive MCP tool for enable check
/// - CarouselConfig::elicit() for all three fields (Survey paradigm)
/// - Type-safe result with automatic field collection
#[instrument(skip(dialog, partial))]
pub async fn elicit_carousel_refactored(
    dialog: Box<dyn ElicitationDialog>,
    partial: &mut PartialNarrative,
    target_act: Option<String>,
) -> BotticelliResult<()> {
    // 1. Create MCP client with primitive elicitation tools
    let client = create_mcp_client_for_dialog(dialog).await?;

    let scope_name = target_act.as_deref().unwrap_or("narrative");

    // 2. Ask if user wants to enable carousel
    let enable_result = client
        .call_tool(
            "elicit_bool".to_string(),
            serde_json::json!({
                "prompt": format!("Enable carousel for {}?", scope_name),
                "default": false
            }),
        )
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to elicit carousel enable: {}",
                e
            )))
        })?;

    let enable = extract_value_as_bool(&enable_result)?;

    if !enable {
        debug!(scope = %scope_name, "Carousel disabled, skipping configuration");
        return Ok(());
    }

    // 3. Elicit configuration using derive macro - ONE LINE replaces 3 dialog calls!
    let config = super::types::CarouselConfig::elicit(&client)
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to elicit carousel configuration: {}",
                e
            )))
        })?;

    debug!(
        scope = %scope_name,
        iterations = config.iterations,
        estimated_tokens = config.estimated_tokens,
        continue_on_error = config.continue_on_error,
        "Carousel configuration elicited"
    );

    // 4. Convert from elicitation CarouselConfig to narrative CarouselConfig
    let narrative_config = botticelli_narrative::CarouselConfig::new(
        config.iterations as u32,
        config.estimated_tokens as u64,
    )
    .with_continue_on_error(config.continue_on_error);

    // 5. Update partial narrative based on target scope
    match target_act {
        None => {
            // Narrative-level carousel
            let updated = PartialNarrativeBuilder::default()
                .name(partial.name().clone())
                .description(partial.description().clone())
                .model(partial.model().clone())
                .temperature(*partial.temperature())
                .max_tokens(*partial.max_tokens())
                .act_order(partial.act_order().clone())
                .acts(partial.acts().clone())
                .carousel(Some(narrative_config))
                .build()
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::InvalidState(format!(
                        "Failed to build partial narrative: {}",
                        e
                    )))
                })?;

            *partial = updated;
            debug!("Narrative-level carousel configured");
        }
        Some(act_name) => {
            // Act-level carousel
            if !partial.acts().contains_key(&act_name) {
                return Err(ChatError::new(ChatErrorKind::InvalidState(format!(
                    "Act '{}' not found",
                    act_name
                )))
                .into());
            }

            let mut updated_acts = partial.acts().clone();
            if let Some(act) = updated_acts.get_mut(&act_name) {
                act.carousel = Some(narrative_config);
            }

            let updated = PartialNarrativeBuilder::default()
                .name(partial.name().clone())
                .description(partial.description().clone())
                .model(partial.model().clone())
                .temperature(*partial.temperature())
                .max_tokens(*partial.max_tokens())
                .act_order(partial.act_order().clone())
                .acts(updated_acts)
                .build()
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::InvalidState(format!(
                        "Failed to build partial narrative: {}",
                        e
                    )))
                })?;

            *partial = updated;
            debug!(act = %act_name, "Act-level carousel configured");
        }
    }

    Ok(())
}

/// Extract value as bool from tool result.
///
/// This helper follows the pattern from elicitation_integration_test.rs.
fn extract_value_as_bool(result: &pmcp::types::CallToolResult) -> Result<bool, ChatError> {
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

    let value: serde_json::Value = serde_json::from_str(result_text).map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to parse tool result: {}",
            e
        )))
    })?;

    value.as_bool().ok_or_else(|| {
        ChatError::new(ChatErrorKind::InvalidState(
            "Expected boolean value in tool result".to_string(),
        ))
    })
}
