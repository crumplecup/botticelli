//! LLM interaction methods for narrative execution.

use super::core::NarrativeExecutor;
use super::utils;
use crate::{ActConfig, CarouselConfig, NarrativeMetadata};
use botticelli_core::{GenerateRequest, Input, Message, MessageBuilder, Role};
use botticelli_error::{
    BackendError, BotticelliError, BotticelliResult, NarrativeError, NarrativeErrorKind,
};
use botticelli_interface::{BotticelliDriver, NarrativeProvider};
use std::time::Instant;
use tracing::instrument;

/// Result of executing an LLM act: (output, finish_reason, temperature, max_tokens, token_usage, duration)
pub(super) type LlmActResult = (
    String,
    Option<String>,
    Option<f32>,
    Option<u32>,
    Option<botticelli_core::TokenUsageData>,
    Option<std::time::Duration>,
);

impl<D, BE> NarrativeExecutor<D, BE>
where
    D: BotticelliDriver<Request = GenerateRequest, Response = botticelli_core::GenerateResponse>,
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Execute an LLM call for an act with text prompts.
    #[instrument(skip(self, narrative, config, processed_inputs, conversation_history))]
    pub(super) async fn execute_llm_act<N>(
        &self,
        act_name: &str,
        narrative: &N,
        config: &ActConfig,
        processed_inputs: Vec<Input>,
        conversation_history: &mut Vec<Message>,
    ) -> BotticelliResult<LlmActResult>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        tracing::debug!(
            processed_inputs_count = processed_inputs.len(),
            "Building LLM request with processed inputs"
        );

        for (idx, input) in processed_inputs.iter().enumerate() {
            match input {
                Input::Text(text) => {
                    let preview = text
                        .char_indices()
                        .take(100)
                        .last()
                        .map(|(idx, _)| &text[..=idx])
                        .unwrap_or(text);
                    tracing::debug!(
                        input_index = idx,
                        text_length = text.len(),
                        text_preview = preview,
                        "Processed input is Text"
                    );
                }
                other => {
                    tracing::debug!(
                        input_index = idx,
                        input_type = ?other,
                        "Processed input is non-Text"
                    );
                }
            }
        }

        conversation_history.push(
            MessageBuilder::default()
                .role(Role::User)
                .content(processed_inputs.clone())
                .build()
                .map_err(|e| {
                    NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                        "Failed to build message: {}",
                        e
                    )))
                })?,
        );

        // Apply narrative-level defaults for model/temperature/max_tokens if act doesn't override
        let metadata = narrative.metadata();
        let model = config.model().clone().or_else(|| metadata.model().clone());
        let temperature = config.temperature().or_else(|| *metadata.temperature());
        let max_tokens = config.max_tokens().or_else(|| *metadata.max_tokens());

        let mut request_builder =
            GenerateRequest::builder().messages(conversation_history.to_vec());

        if let Some(mt) = max_tokens {
            request_builder = request_builder.max_tokens(mt);
        }
        if let Some(temp) = temperature {
            request_builder = request_builder.temperature(temp);
        }
        if let Some(m) = &model {
            request_builder = request_builder.model(m.clone());
        }

        let request = request_builder.build().map_err(|e| {
            BotticelliError::from(BackendError::new(format!("Failed to build request: {}", e)))
        })?;

        // Call the LLM
        let llm_span = tracing::info_span!(
            "llm_call",
            act = %act_name,
            model = ?request.model(),
            temperature = ?request.temperature(),
            max_tokens = ?request.max_tokens(),
            message_count = request.messages().len(),
            tokens = tracing::field::Empty,
        );

        let act_start = Instant::now();
        let response = {
            let _enter = llm_span.enter();
            tracing::info!("Calling LLM API");
            let result = self
                .driver
                .generate(&request)
                .await
                .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))?;
            let act_duration = act_start.elapsed();

            tracing::info!(
                outputs_count = result.outputs().len(),
                duration_ms = act_duration.as_millis(),
                "LLM response received"
            );
            result
        };
        let act_duration = act_start.elapsed();

        // Debug log the output types
        for (idx, output) in response.outputs().iter().enumerate() {
            match output {
                botticelli_core::Output::Text(text) => {
                    let preview: String = text.chars().take(100).collect();
                    let preview = preview.as_str();
                    tracing::debug!(
                        output_index = idx,
                        text_length = text.len(),
                        text_preview = preview,
                        "Output is Text variant"
                    );
                }
                other => {
                    tracing::debug!(
                        output_index = idx,
                        output_type = ?other,
                        "Output is non-Text variant"
                    );
                }
            }
        }

        // Extract text from response
        let response_text = utils::extract_text_from_outputs(response.outputs())?;

        let preview = response_text.chars().take(200).collect::<String>();
        tracing::debug!(
            response_length = response_text.len(),
            response_preview = preview,
            "Response text extracted from LLM outputs"
        );

        Ok((
            response_text,
            model,
            temperature,
            max_tokens,
            *response.usage(),
            Some(act_duration),
        ))
    }
}
