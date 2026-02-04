//! Protocol abstraction for primitive elicitation operations.
//!
//! This trait defines the interface for primitive elicitation operations,
//! enabling different implementations for human (TUI) and agent (LLM sampling) protocols.

use async_trait::async_trait;

/// Protocol for primitive elicitation operations.
///
/// This trait abstracts over human (TUI) and agent (LLM sampling) protocols.
/// Both implementations must provide the same four primitive operations,
/// enabling contract-based tools to work seamlessly with either protocol.
///
/// # Type Parameters
///
/// - `Error`: The error type for this protocol (typically `BotticelliError`)
///
/// # Implementations
///
/// Implementations typically include:
/// - **Human protocol**: Interactive TUI elicitation via DialogResource
/// - **Agent protocol**: LLM sampling-based elicitation via BotticelliDriver
///
/// # Example
///
/// ```no_run
/// use botticelli_interface::ElicitationProtocol;
///
/// async fn example<E: std::error::Error>(protocol: &dyn ElicitationProtocol<Error = E>) {
///     let name = protocol.elicit_text("What is your name?").await.unwrap();
///     println!("Hello, {}!", name);
/// }
/// ```
#[async_trait]
pub trait ElicitationProtocol: Send + Sync {
    /// Error type for this protocol.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Elicit free-form text from user/agent.
    ///
    /// # Arguments
    ///
    /// - `prompt`: The question or prompt to present
    ///
    /// # Returns
    ///
    /// The elicited text value.
    async fn elicit_text(&self, prompt: &str) -> Result<String, Self::Error>;

    /// Elicit boolean confirmation from user/agent.
    ///
    /// # Arguments
    ///
    /// - `prompt`: The question or prompt to present
    /// - `default`: Default value if parsing fails or is ambiguous
    ///
    /// # Returns
    ///
    /// The elicited boolean value.
    async fn elicit_bool(&self, prompt: &str, default: bool) -> Result<bool, Self::Error>;

    /// Elicit integer within range from user/agent.
    ///
    /// # Arguments
    ///
    /// - `prompt`: The question or prompt to present
    /// - `min`: Minimum allowed value (inclusive)
    /// - `max`: Maximum allowed value (inclusive)
    ///
    /// # Returns
    ///
    /// The elicited integer, clamped to [min, max] range.
    async fn elicit_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Self::Error>;

    /// Elicit selection from options.
    ///
    /// # Arguments
    ///
    /// - `prompt`: The question or prompt to present
    /// - `options`: Available options to choose from
    ///
    /// # Returns
    ///
    /// The selected option text.
    async fn elicit_select(&self, prompt: &str, options: &[&str]) -> Result<String, Self::Error>;
}
