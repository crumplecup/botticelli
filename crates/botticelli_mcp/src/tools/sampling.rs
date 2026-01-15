use crate::{ConversationSession, ConversationTurn, ToolRegistry};
use botticelli_core::{GenerateResponse, ToolCall, ToolDefinition, ToolResult};
use botticelli_error::{BotticelliResult, SamplingError};
use botticelli_interface::LlmSamplerOperations;
use std::sync::Arc;
use tracing::instrument;

/// Coordinates LLM sampling for narrative generation.
pub struct SamplingCoordinator {
    sampler: Arc<dyn LlmSamplerOperations<
        Session = ConversationSession,
        ToolDefinition = ToolDefinition,
        Response = GenerateResponse,
        Result = SamplingResult,
        Error = SamplingError,
        ToolCall = ToolCall,
        ToolResult = ToolResult,
    >>,
    tool_registry: Arc<ToolRegistry>,
}

impl SamplingCoordinator {
    /// Create a new sampling coordinator.
    pub fn new(
        sampler: Arc<dyn LlmSamplerOperations<
            Session = ConversationSession,
            ToolDefinition = ToolDefinition,
            Response = GenerateResponse,
            Result = SamplingResult,
            Error = SamplingError,
            ToolCall = ToolCall,
            ToolResult = ToolResult,
        >>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            sampler,
            tool_registry,
        }
    }

    /// Generate a narrative from user description.
    #[instrument(skip(self))]
    pub async fn generate_narrative(
        &self,
        description: String,
    ) -> BotticelliResult<PartialNarrative> {
        let system_prompt = SamplingHelper::narrative_generation_prompt();

        // Create session with user message
        let mut session = ConversationSession::new(system_prompt);
        session.add_turn(ConversationTurn::UserMessage {
            content: description,
            attachments: None,
        });

        // Get tool definitions from registry
        let tools = self.tool_registry.tool_definitions();

        // Run sampling
        let _result = self
            .sampler
            .sample(&mut session, &tools)
            .await
            .map_err(|e| {
                botticelli_error::ChatError::new(botticelli_error::ChatErrorKind::ExecutionFailed(
                    e.to_string(),
                ))
            })?;

        // TODO: Extract narrative from session after LLM tool calling
        // For now, return a placeholder
        Ok(PartialNarrative::new())
    }

    /// Refine an existing narrative based on user feedback.
    #[instrument(skip(self, narrative))]
    pub async fn refine_narrative(
        &self,
        narrative: PartialNarrative,
        feedback: String,
    ) -> BotticelliResult<PartialNarrative> {
        let system_prompt = format!(
            "{}\n\nCurrent narrative:\n{:?}",
            SamplingHelper::narrative_generation_prompt(),
            narrative
        );

        let mut session = ConversationSession::new(system_prompt);
        session.add_turn(ConversationTurn::UserMessage {
            content: feedback,
            attachments: None,
        });

        // Get tool definitions from registry
        let tools = self.tool_registry.tool_definitions();

        let _result = self
            .sampler
            .sample(&mut session, &tools)
            .await
            .map_err(|e| {
                botticelli_error::ChatError::new(botticelli_error::ChatErrorKind::ExecutionFailed(
                    e.to_string(),
                ))
            })?;

        // TODO: Apply refinements from LLM tool calling
        Ok(narrative)
    }

    /// Get reference to the tool registry.
    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }
}

/// Result of a sampling session.
#[derive(Debug, Clone)]
pub enum SamplingResult {
    /// Session completed successfully with final text
    Completed {
        /// Final response from the assistant
        final_response: String,
    },
}

// Import PartialNarrative
use crate::PartialNarrative;

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
