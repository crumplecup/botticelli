//! ElicitationSession orchestrator for managing the elicitation flow.

use botticelli_error::BotticelliResult;


use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative, PartialNarrativeBuilder};
use botticelli_error::{ChatError, ChatErrorKind};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Orchestrates the elicitation process.
///
/// Manages the flow of elicitors, tracks state, and coordinates
/// the overall narrative creation process.
pub struct ElicitationSession {
    /// Current partial narrative being constructed.
    partial: PartialNarrative,
    
    /// Ordered list of elicitors to run.
    elicitors: Vec<Arc<dyn NarrativeElicitor>>,
    
    /// Current elicitor index.
    current_index: usize,
    
    /// Whether session is complete.
    completed: bool,
}

impl ElicitationSession {
    /// Create a new elicitation session with specified elicitors.
    ///
    /// Elicitors run in the order provided. Prerequisites are checked
    /// before each elicitor runs.
    #[instrument(skip(elicitors))]
    pub fn new(elicitors: Vec<Arc<dyn NarrativeElicitor>>) -> Self {
        info!(elicitor_count = elicitors.len(), "Creating elicitation session");
        
        Self {
            partial: PartialNarrativeBuilder::default().build().unwrap(),
            elicitors,
            current_index: 0,
            completed: false,
        }
    }

    /// Run the complete elicitation flow.
    ///
    /// Executes each elicitor in sequence until all are complete.
    #[instrument(skip(self, dialog))]
    pub async fn run(&mut self, dialog: &mut dyn ElicitationDialog) -> BotticelliResult<()> {
        info!("Starting elicitation session");

        while self.current_index < self.elicitors.len() {
            let elicitor = &self.elicitors[self.current_index];

            // Check prerequisites
            if !elicitor.can_run(&self.partial) {
                warn!(
                    elicitor = elicitor.name(),
                    "Elicitor prerequisites not met, skipping"
                );
                
                dialog.show_warning(&format!(
                    "Skipping '{}' - prerequisites not met",
                    elicitor.name()
                )).await?;
                
                self.current_index += 1;
                continue;
            }

            // Show progress
            dialog.show_progress(
                self.current_index + 1,
                self.elicitors.len(),
                &format!("Running: {}", elicitor.description()),
            ).await?;

            // Run elicitor
            debug!(
                elicitor = elicitor.name(),
                index = self.current_index,
                "Running elicitor"
            );

            match elicitor.elicit(dialog, &mut self.partial).await {
                Ok(()) => {
                    info!(elicitor = elicitor.name(), "Elicitor completed successfully");
                    
                    // Show suggestion if available
                    if let Some(suggestion) = elicitor.suggest_next(&self.partial) {
                        dialog.show_info(&format!("💡 Next: {}", suggestion)).await?;
                    }
                }
                Err(e) => {
                    warn!(
                        elicitor = elicitor.name(),
                        error = ?e,
                        "Elicitor failed"
                    );
                    
                    dialog.show_error(&format!(
                        "Failed during '{}': {}",
                        elicitor.name(),
                        e
                    )).await?;

                    // Ask if user wants to retry or skip
                    let retry = dialog.ask_confirmation("Retry this step?", true).await?;
                    
                    if retry {
                        continue; // Don't increment index, retry same elicitor
                    } else {
                        // Skip to next
                        dialog.show_warning(&format!(
                            "Skipping '{}'. You may need to complete this manually.",
                            elicitor.name()
                        )).await?;
                    }
                }
            }

            self.current_index += 1;
        }

        self.completed = true;
        info!("Elicitation session completed");

        Ok(())
    }

    /// Run a single elicitor step.
    ///
    /// Useful for manual control or step-by-step execution.
    #[instrument(skip(self, dialog))]
    pub async fn step(&mut self, dialog: &mut dyn ElicitationDialog) -> BotticelliResult<bool> {
        if self.current_index >= self.elicitors.len() {
            self.completed = true;
            return Ok(false); // No more steps
        }

        let elicitor = &self.elicitors[self.current_index];

        if !elicitor.can_run(&self.partial) {
            warn!(
                elicitor = elicitor.name(),
                "Elicitor prerequisites not met"
            );
            return Err(ChatError::new(ChatErrorKind::InvalidState(format!(
                "Cannot run '{}' - prerequisites not met",
                elicitor.name()
            ))).into());
        }

        dialog.show_progress(
            self.current_index + 1,
            self.elicitors.len(),
            &format!("Running: {}", elicitor.description()),
        ).await?;

        elicitor.elicit(dialog, &mut self.partial).await?;

        self.current_index += 1;

        Ok(self.current_index < self.elicitors.len())
    }

    /// Check if session is complete.
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// Get the current partial narrative.
    pub fn partial(&self) -> &PartialNarrative {
        &self.partial
    }

    /// Get mutable access to partial narrative.
    pub fn partial_mut(&mut self) -> &mut PartialNarrative {
        &mut self.partial
    }

    /// Consume session and return final partial narrative.
    pub fn into_partial(self) -> PartialNarrative {
        self.partial
    }

    /// Get current progress (completed steps / total steps).
    pub fn progress(&self) -> (usize, usize) {
        (self.current_index, self.elicitors.len())
    }

    /// Check if narrative has minimum required fields.
    pub fn has_minimum_required(&self) -> bool {
        self.partial.has_minimum_required()
    }

    /// Preview generated TOML.
    #[instrument(skip(self, dialog))]
    pub async fn preview(&self, dialog: &mut dyn ElicitationDialog) -> BotticelliResult<()> {
        if !self.has_minimum_required() {
            dialog.show_warning("Narrative is incomplete. Preview may fail.").await?;
        }

        match self.partial.to_toml() {
            Ok(toml) => {
                dialog.show_preview(&toml).await?;
                Ok(())
            }
            Err(e) => {
                dialog.show_error(&format!("Failed to generate TOML: {}", e)).await?;
                Err(e.into())
            }
        }
    }

    /// Validate current narrative state.
    #[instrument(skip(self, dialog))]
    pub async fn validate(&self, dialog: &mut dyn ElicitationDialog) -> BotticelliResult<()> {
        if !self.has_minimum_required() {
            dialog.show_warning("Narrative is incomplete. Validation may fail.").await?;
        }

        let validation = self.partial.validate()?;

        if validation.is_valid() {
            dialog.show_info("✅ Narrative is valid!").await?;
        } else {
            let mut msg = format!(
                "❌ Validation failed:\n  {} error(s)\n  {} warning(s)\n",
                validation.errors.len(),
                validation.warnings.len()
            );

            for (i, error) in validation.errors.iter().enumerate() {
                msg.push_str(&format!("\n{}. {}", i + 1, error.message));
            }

            dialog.show_validation(&msg).await?;
        }

        Ok(())
    }

    /// Finalize and convert to complete Narrative.
    #[instrument(skip(self))]
    pub fn finalize(&self) -> BotticelliResult<botticelli_narrative::Narrative> {
        if !self.completed {
            warn!("Attempting to finalize incomplete session");
        }

        if !self.has_minimum_required() {
            return Err(ChatError::new(ChatErrorKind::ValidationError(
                "Narrative missing required fields".to_string(),
            )).into());
        }

        self.partial.try_into_narrative().map_err(|e| e.into())
    }
}

impl Default for ElicitationSession {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
