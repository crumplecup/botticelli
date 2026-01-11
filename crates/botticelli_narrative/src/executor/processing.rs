//! Processing helpers for act execution.

use super::core::NarrativeExecutor;
use crate::{ActConfig, CarouselConfig, NarrativeMetadata, ProcessorContext};
use botticelli_core::{ActExecution, Message, MessageBuilder};
use botticelli_error::{BotticelliResult, NarrativeError, NarrativeErrorKind};
use botticelli_interface::{BotticelliDriver, NarrativeProvider};
use tracing::instrument;

impl<D, BE> NarrativeExecutor<D, BE>
where
    D: BotticelliDriver<
            Request = botticelli_core::GenerateRequest,
            Response = botticelli_core::GenerateResponse,
        >,
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Process act results through registered processors.
    pub(super) async fn process_with_processors<N>(
        &self,
        narrative: &N,
        config: &ActConfig,
        act_execution: &ActExecution,
        sequence_number: usize,
    ) -> BotticelliResult<()>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        let Some(registry) = &self.processor_registry else {
            return Ok(());
        };

        tracing::info!(
            processors = registry.len(),
            "Processing act with registered processors"
        );

        // Determine if this is the last act in the narrative
        let is_last_act = sequence_number == narrative.act_names().len() - 1;

        // Determine if we should extract output:
        // - If extract_output is explicitly set, use that value
        // - Otherwise, only extract for the last act (default behavior)
        let should_extract_output = config.extract_output().unwrap_or(is_last_act);

        tracing::debug!(
            is_last_act,
            extract_config = ?config.extract_output(),
            should_extract_output,
            "Determined extraction policy"
        );

        // Build processor context
        let context = ProcessorContext::new(
            act_execution,
            narrative.metadata(),
            narrative.name(),
            is_last_act,
            should_extract_output,
        );

        if let Err(e) = registry.process(&context).await {
            tracing::error!(
                error = %e,
                "Act processing failed, continuing execution"
            );
            // Note: We don't fail the entire narrative on processor errors
            // The user still gets the execution results
        }

        Ok(())
    }

    /// Apply history retention policies to conversation history.
    #[instrument(skip(conversation_history), fields(act = act_name, history_len = conversation_history.len()))]
    pub(super) fn apply_history_retention(
        act_name: &str,
        conversation_history: &mut [Message],
    ) -> BotticelliResult<()> {
        // Apply history retention policies to the user message we just processed
        // The user message is at conversation_history.len() - 2 (assistant message was just pushed)
        if conversation_history.len() >= 2 {
            let user_msg_idx = conversation_history.len() - 2;
            if let Some(user_message) = conversation_history.get(user_msg_idx) {
                // Apply retention policies to the message content
                let updated_content =
                    crate::HistoryRetention::apply_retention(user_message.content());

                // Only replace if content changed
                if updated_content.len() != user_message.content().len()
                    || updated_content
                        .iter()
                        .zip(user_message.content().iter())
                        .any(|(a, b)| a != b)
                {
                    // Create new message with updated content
                    let updated_message = MessageBuilder::default()
                        .role(*user_message.role())
                        .content(updated_content)
                        .build()
                        .map_err(|e| {
                            NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                                "Failed to build message with retention policy: {}",
                                e
                            )))
                        })?;

                    // Replace the old message
                    conversation_history[user_msg_idx] = updated_message;

                    tracing::debug!(
                        act = %act_name,
                        "Applied history retention policies to user message"
                    );
                }
            }
        }
        Ok(())
    }

    /// Calculate total metrics from act executions.
    #[instrument(skip(act_executions), fields(act_count = act_executions.len()))]
    pub(super) fn calculate_total_metrics(
        act_executions: &[ActExecution],
    ) -> (Option<botticelli_core::TokenUsageData>, Option<u64>) {
        let total_token_usage = act_executions
            .iter()
            .filter_map(|act| act.token_usage().as_ref())
            .fold(
                None,
                |acc: Option<botticelli_core::TokenUsageData>, usage| {
                    Some(match acc {
                        None => *usage,
                        Some(acc) => {
                            // Builder errors should be rare; if it fails, use the existing accumulator
                            botticelli_core::TokenUsageData::builder()
                                .input_tokens(acc.input_tokens() + usage.input_tokens())
                                .output_tokens(acc.output_tokens() + usage.output_tokens())
                                .total_tokens(acc.total_tokens() + usage.total_tokens())
                                .build()
                                .unwrap_or(acc)
                        }
                    })
                },
            );

        let total_duration_ms = act_executions
            .iter()
            .filter_map(|act| *act.duration_ms())
            .sum::<u64>();

        let total_duration_ms = if total_duration_ms > 0 {
            Some(total_duration_ms)
        } else {
            None
        };

        (total_token_usage, total_duration_ms)
    }
}
