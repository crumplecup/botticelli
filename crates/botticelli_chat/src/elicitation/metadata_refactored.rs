//! Refactored metadata elicitation using elicitation crate paradigms.
//!
//! This is a proof-of-concept demonstrating code reduction through the use
//! of the elicitation crate's #[derive(Elicit)] macros and shared infrastructure.
//!
//! **Original**: 167 lines with manual dialog.ask_*() calls
//! **Refactored**: ~50 lines with single .elicit() call (using shared infrastructure)
//! **Reduction**: ~70% fewer lines

use botticelli_error::{BotticelliResult, ChatError, ChatErrorKind};
use botticelli_mcp::{
    ElicitationDialog, NarrativeHelper, PartialNarrative, PartialNarrativeBuilder,
};
use elicitation::Elicitation;
use tracing::{debug, instrument};

use super::infrastructure::create_mcp_client_for_dialog;
use super::types::NarrativeMetadata;

/// Elicit narrative metadata using paradigm-based approach.
///
/// This function demonstrates the new pattern:
/// 1. Create InProcTransport MCP server with dialog
/// 2. Call NarrativeMetadata::elicit() (derive macro)
/// 3. Validate and update PartialNarrative
///
/// Compare to original MetadataElicitor::elicit() which has:
/// - Manual validation loops
/// - Explicit dialog.ask_text() calls
/// - Explicit dialog.ask_choice() calls
/// - Explicit dialog.ask_confirmation() calls
/// - Explicit dialog.ask_number() calls
/// - Manual option handling
///
/// This version replaces all that with:
/// - Single NarrativeMetadata::elicit() call
/// - Automatic field elicitation via Survey paradigm
/// - Type-safe result
#[instrument(skip(dialog, partial))]
pub async fn elicit_metadata_refactored(
    dialog: Box<dyn ElicitationDialog>,
    partial: &mut PartialNarrative,
) -> BotticelliResult<()> {
    // Show introductory message
    // Note: We'd need to clone dialog or use Arc to call show_info here
    // For this proof of concept, we'll skip it

    // 1. Create MCP client with primitive elicitation tools
    let client = create_mcp_client_for_dialog(dialog).await?;

    // 2. Elicit metadata using derive macro - this ONE LINE replaces ~100 lines!
    let metadata = NarrativeMetadata::elicit(&client).await.map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to elicit metadata: {}",
            e
        )))
    })?;

    // 3. Post-elicitation validation (custom logic)
    if !NarrativeHelper::is_valid_name(&metadata.name) {
        return Err(ChatError::new(ChatErrorKind::InvalidState(
            format!(
                "Invalid narrative name '{}'. Must start with letter, contain only alphanumeric and underscores, max 64 chars.",
                metadata.name
            ),
        )).into());
    }

    debug!(name = %metadata.name, "Narrative name validated");

    // 4. Convert types as needed
    let max_tokens = metadata.default_max_tokens.map(|t| t as u32);

    // 5. Update partial narrative
    let updated = PartialNarrativeBuilder::default()
        .name(metadata.name)
        .description(metadata.description)
        .model(metadata.default_model)
        .temperature(metadata.default_temperature)
        .max_tokens(max_tokens)
        .act_order(partial.act_order().clone())
        .acts(partial.acts().clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    /// Mock dialog for testing.
    struct MockDialog {
        text_responses: Vec<String>,
        text_index: std::sync::atomic::AtomicUsize,
        number_response: i64,
        bool_response: bool,
    }

    impl MockDialog {
        fn new() -> Self {
            Self {
                text_responses: vec!["test_narrative".to_string(), "Test description".to_string()],
                text_index: std::sync::atomic::AtomicUsize::new(0),
                number_response: 15,  // Will become 1.5 for temperature
                bool_response: false, // No optional fields
            }
        }
    }

    #[async_trait]
    impl ElicitationDialog for MockDialog {
        async fn ask_text(&mut self, _prompt: &str) -> BotticelliResult<String> {
            let idx = self
                .text_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.text_responses.get(idx).cloned().unwrap_or_default())
        }

        async fn ask_confirmation(
            &mut self,
            _prompt: &str,
            _default: bool,
        ) -> BotticelliResult<bool> {
            Ok(self.bool_response)
        }

        async fn ask_choice(
            &mut self,
            _prompt: &str,
            _options: &[&str],
        ) -> BotticelliResult<usize> {
            Ok(0)
        }

        async fn ask_number(
            &mut self,
            _prompt: &str,
            _min: i64,
            _max: i64,
        ) -> BotticelliResult<i64> {
            Ok(self.number_response)
        }

        async fn ask_file_path(&mut self, _prompt: &str) -> BotticelliResult<String> {
            Ok("/test/path".to_string())
        }

        async fn show_info(&mut self, _message: &str) -> BotticelliResult<()> {
            Ok(())
        }

        async fn show_warning(&mut self, _message: &str) -> BotticelliResult<()> {
            Ok(())
        }

        async fn show_error(&mut self, _message: &str) -> BotticelliResult<()> {
            Ok(())
        }

        async fn show_validation(&mut self, _validation_text: &str) -> BotticelliResult<()> {
            Ok(())
        }

        async fn show_progress(
            &mut self,
            _current: usize,
            _total: usize,
            _description: &str,
        ) -> BotticelliResult<()> {
            Ok(())
        }

        async fn show_preview(&mut self, _toml: &str) -> BotticelliResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_refactored_metadata_elicitation() {
        let _ = tracing_subscriber::fmt::try_init();

        let dialog = Box::new(MockDialog::new());
        let mut partial = PartialNarrativeBuilder::default()
            .build()
            .expect("Failed to build partial");

        let result = elicit_metadata_refactored(dialog, &mut partial).await;

        assert!(result.is_ok(), "Elicitation should succeed");
        assert_eq!(partial.name(), &Some("test_narrative".to_string()));
        assert_eq!(partial.description(), &Some("Test description".to_string()));
    }
}
