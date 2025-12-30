//! Dialog resource for sharing ElicitationDialog across MCP tools.

use crate::elicitation::dialog::ElicitationDialog;
use botticelli_error::BotticelliResult;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared dialog resource for MCP tools.
///
/// This wrapper allows multiple MCP tools to share access to a single
/// ElicitationDialog instance, enabling the primitive elicitation tools
/// (elicit_text, elicit_select, etc.) to interact with the UI layer.
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp::{DialogResource, ElicitationDialog};
/// use std::sync::Arc;
///
/// async fn example(dialog: Box<dyn ElicitationDialog>) {
///     let resource = Arc::new(DialogResource::new(dialog));
///
///     // Use in multiple tools
///     let text = resource.ask_text("Enter name:").await.unwrap();
/// }
/// ```
#[derive(Clone)]
pub struct DialogResource {
    dialog: Arc<Mutex<Box<dyn ElicitationDialog>>>,
}

impl DialogResource {
    /// Create a new dialog resource wrapping an ElicitationDialog.
    pub fn new(dialog: Box<dyn ElicitationDialog>) -> Self {
        Self {
            dialog: Arc::new(Mutex::new(dialog)),
        }
    }

    /// Ask for free-form text input.
    pub async fn ask_text(&self, prompt: &str) -> BotticelliResult<String> {
        self.dialog.lock().await.ask_text(prompt).await
    }

    /// Ask user to choose from a list of options.
    ///
    /// Returns the index of the selected option.
    pub async fn ask_choice(&self, prompt: &str, options: &[&str]) -> BotticelliResult<usize> {
        self.dialog.lock().await.ask_choice(prompt, options).await
    }

    /// Ask for a number within a range.
    pub async fn ask_number(&self, prompt: &str, min: i64, max: i64) -> BotticelliResult<i64> {
        self.dialog.lock().await.ask_number(prompt, min, max).await
    }

    /// Ask for confirmation (yes/no).
    pub async fn ask_confirmation(&self, prompt: &str, default: bool) -> BotticelliResult<bool> {
        self.dialog
            .lock()
            .await
            .ask_confirmation(prompt, default)
            .await
    }
}
