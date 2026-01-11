//! Narrative composition methods for executing referenced narratives.

use super::core::NarrativeExecutor;
use crate::{ActConfig, MultiNarrative};
use botticelli_core::{ActExecution, ActExecutionBuilder, Input, Message, MessageBuilder, Role};
use botticelli_error::{
    BackendError, BotticelliError, BotticelliResult, NarrativeError, NarrativeErrorKind,
};
use botticelli_interface::BotticelliDriver;
use tracing::instrument;

impl<D, BE> NarrativeExecutor<D, BE>
where
    D: BotticelliDriver<
            Request = botticelli_core::GenerateRequest,
            Response = botticelli_core::GenerateResponse,
        >,
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Handle narrative composition by recursively executing referenced narratives.
    #[instrument(skip(self, config, multi), fields(act = act_name, narrative_ref = tracing::field::Empty, sequence = sequence_number))]
    pub(super) async fn execute_narrative_composition(
        &self,
        act_name: &str,
        config: &ActConfig,
        multi: Option<&MultiNarrative>,
        sequence_number: usize,
    ) -> BotticelliResult<(ActExecution, Message)> {
        let narrative_ref_name = config.narrative_ref().as_ref().ok_or_else(|| {
            NarrativeError::new(NarrativeErrorKind::ConfigurationError(
                "Missing narrative_ref field".to_string(),
            ))
        })?;

        tracing::info!(
            act = %act_name,
            referenced_narrative = %narrative_ref_name,
            "Executing narrative composition"
        );

        // Try to resolve the referenced narrative from MultiNarrative context
        let ref_narrative = if let Some(m) = multi {
            m.get_narrative(narrative_ref_name)
        } else {
            None
        }
        .ok_or_else(|| {
            NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                "Referenced narrative '{}' not found. Narrative composition requires MultiNarrative.",
                narrative_ref_name
            )))
        })?;

        // Recursively execute the referenced narrative with the same multi context
        tracing::debug!("Recursively executing referenced narrative from multi");
        let nested_execution = Box::pin(self.execute_impl_with_multi(ref_narrative, multi)).await?;

        // Collect all responses from the nested execution
        let nested_responses: Vec<String> = nested_execution
            .act_executions()
            .iter()
            .map(|e| e.response().clone())
            .collect();

        let combined_response = nested_responses.join("\n\n");

        tracing::info!(
            act_count = nested_execution.act_executions().len(),
            response_len = combined_response.len(),
            "Completed nested narrative execution"
        );

        // Record the composition as a single act
        let act_execution = ActExecutionBuilder::default()
            .act_name(act_name.to_string())
            .inputs(Vec::new())
            .model(config.model().clone())
            .temperature(*config.temperature())
            .max_tokens(*config.max_tokens())
            .response(combined_response.clone())
            .sequence_number(sequence_number)
            .token_usage(None) // TODO: Aggregate from nested executions
            .estimated_cost_usd(None)
            .duration_ms(None)
            .build()
            .map_err(|e| {
                BotticelliError::from(BackendError::new(format!(
                    "Failed to build ActExecution: {}",
                    e
                )))
            })?;

        // Create message for conversation history
        let message = MessageBuilder::default()
            .role(Role::Assistant)
            .content(vec![Input::Text(combined_response)])
            .build()
            .map_err(|e| {
                NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                    "Failed to build message: {}",
                    e
                )))
            })?;

        Ok((act_execution, message))
    }
}
