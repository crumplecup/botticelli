use crate::{LlmSampler, SamplingSession};
use botticelli_error::{BotticelliResult, ChatError};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

/// Manages multi-turn LLM sampling sessions.
///
/// This is a simplified session manager that delegates LLM conversation
/// management to the LlmSampler implementation.
pub struct SamplingSessionManager {
    sessions: Arc<RwLock<std::collections::HashMap<String, SamplingSession>>>,
}

impl SamplingSessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Start a new sampling session.
    #[instrument(skip(self, sampler))]
    pub async fn start_session(
        &self,
        sampler: Arc<dyn LlmSampler>,
        system_prompt: &str,
        user_message: &str,
    ) -> BotticelliResult<SamplingSession> {
        debug!("Starting new sampling session");

        // Delegate to sampler
        let session = sampler.sample(system_prompt, user_message).await?;

        // Store session
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());

        Ok(session)
    }

    /// Get session state.
    #[instrument(skip(self))]
    pub async fn get_session(&self, session_id: &str) -> BotticelliResult<SamplingSession> {
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
    fn default() -> Self {
        Self::new()
    }
}
