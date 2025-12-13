//! Tests for elicitation system.

use botticelli_chat::{
    ChatError, ChatResult, ElicitationDialog, MetadataElicitor, NarrativeElicitor,
    PartialNarrativeBuilder,
};
use async_trait::async_trait;
use std::collections::VecDeque;

/// Mock dialog for testing.
///
/// Pre-loads responses and verifies they are consumed in order.
struct MockDialog {
    text_responses: VecDeque<String>,
    confirmation_responses: VecDeque<bool>,
    choice_responses: VecDeque<usize>,
    number_responses: VecDeque<i64>,
    messages: Vec<String>,
}

impl MockDialog {
    fn new() -> Self {
        Self {
            text_responses: VecDeque::new(),
            confirmation_responses: VecDeque::new(),
            choice_responses: VecDeque::new(),
            number_responses: VecDeque::new(),
            messages: Vec::new(),
        }
    }

    fn with_text_response(mut self, response: impl Into<String>) -> Self {
        self.text_responses.push_back(response.into());
        self
    }

    fn with_confirmation_response(mut self, response: bool) -> Self {
        self.confirmation_responses.push_back(response);
        self
    }

    fn with_choice_response(mut self, response: usize) -> Self {
        self.choice_responses.push_back(response);
        self
    }

    fn with_number_response(mut self, response: i64) -> Self {
        self.number_responses.push_back(response);
        self
    }

    fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[async_trait]
impl ElicitationDialog for MockDialog {
    async fn ask_text(&mut self, _prompt: &str) -> ChatResult<String> {
        self.text_responses
            .pop_front()
            .ok_or_else(|| ChatError::invalid_state("No text response available"))
    }

    async fn ask_confirmation(&mut self, _prompt: &str, _default: bool) -> ChatResult<bool> {
        self.confirmation_responses
            .pop_front()
            .ok_or_else(|| ChatError::invalid_state("No confirmation response available"))
    }

    async fn ask_choice(&mut self, _prompt: &str, _options: &[&str]) -> ChatResult<usize> {
        self.choice_responses
            .pop_front()
            .ok_or_else(|| ChatError::invalid_state("No choice response available"))
    }

    async fn ask_number(&mut self, _prompt: &str, _min: i64, _max: i64) -> ChatResult<i64> {
        self.number_responses
            .pop_front()
            .ok_or_else(|| ChatError::invalid_state("No number response available"))
    }

    async fn ask_file_path(&mut self, _prompt: &str) -> ChatResult<String> {
        self.text_responses
            .pop_front()
            .ok_or_else(|| ChatError::invalid_state("No file path response available"))
    }

    async fn show_info(&mut self, message: &str) -> ChatResult<()> {
        self.messages.push(format!("INFO: {}", message));
        Ok(())
    }

    async fn show_warning(&mut self, message: &str) -> ChatResult<()> {
        self.messages.push(format!("WARNING: {}", message));
        Ok(())
    }

    async fn show_error(&mut self, message: &str) -> ChatResult<()> {
        self.messages.push(format!("ERROR: {}", message));
        Ok(())
    }

    async fn show_validation(&mut self, validation_text: &str) -> ChatResult<()> {
        self.messages
            .push(format!("VALIDATION: {}", validation_text));
        Ok(())
    }

    async fn show_progress(
        &mut self,
        current: usize,
        total: usize,
        description: &str,
    ) -> ChatResult<()> {
        self.messages.push(format!(
            "PROGRESS: {}/{} - {}",
            current, total, description
        ));
        Ok(())
    }

    async fn show_preview(&mut self, toml: &str) -> ChatResult<()> {
        self.messages.push(format!("PREVIEW: {}", toml));
        Ok(())
    }
}

#[tokio::test]
async fn test_metadata_elicitor_basic() {
    // Setup mock dialog with predefined responses
    let mut dialog = MockDialog::new()
        .with_text_response("test_narrative") // name
        .with_text_response("A test narrative for unit testing") // description
        .with_confirmation_response(true) // Configure model?
        .with_choice_response(0) // Model choice (first option)
        .with_confirmation_response(true) // Set temperature?
        .with_number_response(7) // Temperature 0.7 (scaled from input)
        .with_confirmation_response(true) // Set max_tokens?
        .with_number_response(1000); // Max tokens

    // Create elicitor and partial narrative
    let elicitor = MetadataElicitor::new();
    let mut partial = PartialNarrativeBuilder::default()
        .build()
        .expect("Empty partial should build");

    // Run elicitation
    let result = elicitor.elicit(&mut dialog, &mut partial).await;
    assert!(result.is_ok(), "Elicitation should succeed: {:?}", result);

    // Verify partial narrative was updated
    assert_eq!(
        partial.name().as_ref().map(|s| s.as_str()),
        Some("test_narrative")
    );
    assert_eq!(
        partial.description().as_ref().map(|s| s.as_str()),
        Some("A test narrative for unit testing")
    );
    assert!(partial.model().is_some(), "Model should be set");
    assert!(partial.temperature().is_some(), "Temperature should be set");
    assert!(
        partial.max_tokens().is_some(),
        "Max tokens should be set"
    );

    // Verify dialog showed appropriate messages
    let messages = dialog.messages();
    assert!(
        !messages.is_empty(),
        "Should have shown info messages"
    );
}

#[tokio::test]
async fn test_metadata_elicitor_minimal() {
    // Setup mock dialog for minimal configuration (no defaults)
    let mut dialog = MockDialog::new()
        .with_text_response("minimal_narrative") // name
        .with_text_response("Minimal test") // description
        .with_confirmation_response(false) // Configure model? No
        .with_confirmation_response(false) // Set temperature? No
        .with_confirmation_response(false); // Set max_tokens? No

    let elicitor = MetadataElicitor::new();
    let mut partial = PartialNarrativeBuilder::default()
        .build()
        .expect("Empty partial should build");

    let result = elicitor.elicit(&mut dialog, &mut partial).await;
    assert!(result.is_ok(), "Minimal elicitation should succeed: {:?}", result);

    // Verify only required fields are set
    assert!(partial.name().is_some());
    assert!(partial.description().is_some());
    assert!(
        partial.model().is_none(),
        "Model should not be set in minimal mode"
    );
}

#[tokio::test]
async fn test_partial_narrative_minimum_required() {
    // Test has_minimum_required() logic
    let empty = PartialNarrativeBuilder::default()
        .build()
        .expect("Should build");
    assert!(
        !empty.has_minimum_required(),
        "Empty partial should not have minimum"
    );

    let with_name = PartialNarrativeBuilder::default()
        .name(Some("test".to_string()))
        .build()
        .expect("Should build");
    assert!(
        !with_name.has_minimum_required(),
        "Need description and acts"
    );

    let with_name_desc = PartialNarrativeBuilder::default()
        .name(Some("test".to_string()))
        .description(Some("desc".to_string()))
        .build()
        .expect("Should build");
    assert!(
        !with_name_desc.has_minimum_required(),
        "Need at least one act"
    );

    // With act would require full setup, tested in integration tests
}

#[test]
fn test_elicitor_traits() {
    // Verify trait implementations compile
    let metadata_elicitor = MetadataElicitor::new();
    assert_eq!(metadata_elicitor.name(), "Metadata");
    assert!(metadata_elicitor.description().len() > 0);

    // Test is_complete before running
    let partial = PartialNarrativeBuilder::default()
        .build()
        .expect("Should build");
    assert!(
        !metadata_elicitor.is_complete(&partial),
        "Should not be complete without metadata"
    );
}
