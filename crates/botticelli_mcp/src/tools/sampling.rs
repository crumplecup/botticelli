use botticelli_core::{GenerateRequest, GenerateResponse, ToolCall};
use botticelli_error::BotticelliResult;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Trait for executing LLM sampling with tool access.
pub trait LlmSampler: Send + Sync {
    /// Execute a sampling session with the given system prompt and initial user message.
    ///
    /// The LLM will orchestrate tool calls as needed through multi-turn conversation.
    fn sample(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> impl std::future::Future<Output = BotticelliResult<SamplingSession>> + Send;
}

/// A multi-turn sampling session.
#[derive(Debug, Clone)]
pub struct SamplingSession {
    /// Session identifier.
    pub id: String,
    /// Conversation history.
    pub history: Vec<Turn>,
    /// Current state.
    pub state: SessionState,
}

/// State of a sampling session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session is active and accepting input.
    Active,
    /// Session completed successfully.
    Completed,
    /// Session failed with error.
    Failed(String),
}

/// A single turn in the conversation.
#[derive(Debug, Clone)]
pub struct Turn {
    /// The request sent to the LLM.
    pub request: GenerateRequest,
    /// The response from the LLM.
    pub response: GenerateResponse,
    /// Tool calls made during this turn.
    pub tool_calls: Vec<ToolCall>,
    /// Tool responses received during this turn.
    pub tool_responses: Vec<ToolResponse>,
}

/// Response from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// ID of the tool call this responds to.
    pub tool_call_id: String,
    /// Result of the tool execution.
    pub result: serde_json::Value,
}

/// Helper for LLM sampling operations.
pub struct SamplingHelper;

impl SamplingHelper {
    /// Create a system prompt for narrative generation.
    #[instrument]
    pub fn narrative_generation_prompt() -> String {
        r#"You are an expert narrative designer helping create interactive story content.

Your task is to guide the user through creating a narrative using the available MCP tools.

Available tools:
- create_narrative: Initialize a new narrative with metadata
- add_act: Add an act to the narrative
- add_input: Add an input (image/video) to an act
- add_carousel: Add a carousel of images to an act
- validate_narrative: Validate the complete narrative structure
- get_narrative: Retrieve current narrative state

Workflow:
1. Start by gathering basic metadata (title, description, tags)
2. Use create_narrative with the metadata
3. For each act, gather content and use add_act
4. Add media inputs as needed with add_input or add_carousel
5. Validate the final structure with validate_narrative

Guide the conversation naturally - ask clarifying questions, provide examples, and explain validation errors.
When the user provides content, call the appropriate tools to build the narrative incrementally.
"#.to_string()
    }

    /// Create a system prompt for narrative execution orchestration.
    #[instrument]
    pub fn execution_orchestration_prompt() -> String {
        r#"You are an execution coordinator for interactive narrative experiences.

Your task is to orchestrate the execution of a narrative using the available tools.

Available tools:
- load_narrative: Load a narrative by ID
- execute_act: Execute a specific act
- get_execution_state: Get current execution state
- handle_user_choice: Process user interaction choice

Workflow:
1. Load the narrative
2. Execute acts in sequence
3. Handle user interactions as they occur
4. Track state and provide feedback
5. Handle errors gracefully with fallbacks

Monitor execution state and provide clear feedback about progress and any issues.
"#
        .to_string()
    }
}
