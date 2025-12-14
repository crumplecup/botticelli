//! Carousel elicitor for narrative looping configuration.

use botticelli_error::BotticelliResult;

use async_trait::async_trait;
use botticelli_error::{ChatError, ChatErrorKind};
use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative};
use botticelli_narrative::CarouselConfig;
use tracing::{debug, instrument};

/// Elicits carousel configuration for narratives or acts.
///
/// Carousels enable iterative execution with:
/// - Configurable iteration count
/// - Token estimation for budget control
/// - Continue-on-error behavior
/// - Optional budget multipliers (feature-gated)
///
/// Prerequisites: Metadata for narrative-level, acts for act-level
pub struct CarouselElicitor {
    /// Target scope (None = narrative-level, Some = act-level)
    target_act: Option<String>,
}

impl CarouselElicitor {
    /// Create carousel elicitor for narrative-level configuration.
    pub fn for_narrative() -> Self {
        Self { target_act: None }
    }

    /// Create carousel elicitor for specific act.
    pub fn for_act(act_name: String) -> Self {
        Self {
            target_act: Some(act_name),
        }
    }

    /// Elicit carousel configuration interactively.
    #[instrument(skip(self, dialog))]
    async fn elicit_carousel_config(
        &self,
        dialog: &mut dyn ElicitationDialog,
        scope_name: &str,
    ) -> BotticelliResult<CarouselConfig> {
        dialog
            .show_info(&format!("Configuring carousel for {}", scope_name))
            .await?;

        // Iterations
        let iterations = dialog
            .ask_number("Number of iterations (1-1000):", 1, 1000)
            .await? as u32;

        // Estimated tokens per iteration
        let estimated_tokens = dialog
            .ask_number("Estimated tokens per iteration:", 100, 1000000)
            .await? as u64;

        // Continue on error
        let continue_on_error = dialog
            .ask_confirmation("Continue execution if an iteration fails?", false)
            .await?;

        let config = CarouselConfig::new(iterations, estimated_tokens)
            .with_continue_on_error(continue_on_error);

        debug!(
            scope = %scope_name,
            iterations = iterations,
            estimated_tokens = estimated_tokens,
            continue_on_error = continue_on_error,
            "Created carousel configuration"
        );

        Ok(config)
    }
}

impl Default for CarouselElicitor {
    fn default() -> Self {
        Self::for_narrative()
    }
}

#[async_trait]
impl NarrativeElicitor for CarouselElicitor {
    fn name(&self) -> &str {
        "Carousel"
    }

    fn description(&self) -> &str {
        match &self.target_act {
            None => "Configure narrative-level carousel (iterative execution)",
            Some(_) => "Configure act-level carousel",
        }
    }

    fn can_run(&self, partial: &PartialNarrative) -> bool {
        match &self.target_act {
            None => {
                // Narrative-level: requires basic metadata
                partial.name().is_some()
            }
            Some(act_name) => {
                // Act-level: requires the act to exist
                partial.acts().contains_key(act_name)
            }
        }
    }

    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> BotticelliResult<()> {
        use botticelli_mcp::PartialNarrativeBuilder;

        match &self.target_act {
            None => {
                // Narrative-level carousel
                dialog
                    .show_info("Configure narrative-level carousel (iterative execution)")
                    .await?;

                let enable = dialog
                    .ask_confirmation("Enable carousel for entire narrative?", false)
                    .await?;

                if !enable {
                    dialog
                        .show_info("Skipping narrative-level carousel")
                        .await?;
                    return Ok(());
                }

                let config = self.elicit_carousel_config(dialog, "narrative").await?;

                // Update partial narrative with carousel
                let updated = PartialNarrativeBuilder::default()
                    .name(partial.name().clone())
                    .description(partial.description().clone())
                    .model(partial.model().clone())
                    .temperature(*partial.temperature())
                    .max_tokens(*partial.max_tokens())
                    .act_order(partial.act_order().clone())
                    .acts(partial.acts().clone())
                    .carousel(Some(config))
                    .build()
                    .map_err(|e| {
                        ChatError::new(ChatErrorKind::InvalidState(format!(
                            "Failed to build partial narrative: {}",
                            e
                        )))
                    })?;

                *partial = updated;

                dialog
                    .show_info("✓ Narrative-level carousel configured")
                    .await?;
            }
            Some(act_name) => {
                // Act-level carousel
                if !partial.acts().contains_key(act_name) {
                    return Err(ChatError::new(ChatErrorKind::InvalidState(format!(
                        "Act '{}' not found",
                        act_name
                    )))
                    .into());
                }

                dialog
                    .show_info(&format!("Configure carousel for act '{}'", act_name))
                    .await?;

                let enable = dialog
                    .ask_confirmation(&format!("Enable carousel for act '{}'?", act_name), false)
                    .await?;

                if !enable {
                    dialog
                        .show_info(&format!("Skipping carousel for act '{}'", act_name))
                        .await?;
                    return Ok(());
                }

                let config = self.elicit_carousel_config(dialog, act_name).await?;

                // Update the act with carousel config
                let mut updated_acts = partial.acts().clone();
                if let Some(act) = updated_acts.get_mut(act_name) {
                    act.carousel = Some(config);
                }

                // Rebuild partial narrative
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

                dialog
                    .show_info(&format!("✓ Carousel configured for act '{}'", act_name))
                    .await?;
            }
        }

        Ok(())
    }

    fn is_complete(&self, _partial: &PartialNarrative) -> bool {
        // Carousel is optional, so always complete
        true
    }

    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        if partial.name().is_none() {
            Some("Add metadata before configuring carousel".to_string())
        } else if partial.acts().is_empty() {
            Some("Add acts before configuring act-level carousels".to_string())
        } else {
            Some("Validate and finalize narrative".to_string())
        }
    }
}
