//! MCP tools for iterative narrative creation through LLM-guided elicitation.

use crate::{PartialNarrative, PartialNarrativeBuilder};
use botticelli_error::{BotticelliResult, BuilderError, BuilderErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

/// Registry for managing in-progress narrative creation sessions.
#[derive(Debug, Clone)]
pub struct NarrativeRegistry {
    sessions: Arc<RwLock<std::collections::HashMap<String, PartialNarrative>>>,
}

impl NarrativeRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Starts a new narrative creation session.
    #[instrument(skip(self))]
    pub async fn create_session(
        &self,
        session_id: String,
        description: String,
    ) -> BotticelliResult<()> {
        let mut sessions = self.sessions.write().await;
        let partial = PartialNarrativeBuilder::default()
            .description(Some(description))
            .build()
            .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e.to_string())))?;
        sessions.insert(session_id, partial);
        Ok(())
    }

    /// Retrieves a session.
    #[instrument(skip(self))]
    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> BotticelliResult<Option<PartialNarrative>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    /// Updates a session.
    #[instrument(skip(self, partial))]
    pub async fn update_session(
        &self,
        session_id: &str,
        partial: PartialNarrative,
    ) -> BotticelliResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), partial);
        Ok(())
    }

    /// Removes a session.
    #[instrument(skip(self))]
    pub async fn remove_session(
        &self,
        session_id: &str,
    ) -> BotticelliResult<Option<PartialNarrative>> {
        let mut sessions = self.sessions.write().await;
        Ok(sessions.remove(session_id))
    }

    /// Lists all active sessions.
    #[instrument(skip(self))]
    pub async fn list_sessions(&self) -> BotticelliResult<Vec<String>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }
}

impl Default for NarrativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Input for starting a narrative creation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartNarrativeInput {
    /// Unique session identifier.
    pub session_id: String,
    /// High-level description of the narrative to create.
    pub description: String,
}

/// Input for eliciting narrative metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitMetadataInput {
    /// Session identifier.
    pub session_id: String,
}

/// Input for eliciting a narrative act.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitActInput {
    /// Session identifier.
    pub session_id: String,
    /// Act number (1-indexed).
    pub act_number: usize,
}

/// Input for finalizing a narrative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeNarrativeInput {
    /// Session identifier.
    pub session_id: String,
}

/// Tool for starting a narrative creation session.
#[derive(Debug, Clone)]
pub struct StartNarrativeTool {
    registry: NarrativeRegistry,
}

impl StartNarrativeTool {
    /// Creates a new tool instance.
    pub fn new(registry: NarrativeRegistry) -> Self {
        Self { registry }
    }

    /// Executes the tool.
    #[instrument(skip(self))]
    pub async fn execute(&self, input: StartNarrativeInput) -> BotticelliResult<JsonValue> {
        self.registry
            .create_session(input.session_id.clone(), input.description)
            .await?;

        Ok(serde_json::json!({
            "status": "started",
            "session_id": input.session_id,
            "next_step": "elicit_metadata"
        }))
    }
}
