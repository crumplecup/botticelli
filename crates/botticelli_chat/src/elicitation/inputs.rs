//! Input elicitor for narrative act inputs.

use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative};
use crate::{ChatError, ChatErrorKind, ChatResult};
use async_trait::async_trait;
use botticelli_core::{HistoryRetention, Input, MediaSource, TableFormat};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Elicits inputs for narrative acts.
///
/// Supports all Input enum variants:
/// - Text (simple prompt)
/// - Image, Audio, Video, Document (multimodal)
/// - BotCommand (platform commands)
/// - Table (database queries)
/// - Narrative (composition)
///
/// Prerequisites: Acts must be defined
pub struct InputElicitor {
    /// Target act name (None = all acts)
    act_name: Option<String>,
}

impl InputElicitor {
    /// Create input elicitor for a specific act.
    pub fn for_act(act_name: String) -> Self {
        Self {
            act_name: Some(act_name),
        }
    }

    /// Create input elicitor for all acts.
    pub fn for_all_acts() -> Self {
        Self { act_name: None }
    }

    /// Elicit inputs for a single act.
    #[instrument(skip(self, dialog))]
    async fn elicit_act_inputs(
        &self,
        dialog: &mut dyn ElicitationDialog,
        act_name: &str,
    ) -> ChatResult<Vec<Input>> {
        dialog
            .show_info(&format!("Configuring inputs for act '{}'", act_name))
            .await?;

        let mut inputs = Vec::new();

        loop {
            if !inputs.is_empty() {
                let add_more = dialog
                    .ask_confirmation("Add another input to this act?", false)
                    .await?;

                if !add_more {
                    break;
                }
            }

            let input_type_options = &[
                "Text prompt",
                "Image",
                "Audio",
                "Video",
                "Document",
                "Bot command",
                "Database table query",
                "Narrative reference",
            ];

            let choice = dialog
                .ask_choice("Select input type:", input_type_options)
                .await?;

            let input = match choice {
                0 => self.elicit_text(dialog).await?,
                1 => self.elicit_image(dialog).await?,
                2 => self.elicit_audio(dialog).await?,
                3 => self.elicit_video(dialog).await?,
                4 => self.elicit_document(dialog).await?,
                5 => self.elicit_bot_command(dialog).await?,
                6 => self.elicit_table(dialog).await?,
                7 => self.elicit_narrative(dialog).await?,
                _ => {
                    return Err(ChatError::new(ChatErrorKind::InvalidInput(
                        "Invalid input type choice".to_string(),
                    )))
                }
            };

            inputs.push(input);

            if inputs.is_empty() {
                let skip = dialog
                    .ask_confirmation("Act has no inputs. Continue anyway?", false)
                    .await?;

                if !skip {
                    continue;
                }
            }

            break;
        }

        Ok(inputs)
    }

    /// Elicit text input.
    #[instrument(skip(self, dialog))]
    async fn elicit_text(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        let text = dialog.ask_text("Enter text prompt:").await?;
        Ok(Input::Text(text))
    }

    /// Elicit image input.
    #[instrument(skip(self, dialog))]
    async fn elicit_image(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        let source = self.elicit_media_source(dialog, "image").await?;
        let mime = if dialog
            .ask_confirmation("Specify MIME type?", false)
            .await?
        {
            Some(dialog.ask_text("Enter MIME type (e.g., image/png):").await?)
        } else {
            None
        };

        Ok(Input::Image { mime, source })
    }

    /// Elicit audio input.
    #[instrument(skip(self, dialog))]
    async fn elicit_audio(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        let source = self.elicit_media_source(dialog, "audio").await?;
        let mime = if dialog
            .ask_confirmation("Specify MIME type?", false)
            .await?
        {
            Some(dialog.ask_text("Enter MIME type (e.g., audio/mp3):").await?)
        } else {
            None
        };

        Ok(Input::Audio { mime, source })
    }

    /// Elicit video input.
    #[instrument(skip(self, dialog))]
    async fn elicit_video(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        let source = self.elicit_media_source(dialog, "video").await?;
        let mime = if dialog
            .ask_confirmation("Specify MIME type?", false)
            .await?
        {
            Some(dialog.ask_text("Enter MIME type (e.g., video/mp4):").await?)
        } else {
            None
        };

        Ok(Input::Video { mime, source })
    }

    /// Elicit document input.
    #[instrument(skip(self, dialog))]
    async fn elicit_document(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        let source = self.elicit_media_source(dialog, "document").await?;
        let mime = if dialog
            .ask_confirmation("Specify MIME type?", false)
            .await?
        {
            Some(dialog.ask_text("Enter MIME type (e.g., application/pdf):").await?)
        } else {
            None
        };

        let filename = if dialog.ask_confirmation("Specify filename?", false).await? {
            Some(dialog.ask_text("Enter filename:").await?)
        } else {
            None
        };

        Ok(Input::Document {
            mime,
            source,
            filename,
        })
    }

    /// Elicit media source (URL, file path, base64).
    #[instrument(skip(self, dialog))]
    async fn elicit_media_source(
        &self,
        dialog: &mut dyn ElicitationDialog,
        media_type: &str,
    ) -> ChatResult<MediaSource> {
        let source_options = &["URL", "Base64 data", "Binary data (raw bytes)"];

        let choice = dialog
            .ask_choice(
                &format!("How will you provide the {}?", media_type),
                source_options,
            )
            .await?;

        match choice {
            0 => {
                let url = dialog.ask_text("Enter URL:").await?;
                Ok(MediaSource::Url(url))
            }
            1 => {
                let data = dialog.ask_text("Enter base64 data:").await?;
                Ok(MediaSource::Base64(data))
            }
            2 => {
                dialog
                    .show_warning("Binary data input not supported in TUI - use URL or base64")
                    .await?;
                Err(ChatError::new(ChatErrorKind::InvalidInput(
                    "Binary data not supported in interactive mode".to_string(),
                )))
            }
            _ => Err(ChatError::new(ChatErrorKind::InvalidInput(
                "Invalid source choice".to_string(),
            ))),
        }
    }

    /// Elicit bot command input.
    #[instrument(skip(self, dialog))]
    async fn elicit_bot_command(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        dialog
            .show_info("Configure bot command execution")
            .await?;

        let platform = dialog
            .ask_text("Enter platform (e.g., discord, slack):")
            .await?;

        let command = dialog.ask_text("Enter command (e.g., server.get_stats):").await?;

        // Arguments
        let mut args = HashMap::new();
        if dialog
            .ask_confirmation("Add command arguments?", false)
            .await?
        {
            loop {
                let key = dialog.ask_text("Argument name (or empty to finish):").await?;
                if key.is_empty() {
                    break;
                }

                let value = dialog.ask_text(&format!("Value for '{}':", key)).await?;

                // Try to parse as JSON value
                let json_value = serde_json::from_str(&value)
                    .unwrap_or_else(|_| serde_json::Value::String(value));

                args.insert(key, json_value);
            }
        }

        let required = dialog
            .ask_confirmation("Is this command required (halt on failure)?", false)
            .await?;

        let cache_duration = if dialog
            .ask_confirmation("Enable caching?", false)
            .await?
        {
            let seconds = dialog
                .ask_number("Cache duration in seconds:", 0, 86400)
                .await?;
            Some(seconds as u64)
        } else {
            None
        };

        let history_retention = self.elicit_history_retention(dialog).await?;

        Ok(Input::BotCommand {
            platform,
            command,
            args,
            required,
            cache_duration,
            history_retention,
        })
    }

    /// Elicit table query input.
    #[instrument(skip(self, dialog))]
    async fn elicit_table(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        dialog.show_info("Configure database table query").await?;

        let table_name = dialog.ask_text("Enter table name:").await?;

        let columns = if dialog
            .ask_confirmation("Specify columns (default: all)?", false)
            .await?
        {
            let cols_str = dialog
                .ask_text("Enter column names (comma-separated):")
                .await?;
            Some(cols_str.split(',').map(|s| s.trim().to_string()).collect())
        } else {
            None
        };

        let where_clause = if dialog
            .ask_confirmation("Add WHERE clause?", false)
            .await?
        {
            Some(dialog.ask_text("Enter WHERE clause (without WHERE):").await?)
        } else {
            None
        };

        let limit = if dialog.ask_confirmation("Set row limit?", false).await? {
            let num = dialog.ask_number("Maximum rows:", 1, 10000).await?;
            Some(num as u32)
        } else {
            None
        };

        let offset = if dialog.ask_confirmation("Set offset?", false).await? {
            let num = dialog.ask_number("Offset:", 0, 1000000).await?;
            Some(num as u32)
        } else {
            None
        };

        let order_by = if dialog
            .ask_confirmation("Add ORDER BY clause?", false)
            .await?
        {
            Some(dialog.ask_text("Enter ORDER BY clause (without ORDER BY):").await?)
        } else {
            None
        };

        let alias = if dialog
            .ask_confirmation("Set alias for {{alias}} interpolation?", false)
            .await?
        {
            Some(dialog.ask_text("Enter alias:").await?)
        } else {
            None
        };

        let format_options = &["JSON", "Markdown", "CSV"];
        let format_choice = dialog
            .ask_choice("Select output format:", format_options)
            .await?;
        let format = match format_choice {
            0 => TableFormat::Json,
            1 => TableFormat::Markdown,
            2 => TableFormat::Csv,
            _ => TableFormat::Json,
        };

        let sample = if dialog
            .ask_confirmation("Random sample rows?", false)
            .await?
        {
            let num = dialog.ask_number("Sample size:", 1, 1000).await?;
            Some(num as u32)
        } else {
            None
        };

        let destructive_read = dialog
            .ask_confirmation(
                "Destructive read (pull and delete rows)?",
                false,
            )
            .await?;

        let history_retention = self.elicit_history_retention(dialog).await?;

        Ok(Input::Table {
            table_name,
            columns,
            where_clause,
            limit,
            offset,
            order_by,
            alias,
            format,
            sample,
            destructive_read,
            history_retention,
        })
    }

    /// Elicit narrative reference input.
    #[instrument(skip(self, dialog))]
    async fn elicit_narrative(&self, dialog: &mut dyn ElicitationDialog) -> ChatResult<Input> {
        dialog.show_info("Configure narrative reference").await?;

        let name = dialog
            .ask_text("Enter narrative name (without .toml):")
            .await?;

        let path = if dialog
            .ask_confirmation("Specify custom path?", false)
            .await?
        {
            Some(dialog.ask_text("Enter relative path:").await?)
        } else {
            None
        };

        let history_retention = self.elicit_history_retention(dialog).await?;

        Ok(Input::Narrative {
            name,
            path,
            history_retention,
        })
    }

    /// Elicit history retention setting.
    #[instrument(skip(self, dialog))]
    async fn elicit_history_retention(
        &self,
        dialog: &mut dyn ElicitationDialog,
    ) -> ChatResult<HistoryRetention> {
        let retention_options = &[
            "Full (keep entire content)",
            "Summary (keep summary only)",
            "Drop (discard after processing)",
        ];

        let choice = dialog
            .ask_choice("History retention mode:", retention_options)
            .await?;

        Ok(match choice {
            0 => HistoryRetention::Full,
            1 => HistoryRetention::Summary,
            2 => HistoryRetention::Drop,
            _ => HistoryRetention::Full,
        })
    }
}

impl Default for InputElicitor {
    fn default() -> Self {
        Self::for_all_acts()
    }
}

#[async_trait]
impl NarrativeElicitor for InputElicitor {
    fn name(&self) -> &str {
        "Inputs"
    }

    fn description(&self) -> &str {
        "Define inputs for narrative acts"
    }

    fn can_run(&self, partial: &PartialNarrative) -> bool {
        // Requires at least one act
        !partial.acts().is_empty()
    }

    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> ChatResult<()> {
        use botticelli_mcp::PartialNarrativeBuilder;

        dialog.show_info("Let's configure act inputs.").await?;

        // Determine which acts to configure
        let act_names: Vec<String> = if let Some(ref target) = self.act_name {
            // Specific act
            if !partial.acts().contains_key(target) {
                return Err(ChatError::new(ChatErrorKind::InvalidState(format!(
                    "Act '{}' not found",
                    target
                ))));
            }
            vec![target.clone()]
        } else {
            // All acts - ask user which ones to configure
            let mut acts_to_configure = Vec::new();
            for act_name in partial.act_order() {
                let configure = dialog
                    .ask_confirmation(
                        &format!("Configure inputs for act '{}'?", act_name),
                        true,
                    )
                    .await?;

                if configure {
                    acts_to_configure.push(act_name.clone());
                }
            }
            acts_to_configure
        };

        if act_names.is_empty() {
            dialog
                .show_info("No acts selected for input configuration.")
                .await?;
            return Ok(());
        }

        // Clone current acts and update with inputs
        let mut updated_acts = partial.acts().clone();

        // Elicit inputs for each act
        for act_name in &act_names {
            let inputs = self.elicit_act_inputs(dialog, act_name).await?;

            debug!(
                act = %act_name,
                input_count = inputs.len(),
                "Configured inputs for act"
            );

            // Update the act with inputs
            if let Some(act) = updated_acts.get_mut(act_name) {
                act.inputs = inputs.clone();
            }

            dialog
                .show_info(&format!(
                    "✓ Configured {} input(s) for act '{}'",
                    inputs.len(),
                    act_name
                ))
                .await?;
        }

        // Rebuild PartialNarrative with updated acts
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

        Ok(())
    }

    fn is_complete(&self, _partial: &PartialNarrative) -> bool {
        // Inputs are optional, so always complete
        true
    }

    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        if partial.acts().is_empty() {
            Some("Add acts before configuring inputs".to_string())
        } else {
            Some("Configure carousel or finalize narrative".to_string())
        }
    }
}
