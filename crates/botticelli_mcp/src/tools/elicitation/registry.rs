//! Registry for managing active narrative creation sessions.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

/// Registry managing active narrative creation sessions.
///
/// Each session is identified by a UUID and tracks narrative state
/// as JSON during conversational elicitation.
#[derive(Debug, Clone)]
pub struct NarrativeRegistry {
    narratives: Arc<RwLock<HashMap<Uuid, Value>>>,
}

impl NarrativeRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            narratives: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new narrative session with a generated UUID.
    ///
    /// Returns the UUID for tracking this narrative.
    #[instrument(skip(self, state))]
    pub fn create_session(&self, state: Value) -> Uuid {
        let id = Uuid::new_v4();
        debug!(narrative_id = %id, "Creating narrative session");

        let mut narratives = self
            .narratives
            .write()
            .expect("Registry lock poisoned");
        narratives.insert(id, state);

        info!(narrative_id = %id, "Created narrative session");
        id
    }

    /// Get narrative state by UUID.
    ///
    /// Returns None if narrative doesn't exist.
    #[instrument(skip(self), fields(narrative_id = %id))]
    pub fn get(&self, id: &Uuid) -> Option<Value> {
        let narratives = self.narratives.read().expect("Registry lock poisoned");
        let result = narratives.get(id).cloned();

        if result.is_some() {
            debug!("Found narrative");
        } else {
            warn!("Narrative not found");
        }

        result
    }

    /// Update narrative state by UUID.
    ///
    /// Returns true if narrative existed and was updated.
    #[instrument(skip(self, state), fields(narrative_id = %id))]
    pub fn update(&self, id: &Uuid, state: Value) -> bool {
        let mut narratives = self
            .narratives
            .write()
            .expect("Registry lock poisoned");

        if narratives.contains_key(id) {
            narratives.insert(*id, state);
            debug!("Updated narrative");
            true
        } else {
            warn!("Narrative not found for update");
            false
        }
    }

    /// Remove a narrative from the registry.
    ///
    /// Called after finalization. Returns the state if it existed.
    #[instrument(skip(self), fields(narrative_id = %id))]
    pub fn remove(&self, id: &Uuid) -> Option<Value> {
        let mut narratives = self
            .narratives
            .write()
            .expect("Registry lock poisoned");
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
        let mut narratives = self
            .narratives
            .write()
            .expect("Registry lock poisoned");
        let count = narratives.len();
        narratives.clear();
        info!(count, "Cleared all narrative sessions");
    }
}

impl Default for NarrativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_and_get_session() {
        let registry = NarrativeRegistry::new();
        let state = json!({"name": "test", "description": "test narrative"});

        let id = registry.create_session(state.clone());
        let retrieved = registry.get(&id);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap()["name"], "test");
    }

    #[test]
    fn test_update_session() {
        let registry = NarrativeRegistry::new();
        let state = json!({"name": "test"});

        let id = registry.create_session(state);

        let updated = json!({"name": "updated"});
        assert!(registry.update(&id, updated));

        let retrieved = registry.get(&id).expect("Should exist");
        assert_eq!(retrieved["name"], "updated");
    }

    #[test]
    fn test_remove_session() {
        let registry = NarrativeRegistry::new();
        let state = json!({"name": "test"});

        let id = registry.create_session(state);
        assert!(registry.get(&id).is_some());

        let removed = registry.remove(&id);
        assert!(removed.is_some());
        assert!(registry.get(&id).is_none());
    }

    #[test]
    fn test_active_sessions() {
        let registry = NarrativeRegistry::new();
        assert_eq!(registry.active_sessions().len(), 0);

        let state = json!({"name": "test"});
        let id1 = registry.create_session(state.clone());
        let id2 = registry.create_session(state);

        let sessions = registry.active_sessions();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&id1));
        assert!(sessions.contains(&id2));
    }

    #[test]
    fn test_clear() {
        let registry = NarrativeRegistry::new();
        let state = json!({"name": "test"});

        registry.create_session(state.clone());
        registry.create_session(state);
        assert_eq!(registry.active_sessions().len(), 2);

        registry.clear();
        assert_eq!(registry.active_sessions().len(), 0);
    }

    #[test]
    fn test_nonexistent_narrative() {
        let registry = NarrativeRegistry::new();
        let fake_id = Uuid::new_v4();

        assert!(registry.get(&fake_id).is_none());
        assert!(!registry.update(&fake_id, json!({"name": "test"})));
        assert!(registry.remove(&fake_id).is_none());
    }
}
