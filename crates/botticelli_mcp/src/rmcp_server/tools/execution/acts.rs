//! Single act execution tools.
//!
//! Execute individual narrative acts with optional context from previous acts.

use crate::rmcp_server::BotticelliServer;
use crate::{ExecuteActParams, ExecuteActResult};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

impl BotticelliServer {
    /// Execute a single narrative act with context.
    #[instrument(skip(self, params), fields(model = params.model(), max_tokens = params.max_tokens(), prompt_len = params.prompt().len(), has_context = params.context().is_some()))]
    pub async fn execute_act(
        &self,
        Parameters(params): Parameters<ExecuteActParams>,
    ) -> Result<Json<ExecuteActResult>, rmcp::ErrorData> {
        let prompt = params.prompt().clone();
        let model = params.model().clone();
        let max_tokens = *params.max_tokens();
        let context = params.context().clone();

        debug!(%model, has_context = context.is_some(), "Executing act");

        // Placeholder implementation
        // Full implementation would:
        // 1. Select driver based on model prefix
        // 2. Build message with system prompt, context, and user prompt
        // 3. Execute with driver
        // 4. Return response with token usage

        let response = format!(
            "Act execution placeholder\n\nPrompt: {}\nModel: {}\nContext: {}\n\nFull execution requires LLM backend integration.",
            prompt,
            model,
            context.as_deref().unwrap_or("(none)")
        );

        debug!(response_len = response.len(), "Act execution complete");

        Ok(Json(ExecuteActResult::new(
            response,
            model,
            Some(max_tokens),
            true,
        )))
    }
}
