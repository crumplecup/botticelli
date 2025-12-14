use botticelli_core::GenerateResponse;
use botticelli_mcp::{ConversationSession, LlmSampler, SamplingError, ToolDefinition};
use tracing::instrument;

/// Placeholder for chat LLM sampler implementation.
///
/// This will be fully implemented once we have the LlmClient trait
/// and proper tool call extraction logic.
pub struct ChatLlmSampler;

impl ChatLlmSampler {
    /// Creates a new chat LLM sampler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ChatLlmSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmSampler for ChatLlmSampler {
    #[instrument(skip(self, _session, _available_tools))]
    async fn generate(
        &self,
        _session: &ConversationSession,
        _available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError> {
        // TODO: Implement single-turn generation with:
        // 1. Convert session to GenerateRequest
        // 2. Call LLM provider
        // 3. Return GenerateResponse

        unimplemented!("ChatLlmSampler::generate requires LlmProvider implementation")
    }

    // Note: sample() and execute_tools() use default implementations from trait
}
