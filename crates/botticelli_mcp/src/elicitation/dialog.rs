//! Dialog trait for UI-agnostic elicitation.

use async_trait::async_trait;
use botticelli_error::BotticelliResult;

/// UI interaction primitives for elicitation.
///
/// This trait abstracts the user interface layer, allowing elicitation
/// to work across different platforms (TUI, Web, Android).
#[async_trait]
pub trait ElicitationDialog: Send + Sync {
    /// Ask for free-form text input.
    async fn ask_text(&mut self, prompt: &str) -> BotticelliResult<String>;

    /// Ask for confirmation (yes/no).
    ///
    /// # Arguments
    ///
    /// * `prompt` - The question to ask
    /// * `default` - Default value if user just presses enter
    async fn ask_confirmation(&mut self, prompt: &str, default: bool) -> BotticelliResult<bool>;

    /// Ask user to choose from a list of options.
    ///
    /// Returns the index of the selected option.
    async fn ask_choice(&mut self, prompt: &str, options: &[&str]) -> BotticelliResult<usize>;

    /// Ask for a number within a range.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The question to ask
    /// * `min` - Minimum acceptable value (inclusive)
    /// * `max` - Maximum acceptable value (inclusive)
    async fn ask_number(&mut self, prompt: &str, min: i64, max: i64) -> BotticelliResult<i64>;

    /// Ask for a file path.
    ///
    /// Platform-specific implementation (file picker on GUI, text input on TUI).
    async fn ask_file_path(&mut self, prompt: &str) -> BotticelliResult<String>;

    /// Display an informational message.
    async fn show_info(&mut self, message: &str) -> BotticelliResult<()>;

    /// Display a warning message.
    async fn show_warning(&mut self, message: &str) -> BotticelliResult<()>;

    /// Display an error message.
    async fn show_error(&mut self, message: &str) -> BotticelliResult<()>;

    /// Display validation results.
    ///
    /// Shows errors and warnings from narrative validation.
    async fn show_validation(&mut self, validation_text: &str) -> BotticelliResult<()>;

    /// Show progress indicator for current step.
    ///
    /// # Arguments
    ///
    /// * `current` - Current step number
    /// * `total` - Total number of steps
    /// * `description` - Description of current step
    async fn show_progress(
        &mut self,
        current: usize,
        total: usize,
        description: &str,
    ) -> BotticelliResult<()>;

    /// Display a preview of generated TOML.
    async fn show_preview(&mut self, toml: &str) -> BotticelliResult<()>;
}
