//! Protocol implementations for primitive elicitation operations.
//!
//! This module provides concrete implementations of the [`ElicitationProtocol`] trait
//! for both human (TUI) and agent (LLM sampling) use cases.

use crate::dialog_resource::DialogResource;
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Role};
use botticelli_error::BotticelliError;
use botticelli_interface::{BotticelliDriver, ElicitationProtocol};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Provider enum wrapping protocol implementations.
///
/// Allows storing either HumanProtocol or AgentProtocol in server state
/// while maintaining a single type.
#[derive(Clone)]
pub enum ElicitationProvider {
    /// Human interactive protocol (TUI/CLI).
    Human(HumanProtocol),
    /// Agent protocol (stores the AgentProtocol directly, not the driver).
    ///
    /// Note: AgentProtocol is not Clone-able when it wraps a driver trait object,
    /// so we Box it for interior mutability.
    Agent(Arc<dyn ElicitationProtocol<Error = BotticelliError>>),
}

#[async_trait]
impl ElicitationProtocol for ElicitationProvider {
    type Error = BotticelliError;

    async fn elicit_text(&self, prompt: &str) -> Result<String, Self::Error> {
        match self {
            ElicitationProvider::Human(h) => h.elicit_text(prompt).await,
            ElicitationProvider::Agent(protocol) => protocol.elicit_text(prompt).await,
        }
    }

    async fn elicit_bool(&self, prompt: &str, default: bool) -> Result<bool, Self::Error> {
        match self {
            ElicitationProvider::Human(h) => h.elicit_bool(prompt, default).await,
            ElicitationProvider::Agent(protocol) => protocol.elicit_bool(prompt, default).await,
        }
    }

    async fn elicit_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Self::Error> {
        match self {
            ElicitationProvider::Human(h) => h.elicit_number(prompt, min, max).await,
            ElicitationProvider::Agent(protocol) => protocol.elicit_number(prompt, min, max).await,
        }
    }

    async fn elicit_select(&self, prompt: &str, options: &[&str]) -> Result<String, Self::Error> {
        match self {
            ElicitationProvider::Human(h) => h.elicit_select(prompt, options).await,
            ElicitationProvider::Agent(protocol) => protocol.elicit_select(prompt, options).await,
        }
    }
}

/// Human protocol implementation (interactive TUI).
///
/// Wraps a [`DialogResource`] and delegates all elicitation operations to it.
/// This enables interactive elicitation through a TUI or CLI interface.
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp::elicitation_protocol::HumanProtocol;
/// use botticelli_mcp::DialogResource;
/// use botticelli_interface::ElicitationProtocol;
/// use std::sync::Arc;
///
/// async fn example(dialog: Arc<DialogResource>) {
///     let protocol = HumanProtocol::new(dialog);
///     let name = protocol.elicit_text("Enter your name:").await.unwrap();
/// }
/// ```
#[derive(Clone)]
pub struct HumanProtocol {
    dialog: Arc<DialogResource>,
}

impl HumanProtocol {
    /// Create a new human protocol wrapping a DialogResource.
    #[instrument(skip(dialog))]
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        debug!("Created HumanProtocol");
        Self { dialog }
    }
}

#[async_trait]
impl ElicitationProtocol for HumanProtocol {
    type Error = BotticelliError;

    #[instrument(skip(self), fields(prompt))]
    async fn elicit_text(&self, prompt: &str) -> Result<String, Self::Error> {
        debug!("Eliciting text via human protocol");
        self.dialog.ask_text(prompt).await
    }

    #[instrument(skip(self), fields(prompt, default))]
    async fn elicit_bool(&self, prompt: &str, default: bool) -> Result<bool, Self::Error> {
        debug!("Eliciting bool via human protocol");
        self.dialog.ask_confirmation(prompt, default).await
    }

    #[instrument(skip(self), fields(prompt, min, max))]
    async fn elicit_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Self::Error> {
        debug!("Eliciting number via human protocol");
        self.dialog.ask_number(prompt, min, max).await
    }

    #[instrument(skip(self), fields(prompt, option_count = options.len()))]
    async fn elicit_select(&self, prompt: &str, options: &[&str]) -> Result<String, Self::Error> {
        debug!("Eliciting selection via human protocol");
        let index = self.dialog.ask_choice(prompt, options).await?;
        Ok(options[index].to_string())
    }
}

/// Agent protocol implementation (LLM sampling).
///
/// Uses an LLM driver to elicit values through sampling. Each primitive operation
/// constructs a specialized prompt that instructs the LLM to return only the
/// requested value without explanation.
///
/// # Generic Parameter
///
/// - `D`: The BotticelliDriver implementation
///
/// # Prompt Engineering
///
/// Each primitive type uses specific prompt engineering:
/// - **Text**: Direct request with no formatting
/// - **Bool**: Explicit "true/false" instruction with fallback to default
/// - **Number**: Integer-only request with range clamping
/// - **Select**: Lists options explicitly with exact/fuzzy matching
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp::elicitation_protocol::AgentProtocol;
/// use botticelli_interface::ElicitationProtocol;
/// use std::sync::Arc;
///
/// async fn example<D>(driver: Arc<D>)
/// where
///     D: botticelli_interface::BotticelliDriver<
///         Request = botticelli_core::GenerateRequest,
///         Response = botticelli_core::GenerateResponse,
///         Error = botticelli_error::BotticelliError,
///     >,
/// {
///     let protocol = AgentProtocol::new(driver);
///     let name = protocol.elicit_text("Enter your name:").await.unwrap();
/// }
/// ```
pub struct AgentProtocol<D> {
    driver: Arc<D>,
}

impl<D> AgentProtocol<D>
where
    D: BotticelliDriver<Request = GenerateRequest, Response = GenerateResponse, Error = BotticelliError>,
{
    /// Create a new agent protocol using an LLM driver.
    #[instrument(skip(driver))]
    pub fn new(driver: Arc<D>) -> Self {
        debug!("Created AgentProtocol");
        Self { driver }
    }

    /// Construct sampling request with prompt engineering for specific primitive.
    #[instrument(skip(self), fields(primitive_type))]
    fn build_request(&self, prompt: &str, primitive_type: &str) -> GenerateRequest {
        use botticelli_core::MessageBuilder;
        
        debug!("Building sampling request for {}", primitive_type);
        let message = MessageBuilder::default()
            .role(Role::User)
            .content(vec![Input::Text(format!(
                "You are helping elicit information.\n\n\
                Task: {}\n\
                Type: {}\n\n\
                Provide ONLY the requested value, no explanation or formatting.\n\
                For text: provide the literal text\n\
                For bool: provide 'true' or 'false'\n\
                For number: provide the integer\n\
                For select: provide the exact option text",
                prompt, primitive_type
            ))])
            .build()
            .expect("Failed to build message");

        GenerateRequest::builder()
            .messages(vec![message])
            .max_tokens(100u32)
            .temperature(0.7)
            .build()
            .expect("Failed to build request")
    }

    /// Extract text from response outputs.
    #[instrument(skip(self, response))]
    fn extract_text(&self, response: GenerateResponse) -> Result<String, BotticelliError> {
        use botticelli_core::Output;
        use botticelli_error::{SamplingError, SamplingErrorKind};

        // Find first text output
        for output in response.outputs() {
            if let Output::Text(text) = output {
                return Ok(text.clone());
            }
        }

        // No text output found
        Err(SamplingError::new(
            SamplingErrorKind::ProviderError("No text output in response".to_string()),
        ).into())
    }
}

#[async_trait]
impl<D> ElicitationProtocol for AgentProtocol<D>
where
    D: BotticelliDriver<Request = GenerateRequest, Response = GenerateResponse, Error = BotticelliError>,
{
    type Error = BotticelliError;

    #[instrument(skip(self), fields(prompt))]
    async fn elicit_text(&self, prompt: &str) -> Result<String, Self::Error> {
        debug!("Eliciting text via agent protocol");
        let request = self.build_request(prompt, "text");
        let response = self.driver.generate(&request).await?;
        let text = self.extract_text(response)?;
        Ok(text.trim().to_string())
    }

    #[instrument(skip(self), fields(prompt, default))]
    async fn elicit_bool(&self, prompt: &str, default: bool) -> Result<bool, Self::Error> {
        debug!("Eliciting bool via agent protocol");
        let request = self.build_request(prompt, "bool");
        let response = self.driver.generate(&request).await?;
        let text = self.extract_text(response)?;

        let text = text.trim().to_lowercase();
        let result = match text.as_str() {
            "true" | "yes" | "y" => true,
            "false" | "no" | "n" => false,
            _ => {
                debug!("Ambiguous response '{}', using default {}", text, default);
                default
            }
        };

        Ok(result)
    }

    #[instrument(skip(self), fields(prompt, min, max))]
    async fn elicit_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Self::Error> {
        debug!("Eliciting number via agent protocol");
        let request = self.build_request(prompt, "integer");
        let response = self.driver.generate(&request).await?;
        let text = self.extract_text(response)?;

        let text = text.trim();
        let num = text.parse::<i64>().map_err(|e| {
            use botticelli_error::{SamplingError, SamplingErrorKind};
            SamplingError::new(
                SamplingErrorKind::ProviderError(format!("Failed to parse number '{}': {}", text, e))
            )
        })?;

        // Clamp to range
        let clamped = num.max(min).min(max);
        if clamped != num {
            debug!("Clamped {} to range [{}, {}] = {}", num, min, max, clamped);
        }

        Ok(clamped)
    }

    #[instrument(skip(self), fields(prompt, option_count = options.len()))]
    async fn elicit_select(&self, prompt: &str, options: &[&str]) -> Result<String, Self::Error> {
        debug!("Eliciting selection via agent protocol");
        let options_str = options.join(", ");
        let enhanced_prompt = format!("{}\nOptions: {}", prompt, options_str);
        let request = self.build_request(&enhanced_prompt, "selection");

        let response = self.driver.generate(&request).await?;
        let text = self.extract_text(response)?;
        let text = text.trim();

        // Try exact match first
        if let Some(option) = options.iter().find(|&&opt| opt == text) {
            debug!("Exact match: '{}'", option);
            return Ok(option.to_string());
        }

        // Try case-insensitive match
        if let Some(option) = options.iter().find(|&&opt| opt.eq_ignore_ascii_case(text)) {
            debug!("Case-insensitive match: '{}'", option);
            return Ok(option.to_string());
        }

        // Fallback: return first option
        debug!(
            "No match for '{}', falling back to first option: '{}'",
            text, options[0]
        );
        Ok(options[0].to_string())
    }
}
