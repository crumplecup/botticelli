//! Registry for managing active narrative creation sessions.

use botticelli_interface::{RegistryOperations, NarrativeRegistryOperations};
use botticelli_error::{McpError, McpResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry managing active narrative creation sessions.
///
/// Generic over types implementing `RegistryOperations`.
#[derive(Debug, Clone)]
pub struct NarrativeRegistry<T: RegistryOperations<Key = String>> {
    narratives: Arc<RwLock<HashMap<String, T>>>,
}

impl<T: RegistryOperations<Key = String>> NarrativeRegistry<T> {
    /// Create a new empty registry.
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
    pub fn update(&self, key: &str, args: serde_json::Value) -> McpResult<()>
    {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");

        let narrative = narratives
            .get_mut(key)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", key)))?;

        narrative.update_from_json(args)?;
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
}

impl<T: RegistryOperations<Key = String>> Default for NarrativeRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NarrativeRegistryOperations for NarrativeRegistry<T>
where
    T: RegistryOperations<Key = String> + Clone,
{
    type Narrative = T;

    fn get_narrative(&self, id: &str) -> McpResult<Self::Narrative> {
        self.get(id)
    }

    fn update_narrative<F>(&self, id: &str, update_fn: F) -> McpResult<()>
    where
        F: FnOnce(&mut Self::Narrative) -> McpResult<()>,
    {
        let mut narratives = self.narratives.write().expect("Registry lock poisoned");
        let narrative = narratives
            .get_mut(id)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", id)))?;
        
        update_fn(narrative)
    }

    fn create_session(&mut self, narrative: Self::Narrative) -> String {
        self.add(narrative)
    }

    fn finalize_narrative(&self, id: &str) -> McpResult<Self::Narrative> {
        self.remove(id)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", id)))
    }
}
