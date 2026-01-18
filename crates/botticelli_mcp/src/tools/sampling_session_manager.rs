use crate::{ConversationSession, ConversationTurn};
use botticelli_core::{GenerateResponse, ToolCall, ToolDefinition, ToolResult};
use botticelli_error::{BotticelliResult, ChatError, SamplingError};
use botticelli_interface::LlmSamplerOperations;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

/// Manages multi-turn LLM sampling sessions.
///
/// This is a simplified session manager that delegates LLM conversation
/// management to the LlmSampler implementation.
pub struct SamplingSessionManager {
    sessions: Arc<RwLock<std::collections::HashMap<String, ConversationSession>>>,
}

impl SamplingSessionManager {
    /// Create a new session manager.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Start a new sampling session.
    #[instrument(skip(self, sampler, system_prompt, user_message))]
    pub async fn start_session<S>(
        &self,
        sampler: Arc<S>,
        system_prompt: impl Into<String>,
        user_message: impl Into<String>,
        available_tools: &[ToolDefinition],
    ) -> BotticelliResult<ConversationSession>
    where
        S: LlmSamplerOperations<
            Session = ConversationSession,
            ToolDefinition = ToolDefinition,
            Response = GenerateResponse,
            Result = crate::SamplingResult,
            Error = SamplingError,
            ToolCall = ToolCall,
            ToolResult = ToolResult,
        > + 'static,
    {
        debug!("Starting new sampling session");

        // Create session with user message
        let mut session = ConversationSession::new(system_prompt);
        session.add_turn(ConversationTurn::UserMessage {
            content: user_message.into(),
            attachments: None,
        });

        // Run sampling
        let _result = sampler
            .sample(&mut session, available_tools)
            .await?;

        // Store session
        self.sessions
            .write()
            .await
            .insert(session.id().clone(), session.clone());

        Ok(session)
    }

    /// Get session state.
    #[instrument(skip(self))]
    pub async fn get_session(&self, session_id: &str) -> BotticelliResult<ConversationSession> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                ChatError::validation_error(format!("Session not found: {}", session_id))
            })
            .map_err(Into::into)
    }

    /// List all session IDs.
    #[instrument(skip(self))]
    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Remove a session.
    #[instrument(skip(self))]
    pub async fn remove_session(&self, session_id: &str) -> BotticelliResult<()> {
        self.sessions
            .write()
            .await
            .remove(session_id)
            .ok_or_else(|| {
                ChatError::validation_error(format!("Session not found: {}", session_id))
            })?;
        Ok(())
    }
}

impl Default for SamplingSessionManager {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}
