//! LLM sampling trait with tool support.

use async_trait::async_trait;

/// Trait for LLM sampling with tool support.
///
/// Provides both low-level (single generation) and high-level (full session)
/// interfaces for maximum flexibility.
///
/// # Associated Types
///
/// Implementations specify concrete types for:
/// - Session state management
/// - Tool definitions and results
/// - Response and result types
/// - Error handling
///
/// # Examples
///
/// ```ignore
/// use botticelli_interface::LlmSamplerOperations;
///
/// pub struct MySampler;
///
/// #[async_trait]
/// impl LlmSamplerOperations for MySampler {
///     type Session = ConversationSession;
///     type ToolDefinition = ToolDefinition;
///     type Response = GenerateResponse;
///     type Result = SamplingResult;
///     type Error = SamplingError;
///     type ToolCall = ToolCall;
///     type ToolResult = ToolResult;
///
///     async fn generate(&self, session: &Self::Session, tools: &[Self::ToolDefinition])
///         -> Result<Self::Response, Self::Error>
///     {
///         // Implementation
///     }
/// }
/// ```
#[async_trait]
pub trait LlmSamplerOperations: Send + Sync {
    /// Session type for conversation state
    type Session;

    /// Tool definition type
    type ToolDefinition;

    /// Response type from generation
    type Response;

    /// Result type for completed sessions
    type Result;

    /// Error type
    type Error: std::error::Error + Send + Sync + 'static;

    /// Tool call type
    type ToolCall;

    /// Tool result type
    type ToolResult;

    /// Low-level: Generate a single response with optional tools.
    ///
    /// This is the core primitive. The high-level `sample()` method
    /// is built on top of this by calling it in a loop.
    ///
    /// # Arguments
    ///
    /// * `session` - Current conversation session state
    /// * `available_tools` - Tools available for this generation
    ///
    /// # Returns
    ///
    /// Response containing text and/or tool calls, or an error
    async fn generate(
        &self,
        session: &Self::Session,
        available_tools: &[Self::ToolDefinition],
    ) -> Result<Self::Response, Self::Error>;

    /// High-level: Run a complete sampling session.
    ///
    /// Starts with the initial session state and runs until:
    /// - The model stops calling tools (returns text)
    /// - Maximum turns reached
    /// - Error occurs
    ///
    /// Default implementation uses `generate()` in a loop, but can be
    /// overridden for custom behavior (streaming, custom termination, etc.)
    ///
    /// # Arguments
    ///
    /// * `session` - Mutable session to update with conversation turns
    /// * `available_tools` - Tools available for execution
    ///
    /// # Returns
    ///
    /// Result containing final response or error
    async fn sample(
        &self,
        session: &mut Self::Session,
        available_tools: &[Self::ToolDefinition],
    ) -> Result<Self::Result, Self::Error>;

    /// Execute tool calls and return results.
    ///
    /// Implementations should delegate to their tool registry to execute
    /// the requested tool calls.
    ///
    /// # Arguments
    ///
    /// * `calls` - Tool calls to execute
    ///
    /// # Returns
    ///
    /// Vector of tool results in the same order as calls, or error
    async fn execute_tools(
        &self,
        calls: &[Self::ToolCall],
    ) -> Result<Vec<Self::ToolResult>, Self::Error>;
}
