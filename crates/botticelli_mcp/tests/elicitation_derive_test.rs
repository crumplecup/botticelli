//! Tests for elicitation crate derive macros with primitive tools.
//!
//! **OBSOLETE**: These tests use the old pmcp protocol, InProcTransport, and
//! register_all_tools() which have been replaced by rmcp and ToolRegistry.
//! All tests marked as ignored pending removal.
//!
//! Verifies Phase 3 integration: elicitation crate's #[derive(Elicit)] works
//! with our primitive tools via InProcTransport.

#![cfg_attr(test, allow(unused))]

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_mcp::{DialogResource, ElicitationDialog, InProcTransport, register_all_tools};
use elicitation::{Elicit, Elicitation, Prompt, Select};
use pmcp::{Client, ClientCapabilities, Server};
use std::sync::Arc;

/// Simple enum to test Select paradigm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
#[prompt("Choose an option:")]
enum TestChoice {
    OptionA,
    OptionB,
    OptionC,
}

/// Simple struct to test Survey paradigm with bool field.
#[derive(Debug, Clone, PartialEq, Eq, Elicit)]
#[prompt("Configure settings:")]
struct TestSettings {
    #[prompt("Enable feature?")]
    enabled: bool,
}

/// Mock dialog that returns preset values.
#[derive(Debug)]
struct MockDialog {
    choice_index: usize,
    bool_value: bool,
}

impl MockDialog {
    fn new() -> Self {
        Self {
            choice_index: 1,
            bool_value: true,
        }
    }

    fn with_choice(mut self, index: usize) -> Self {
        self.choice_index = index;
        self
    }

    fn with_bool(mut self, value: bool) -> Self {
        self.bool_value = value;
        self
    }
}

#[async_trait]
impl ElicitationDialog for MockDialog {
    async fn ask_text(&mut self, _prompt: &str) -> BotticelliResult<String> {
        Ok("test".to_string())
    }

    async fn ask_confirmation(&mut self, _prompt: &str, _default: bool) -> BotticelliResult<bool> {
        Ok(self.bool_value)
    }

    async fn ask_choice(&mut self, _prompt: &str, _options: &[&str]) -> BotticelliResult<usize> {
        Ok(self.choice_index)
    }

    async fn ask_number(&mut self, _prompt: &str, _min: i64, _max: i64) -> BotticelliResult<i64> {
        Ok(42)
    }

    async fn ask_file_path(&mut self, _prompt: &str) -> BotticelliResult<String> {
        Ok("/mock/path".to_string())
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

/// Helper to create server with primitive tools and client.
async fn setup_client_with_dialog(dialog: MockDialog) -> Client<InProcTransport> {
    // Wrap dialog in DialogResource
    let dialog_resource = Arc::new(DialogResource::new(Box::new(dialog)));

    // Build server with primitive elicitation tools
    let builder = Server::builder()
        .name("test-elicitation-server")
        .version("0.1.0")
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    let builder = register_all_tools(
        builder,
        Some(dialog_resource),
        #[cfg(feature = "database")]
        None,
    );

    let server = builder.build().expect("Failed to build server");

    // Create in-process transport
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    // Create and initialize client
    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize client");

    client
}

#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_select_paradigm_with_derive() {
    let _ = tracing_subscriber::fmt::try_init();

    // Mock dialog returns index 1 = OptionB
    let mock_dialog = MockDialog::new().with_choice(1);
    let client = setup_client_with_dialog(mock_dialog).await;

    // Use elicitation crate's derive macro to elicit the enum
    let result = TestChoice::elicit(&client)
        .await
        .expect("Failed to elicit TestChoice");

    assert_eq!(result, TestChoice::OptionB);
}

#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_select_first_option() {
    let _ = tracing_subscriber::fmt::try_init();

    // Mock dialog returns index 0 = OptionA
    let mock_dialog = MockDialog::new().with_choice(0);
    let client = setup_client_with_dialog(mock_dialog).await;

    let result = TestChoice::elicit(&client)
        .await
        .expect("Failed to elicit TestChoice");

    assert_eq!(result, TestChoice::OptionA);
}

#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_select_last_option() {
    let _ = tracing_subscriber::fmt::try_init();

    // Mock dialog returns index 2 = OptionC
    let mock_dialog = MockDialog::new().with_choice(2);
    let client = setup_client_with_dialog(mock_dialog).await;

    let result = TestChoice::elicit(&client)
        .await
        .expect("Failed to elicit TestChoice");

    assert_eq!(result, TestChoice::OptionC);
}

#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_survey_paradigm_with_bool() {
    let _ = tracing_subscriber::fmt::try_init();

    // Mock dialog returns true for bool field
    let mock_dialog = MockDialog::new().with_bool(true);
    let client = setup_client_with_dialog(mock_dialog).await;

    // Use elicitation crate's derive macro to elicit the struct
    let result = TestSettings::elicit(&client)
        .await
        .expect("Failed to elicit TestSettings");

    assert_eq!(result.enabled, true);
}

#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_survey_paradigm_with_false() {
    let _ = tracing_subscriber::fmt::try_init();

    // Mock dialog returns false for bool field
    let mock_dialog = MockDialog::new().with_bool(false);
    let client = setup_client_with_dialog(mock_dialog).await;

    let result = TestSettings::elicit(&client)
        .await
        .expect("Failed to elicit TestSettings");

    assert_eq!(result.enabled, false);
}
