use botticelli_mcp::{LlmSampler, SamplingSession};
use botticelli_error::BotticelliResult;
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

impl LlmSampler for ChatLlmSampler {
    #[instrument(skip(self))]
    async fn sample(&self, _system_prompt: &str, _user_message: &str) -> BotticelliResult<SamplingSession> {
        // TODO: Implement full sampling loop with:
        // 1. LLM client for generation
        // 2. Tool call extraction from responses
        // 3. Tool execution
        // 4. Multi-turn conversation management
        
        unimplemented!("ChatLlmSampler requires LlmClient trait implementation")
    }
}
