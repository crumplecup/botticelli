//! Act elicitor for narrative acts and ordering.

use botticelli_error::BotticelliResult;


use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialAct, PartialNarrative, PartialNarrativeBuilder};
use botticelli_error::{ChatError, ChatErrorKind};
use async_trait::async_trait;
use botticelli_mcp::NarrativeHelper;
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Elicits acts and their execution order.
///
/// Prerequisites: Metadata complete (name, description)
pub struct ActElicitor;

impl ActElicitor {
    /// Create a new act elicitor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ActElicitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NarrativeElicitor for ActElicitor {
    fn name(&self) -> &str {
        "Acts"
    }

    fn description(&self) -> &str {
        "Define workflow acts and their execution order"
    }

    fn can_run(&self, partial: &PartialNarrative) -> bool {
        // Requires metadata
        partial.name().is_some() && partial.description().is_some()
    }

    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> BotticelliResult<()> {
        dialog.show_info("Let's define the workflow acts.").await?;

        let approach_options = &[
            "Auto-extract from description",
            "Manual count-based specification",
            "Enter acts one-by-one",
        ];

        let approach = dialog.ask_choice("How would you like to define acts?", approach_options).await?;

        let (act_order, acts) = match approach {
            0 => self.extract_from_description(dialog, partial).await?,
            1 => self.count_based_specification(dialog).await?,
            2 => self.one_by_one_specification(dialog).await?,
            _ => return Err(ChatError::new(ChatErrorKind::InvalidInput("Invalid approach".to_string())).into()),
        };

        debug!(act_count = act_order.len(), "Acts defined");

        // Update partial narrative
        let updated = PartialNarrativeBuilder::default()
            .name(partial.name().clone())
            .description(partial.description().clone())
            .model(partial.model().clone())
            .temperature(*partial.temperature())
            .max_tokens(*partial.max_tokens())
            .act_order(act_order.clone())
            .acts(acts)
            .build()
            .map_err(|e| {
                ChatError::new(ChatErrorKind::InvalidState(format!("Failed to build partial narrative: {}", e)))
            })?;

        *partial = updated;

        dialog.show_info(&format!("✓ Defined {} act(s)", act_order.len())).await?;

        Ok(())
    }

    fn is_complete(&self, partial: &PartialNarrative) -> bool {
        !partial.acts().is_empty() && !partial.act_order().is_empty()
    }

    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        if self.is_complete(partial) {
            Some("Preview or save the narrative".to_string())
        } else {
            None
        }
    }
}

impl ActElicitor {
    /// Extract acts from narrative description.
    #[instrument(skip(self, dialog, partial))]
    async fn extract_from_description(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &PartialNarrative,
    ) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
        let description = partial.description().as_ref().unwrap();
        
        dialog.show_info("Analyzing description to extract workflow steps...").await?;
        
        let extracted_acts = NarrativeHelper::extract_acts_from_description(description);
        
        if extracted_acts.is_empty() {
            dialog.show_warning("Could not extract acts from description. Try another method.").await?;
            return Err(ChatError::new(ChatErrorKind::InvalidState("No acts extracted".to_string())).into());
        }

        // Show extracted acts
        let mut info = format!("Extracted {} act(s):\n", extracted_acts.len());
        for (i, act) in extracted_acts.iter().enumerate() {
            info.push_str(&format!("  {}. {} - {}\n", i + 1, act.name, act.prompt));
        }
        dialog.show_info(&info).await?;

        let confirmed = dialog.ask_confirmation("Use these acts?", true).await?;
        
        if !confirmed {
            return Err(ChatError::new(ChatErrorKind::InvalidState("User rejected extracted acts".to_string())).into());
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

        Ok((act_order, acts))
    }

    /// Count-based specification (user specifies count, we name acts).
    #[instrument(skip(self, dialog))]
    async fn count_based_specification(
        &self,
        dialog: &mut dyn ElicitationDialog,
    ) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
        let count = dialog.ask_number("How many acts?", 1, 100).await? as usize;

        let mut act_order = Vec::new();
        let mut acts = HashMap::new();

        for i in 0..count {
            let act_name = format!("act{}", i + 1);
            let prompt = dialog.ask_text(&format!("Enter prompt for {} (step {}/{}):", act_name, i + 1, count)).await?;

            act_order.push(act_name.clone());
            acts.insert(act_name, PartialAct::new(prompt, None, None, Vec::new(), None));
        }

        Ok((act_order, acts))
    }

    /// One-by-one specification (user enters each act).
    #[instrument(skip(self, dialog))]
    async fn one_by_one_specification(
        &self,
        dialog: &mut dyn ElicitationDialog,
    ) -> BotticelliResult<(Vec<String>, HashMap<String, PartialAct>)> {
        let mut act_order = Vec::new();
        let mut acts = HashMap::new();

        loop {
            let act_name = dialog.ask_text(&format!("Enter act name (act {} of ?):", act_order.len() + 1)).await?;
            
            if !NarrativeHelper::is_valid_name(&act_name) {
                dialog.show_error("Invalid act name. Must start with letter, alphanumeric + underscores only.").await?;
                continue;
            }

            if acts.contains_key(&act_name) {
                dialog.show_error("Act name already exists. Choose a different name.").await?;
                continue;
            }

            let prompt = dialog.ask_text(&format!("Enter prompt for '{}':", act_name)).await?;

            act_order.push(act_name.clone());
            acts.insert(act_name, PartialAct::new(prompt, None, None, Vec::new(), None));

            if !dialog.ask_confirmation("Add another act?", true).await? {
                break;
            }
        }

        Ok((act_order, acts))
    }
}
