use crate::{ConversationSession, ConversationTurn, SessionState, ToolRegistry, ToolResult};
use botticelli_core::{GenerateResponse, ToolCall, ToolDefinition};
use botticelli_error::BotticelliResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// Coordinates LLM sampling for narrative generation.
pub struct SamplingCoordinator {
    sampler: Arc<dyn LlmSampler>,
    tool_registry: Arc<ToolRegistry>,
}

impl SamplingCoordinator {
    /// Create a new sampling coordinator.
    pub fn new(sampler: Arc<dyn LlmSampler>, tool_registry: Arc<ToolRegistry>) -> Self {
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

/// Trait for LLM sampling with tool support.
///
/// Provides both low-level (single generation) and high-level (full session)
/// interfaces for maximum flexibility.
#[async_trait::async_trait]
pub trait LlmSampler: Send + Sync {
    /// Low-level: Generate a single response with optional tools.
    ///
    /// This is the core primitive. The high-level `sample()` method
    /// is built on top of this by calling it in a loop.
    async fn generate(
        &self,
        session: &ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError>;

    /// High-level: Run a complete sampling session.
    ///
    /// Starts with the initial session state and runs until:
    /// - The model stops calling tools (returns text)
    /// - Maximum turns reached
    /// - Error occurs
    ///
    /// Default implementation uses generate() in a loop, but can be
    /// overridden for custom behavior (streaming, custom termination, etc.)
    async fn sample(
        &self,
        session: &mut ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<SamplingResult, SamplingError> {
        while session.is_active() {
            // Generate next response
            let response = self.generate(session, available_tools).await?;

            // Process response based on outputs
            let has_tool_calls = !response.outputs().is_empty()
                && response
                    .outputs()
                    .iter()
                    .any(|o| matches!(o, botticelli_core::Output::ToolCalls(_)));

            if has_tool_calls {
                // Extract tool calls from outputs
                let mut all_calls = vec![];
                let mut thinking_text = String::new();

                for output in response.outputs() {
                    match output {
                        botticelli_core::Output::Text(text) => {
                            if !thinking_text.is_empty() {
                                thinking_text.push(' ');
                            }
                            thinking_text.push_str(text);
                        }
                        botticelli_core::Output::ToolCalls(calls) => {
                            all_calls.extend(calls.clone());
                        }
                        _ => {}
                    }
                }

                let thinking = if thinking_text.is_empty() {
                    None
                } else {
                    Some(thinking_text)
                };

                session.add_turn(ConversationTurn::AssistantToolCalls {
                    calls: all_calls.clone(),
                    thinking,
                });

                // Execute tools
                let results = self.execute_tools(&all_calls).await?;
                session.add_turn(ConversationTurn::ToolResults { results });

                // Continue loop for next turn
            } else {
                // Model is done (no tool calls)
                let text = response
                    .outputs()
                    .iter()
                    .filter_map(|o| match o {
                        botticelli_core::Output::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                if !text.is_empty() {
                    session.add_turn(ConversationTurn::AssistantMessage {
                        content: text.clone(),
                    });
                }

                session.state = SessionState::Completed;
                return Ok(SamplingResult::Completed {
                    final_response: text,
                });
            }
        }

        // Session ended without completion
        Err(SamplingError::new(SamplingErrorKind::MaxTurnsExceeded {
            max: session.max_turns,
        }))
    }

    /// Execute tool calls and return results.
    ///
    /// Default implementation returns errors - must be overridden
    /// to provide actual tool execution.
    async fn execute_tools(&self, _calls: &[ToolCall]) -> Result<Vec<ToolResult>, SamplingError> {
        Err(SamplingError::new(SamplingErrorKind::NoToolRegistry))
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

/// Errors from sampling operations.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Sampling: {} at {}:{}", kind, file, line)]
pub struct SamplingError {
    /// Error kind
    pub kind: SamplingErrorKind,
    /// Line number
    pub line: u32,
    /// File name
    pub file: &'static str,
}

/// Types of sampling errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum SamplingErrorKind {
    /// Max turns exceeded
    #[display("Max turns exceeded: {}", max)]
    MaxTurnsExceeded {
        /// Maximum turns allowed
        max: usize,
    },

    /// Tool execution failed
    #[display("Tool execution failed: {} - {}", tool_name, reason)]
    ToolExecutionFailed {
        /// Tool name
        tool_name: String,
        /// Reason for failure
        reason: String,
    },

    /// Unknown tool
    #[display("Unknown tool: {}", name)]
    UnknownTool {
        /// Tool name
        name: String,
    },

    /// Provider error
    #[display("Provider error: {}", _0)]
    ProviderError(String),

    /// No tool registry configured
    #[display("No tool registry configured")]
    NoToolRegistry,

    /// Request building failed
    #[display("Request building failed: {}", _0)]
    RequestBuildingFailed(String),
}

impl SamplingError {
    /// Create a new sampling error with location tracking.
    #[track_caller]
    pub fn new(kind: SamplingErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
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
