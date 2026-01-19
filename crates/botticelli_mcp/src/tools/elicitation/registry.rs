//! Registry for managing active narrative creation sessions.

use botticelli_error::{McpError, McpResult};
use botticelli_interface::RegistryOperations;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::instrument;

/// Registry managing active narrative creation sessions.
///
/// Generic over types implementing `RegistryOperations`.
#[derive(Debug, Clone)]
pub struct NarrativeRegistry<T: RegistryOperations<Key = String>> {
    narratives: Arc<RwLock<HashMap<String, T>>>,
}

impl<T: RegistryOperations<Key = String>> NarrativeRegistry<T> {
    /// Create a new empty registry.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            narratives: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a narrative to the registry using its registry key.
    ///
    /// Returns the key for tracking this narrative.
    #[tracing::instrument(skip(self, item))]
    pub fn add(&self, item: T) -> String
    where
        T: Clone,
    {
        let key = item.registry_key();
        tracing::debug!(narrative_key = %key, "Adding narrative to registry");

        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        narratives.insert(key.clone(), item);

        tracing::info!(narrative_key = %key, "Added narrative to registry");
        key
    }

    /// Get narrative by key.
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist.
    #[tracing::instrument(skip(self))]
    pub fn get(&self, key: &str) -> McpResult<T>
    where
        T: Clone,
    {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        narratives
            .get(key)
            .cloned()
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", key)))
    }

    /// Update narrative using `update_from_json`.
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist or update fails.
    #[tracing::instrument(skip(self, args))]
    pub fn update(&self, key: &str, args: serde_json::Value) -> McpResult<()> {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");

        let narrative = narratives
            .get_mut(key)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", key)))?;

        narrative
            .update_from_json(args)
            .map_err(|e| McpError::execution_failed(format!("{:?}", e)))?;
        tracing::debug!("Updated narrative");
        Ok(())
    }

    /// Remove a narrative from the registry.
    ///
    /// Called after finalization. Returns the item if it existed.
    #[tracing::instrument(skip(self))]
    pub fn remove(&self, key: &str) -> Option<T> {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let result = narratives.remove(key);

        if result.is_some() {
            tracing::info!("Removed narrative from registry");
        } else {
            tracing::warn!("Narrative not found for removal");
        }

        result
    }

    /// Get narrative by key (alias for `get`).
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist.
    #[tracing::instrument(skip(self))]
    pub fn get_narrative(&self, key: &str) -> McpResult<T>
    where
        T: Clone,
    {
        self.get(key)
    }

    /// Update narrative with a closure.
    ///
    /// # Errors
    ///
    /// Returns error if narrative doesn't exist or update fails.
    #[tracing::instrument(skip(self, update_fn))]
    pub fn update_narrative<F>(&self, key: &str, update_fn: F) -> McpResult<()>
    where
        F: FnOnce(&mut T) -> McpResult<()>,
    {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let narrative = narratives
            .get_mut(key)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", key)))?;

        update_fn(narrative)
    }

    /// Create a new session (alias for `add`).
    #[tracing::instrument(skip(self, item))]
    pub fn create_session(&self, item: T) -> String
    where
        T: Clone,
    {
        self.add(item)
    }

    /// Get all active narrative keys.
    ///
    /// Useful for debugging and monitoring.
    #[tracing::instrument(skip(self))]
    pub fn list_keys(&self) -> Vec<String> {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        let keys: Vec<String> = narratives.keys().cloned().collect();
        tracing::debug!(count = keys.len(), "Retrieved active narrative keys");
        keys
    }

    /// Get all narratives.
    #[tracing::instrument(skip(self))]
    pub fn list_all(&self) -> Vec<T>
    where
        T: Clone,
    {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        narratives.values().cloned().collect()
    }

    /// Clear all narratives.
    ///
    /// Used for testing or cleanup.
    #[tracing::instrument(skip(self))]
    pub fn clear(&self) {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let count = narratives.len();
        narratives.clear();
        tracing::info!(count, "Cleared all narratives from registry");
    }

    /// Get count of active sessions.
    ///
    /// Useful for monitoring and testing.
    #[tracing::instrument(skip(self))]
    pub fn session_count(&self) -> usize {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        narratives.len()
    }
}

impl<T: RegistryOperations<Key = String>> Default for NarrativeRegistry<T> {
    #[instrument]
    fn default() -> Self {
        Self::new()
    }
}

// Implement ElicitationRegistryOperations for the registry
impl<T> botticelli_interface::ElicitationRegistryOperations<T> for NarrativeRegistry<T>
where
    T: RegistryOperations<Key = String> + Clone + Send + Sync + serde::Serialize,
{
    type Error = botticelli_error::BotticelliError;

    #[tracing::instrument(skip(self))]
    fn get_narrative(&self, id: &str) -> Result<T, Self::Error> {
        self.get(id).map_err(Into::into)
    }

    #[tracing::instrument(skip(self, updater))]
    fn update_narrative<F>(&self, id: &str, updater: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut T) -> Result<(), Self::Error>,
    {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let narrative = narratives.get_mut(id).ok_or_else(|| {
            botticelli_error::BotticelliError::from(McpError::invalid_input(format!(
                "Narrative {} not found",
                id
            )))
        })?;

        updater(narrative)
    }

    #[tracing::instrument(skip(self))]
    fn remove_narrative(&self, id: &str) -> Result<Option<T>, Self::Error> {
        Ok(self.remove(id))
    }

    #[tracing::instrument(skip(self, narrative))]
    fn add_narrative(&self, narrative: T) -> String {
        self.add(narrative)
    }

    #[tracing::instrument(skip(self))]
    fn get_narrative_state(&self, id: &str) -> Result<serde_json::Value, Self::Error> {
        let narrative: T = self
            .get(id)
            .map_err(|e: McpError| botticelli_error::BotticelliError::from(e))?;
        // Convert narrative to JSON for state representation
        serde_json::to_value(&narrative)
            .map_err(|e| botticelli_error::BotticelliError::from(McpError::from(e)))
    }

    #[tracing::instrument(skip(self))]
    fn validate_narrative(&self, _id: &str) -> Result<serde_json::Value, Self::Error> {
        // TODO: Implement proper validation
        Ok(serde_json::json!({
            "valid": true
        }))
    }

    #[tracing::instrument(skip(self))]
    fn list_narrative_ids(&self) -> Vec<String> {
        self.list_keys()
    }
}
