//! Helper functions for driver selection and execution.

use crate::BotticelliServer;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use std::sync::Arc;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use tracing::instrument;

/// Macro to reduce duplication in driver selection.
///
/// Takes driver name and converts to trait object, handling None case.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
macro_rules! select_driver_impl {
    ($self:expr, $driver_getter:ident, $driver_name:expr) => {{
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        use tracing::debug;

        debug!(driver = $driver_name, "Selected driver");
        $self
            .$driver_getter()
            .clone()
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("{} driver not configured", $driver_name)),
                    None,
                )
            })
            .map(|d| {
                d as Arc<
                    dyn botticelli_interface::ExecutionDriver<
                            botticelli_core::GenerateRequest,
                            botticelli_core::GenerateResponse,
                        >,
                >
            })
    }};
}

impl BotticelliServer {
    /// Select driver based on model prefix.
    ///
    /// Returns a trait object that can be used by any of our execution functions.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier (e.g., "gemini-pro", "claude-3")
    ///
    /// # Returns
    ///
    /// Returns `Ok(driver)` if a matching driver is found and configured,
    /// or `Err` with appropriate error if no driver matches or driver not configured.
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    #[instrument(skip(self), fields(model = %model))]
    pub(super) fn select_driver(
        &self,
        model: &str,
    ) -> Result<
        Arc<
            dyn botticelli_interface::ExecutionDriver<
                    botticelli_core::GenerateRequest,
                    botticelli_core::GenerateResponse,
                >,
        >,
        rmcp::ErrorData,
    > {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        use tracing::debug;

        debug!(model = %model, "Selecting driver");

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            // Gemini
            #[cfg(feature = "gemini")]
            if model.starts_with("gemini") || model.starts_with("models/gemini") {
                return select_driver_impl!(self, gemini_driver, "Gemini");
            }

            // Anthropic (Claude)
            #[cfg(feature = "anthropic")]
            if model.starts_with("claude") {
                return select_driver_impl!(self, anthropic_driver, "Anthropic");
            }

            // Ollama (Llama, Mistral, etc.)
            #[cfg(feature = "ollama")]
            if model.starts_with("llama")
                || model.starts_with("mistral")
                || model.starts_with("codellama")
                || model.starts_with("ollama")
            {
                return select_driver_impl!(self, ollama_driver, "Ollama");
            }

            // HuggingFace
            #[cfg(feature = "huggingface")]
            if model.starts_with("hf") || model.starts_with("huggingface") {
                return select_driver_impl!(self, huggingface_driver, "HuggingFace");
            }

            // Groq
            #[cfg(feature = "groq")]
            if model.starts_with("groq") {
                return select_driver_impl!(self, groq_driver, "Groq");
            }

            // No matching driver
            Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "No driver configured for model: {}. Available drivers require feature flags.",
                    model
                )),
                None,
            ))
        }

        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed(
                    "No LLM drivers enabled. Enable at least one feature: gemini, anthropic, ollama, huggingface, or groq",
                ),
                None,
            ))
        }
    }
}
