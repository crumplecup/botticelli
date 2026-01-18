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
                debug!("Selected Gemini driver");
                return self
                    .gemini_driver()
                    .clone()
                    .ok_or_else(|| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Borrowed("Gemini driver not configured"),
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
                    });
            }

            // Anthropic (Claude)
            #[cfg(feature = "anthropic")]
            if model.starts_with("claude") {
                debug!("Selected Anthropic driver");
                return self
                    .anthropic_driver()
                    .clone()
                    .ok_or_else(|| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Borrowed("Anthropic driver not configured"),
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
                    });
            }

            // Ollama (Llama, Mistral, etc.)
            #[cfg(feature = "ollama")]
            if model.starts_with("llama")
                || model.starts_with("mistral")
                || model.starts_with("codellama")
                || model.starts_with("ollama")
            {
                debug!("Selected Ollama driver");
                return self
                    .ollama_driver()
                    .clone()
                    .ok_or_else(|| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Borrowed("Ollama driver not configured"),
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
                    });
            }

            // HuggingFace
            #[cfg(feature = "huggingface")]
            if model.starts_with("hf") || model.starts_with("huggingface") {
                debug!("Selected HuggingFace driver");
                return self
                    .huggingface_driver()
                    .clone()
                    .ok_or_else(|| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Borrowed("HuggingFace driver not configured"),
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
                    });
            }

            // Groq
            #[cfg(feature = "groq")]
            if model.starts_with("groq") {
                debug!("Selected Groq driver");
                return self
                    .groq_driver()
                    .clone()
                    .ok_or_else(|| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Borrowed("Groq driver not configured"),
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
                    });
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
            use rmcp::model::ErrorCode;
            use std::borrow::Cow;
            
            Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No LLM drivers enabled. Enable at least one feature: gemini, anthropic, ollama, huggingface, or groq"),
                None,
            ))
        }
    }
}
