//! Registry for managing active narrative creation sessions.

use crate::{RegistryOperations};
use botticelli_error::{McpError, McpResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

/// Registry managing active narrative creation sessions.
///
/// Generic over types implementing `RegistryOperations`.
#[derive(Debug, Clone)]
pub struct NarrativeRegistry<T: RegistryOperations> {
    narratives: Arc<RwLock<HashMap<Uuid, T>>>,
}

impl<T: RegistryOperations> NarrativeRegistry<T> {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            narratives: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new narrative session with a generated UUID.
    ///
    /// Returns the UUID for tracking this narrative.
    #[instrument(skip(self, item))]
    pub fn create_session(&self, item: T) -> Uuid {
        let id = Uuid::new_v4();
        debug!(narrative_id = %id, "Creating narrative session");

        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        narratives.insert(id, item);

        info!(narrative_id = %id, "Created narrative session");
        id
    }

    /// Get narrative by UUID.
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist.
    #[instrument(skip(self), fields(narrative_id = %id))]
    pub fn get_narrative(&self, id: Uuid) -> McpResult<T>
    where
        T: Clone,
    {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        narratives
            .get(&id)
            .cloned()
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", id)))
    }

    /// Update narrative using a closure.
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist or update fails.
    #[instrument(skip(self, update_fn), fields(narrative_id = %id))]
    pub async fn update_narrative<F>(&self, id: Uuid, update_fn: F) -> McpResult<()>
    where
        F: FnOnce(&mut T) -> McpResult<()>,
        T: Clone,
    {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");

        let narrative = narratives
            .get_mut(&id)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", id)))?;

        update_fn(narrative)?;
        debug!("Updated narrative");
        Ok(())
    }

    /// Remove a narrative from the registry.
    ///
    /// Called after finalization. Returns the item if it existed.
    #[instrument(skip(self), fields(narrative_id = %id))]
    pub fn remove(&self, id: &Uuid) -> Option<T> {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let result = narratives.remove(id);

        if result.is_some() {
            info!("Removed narrative session");
        } else {
            warn!("Narrative not found for removal");
        }

        result
    }

    /// Get all active narrative IDs.
    ///
    /// Useful for debugging and monitoring.
    #[instrument(skip(self))]
    pub fn active_sessions(&self) -> Vec<Uuid> {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        let sessions: Vec<Uuid> = narratives.keys().copied().collect();
        debug!(count = sessions.len(), "Retrieved active sessions");
        sessions
    }

    /// Clear all sessions.
    ///
    /// Used for testing or cleanup.
    #[instrument(skip(self))]
    pub fn clear(&self) {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let count = narratives.len();
        narratives.clear();
        info!(count, "Cleared all narrative sessions");
    }
}

impl<T: RegistryOperations> Default for NarrativeRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}
