//! Metadata elicitor for narrative [narrative] section.

use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative, PartialNarrativeBuilder};
use crate::{ChatError, ChatErrorKind, ChatResult};
use async_trait::async_trait;
use botticelli_mcp::NarrativeHelper;
use tracing::{debug, instrument};

/// Elicits narrative metadata (name, description, model, temperature, etc.).
///
/// Prerequisites: None (can always run first)
pub struct MetadataElicitor;

impl MetadataElicitor {
    /// Create a new metadata elicitor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetadataElicitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NarrativeElicitor for MetadataElicitor {
    fn name(&self) -> &str {
        "Metadata"
    }

    fn description(&self) -> &str {
        "Gather basic narrative information (name, description, model settings)"
    }

    fn can_run(&self, _partial: &PartialNarrative) -> bool {
        true // No prerequisites
    }

    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> ChatResult<()> {
        dialog.show_info("Let's create a new narrative!").await?;

        // Name
        let name = loop {
            let input = dialog.ask_text("Enter narrative name (alphanumeric and underscores):").await?;
            
            if NarrativeHelper::is_valid_name(&input) {
                break input;
            }
            
            dialog.show_error("Invalid name. Must start with letter, contain only alphanumeric and underscores, max 64 chars.").await?;
        };

        debug!(name = %name, "Narrative name validated");

        // Description
        let description = dialog.ask_text("Enter narrative description (what does this workflow do?):").await?;

        // Optional: Default model
        let model = if dialog.ask_confirmation("Set a default model for all acts?", false).await? {
            let model_options = &[
                "gemini-2.0-flash-exp",
                "gemini-1.5-flash",
                "gemini-1.5-pro",
                "claude-3-5-sonnet-20241022",
                "gpt-4o",
                "Custom (enter manually)",
            ];
            
            let choice = dialog.ask_choice("Select default model:", model_options).await?;
            
            if choice == model_options.len() - 1 {
                // Custom
                Some(dialog.ask_text("Enter model name:").await?)
            } else {
                Some(model_options[choice].to_string())
            }
        } else {
            None
        };

        // Optional: Temperature
        let temperature = if dialog.ask_confirmation("Set a default temperature?", false).await? {
            let temp_f64 = dialog.ask_number("Enter temperature (0-20, will be divided by 10):", 0, 20).await? as f64 / 10.0;
            Some(temp_f64)
        } else {
            None
        };

        // Optional: Max tokens
        let max_tokens = if dialog.ask_confirmation("Set a default max_tokens?", false).await? {
            let tokens = dialog.ask_number("Enter max tokens:", 1, 1000000).await?;
            Some(tokens as u32)
        } else {
            None
        };

        // Update partial narrative using builder
        let updated = PartialNarrativeBuilder::default()
            .name(name)
            .description(description)
            .model(model)
            .temperature(temperature)
            .max_tokens(max_tokens)
            .act_order(partial.act_order().clone())
            .acts(partial.acts().clone())
            .build()
            .map_err(|e| {
                ChatError::new(ChatErrorKind::InvalidState(format!("Failed to build partial narrative: {}", e)))
            })?;

        *partial = updated;

        dialog.show_info(&format!("✓ Metadata complete for '{}'", partial.name().as_ref().unwrap())).await?;

        Ok(())
    }

    fn is_complete(&self, partial: &PartialNarrative) -> bool {
        partial.name().is_some() && partial.description().is_some()
    }

    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        if self.is_complete(partial) {
            Some("Add acts to define the workflow steps".to_string())
        } else {
            None
        }
    }
}
