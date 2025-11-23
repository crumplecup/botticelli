//! Narrative state management for persistent runtime data.
//!
//! This module provides mechanisms for storing and retrieving state that persists
//! across narrative executions, such as Discord channel IDs, message IDs, and other
//! runtime artifacts.

use botticelli_error::{BotticelliResult, ConfigError, JsonError};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Represents different scopes for state storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateScope {
    /// Global state shared across all narratives
    Global,
    /// State scoped to a specific narrative
    Narrative(String),
    /// State scoped to a specific platform (e.g., Discord server)
    Platform {
        /// Platform name (e.g., "discord")
        platform: String,
        /// Platform-specific ID (e.g., guild_id)
        id: String,
    },
}

/// A key-value store for narrative state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NarrativeState {
    /// The state data
    data: HashMap<String, String>,
}

impl NarrativeState {
    /// Creates a new empty state.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Gets a value from the state.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Sets a value in the state.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        debug!(key = %key, value = %value, "Setting state value");
        self.data.insert(key, value);
    }

    /// Removes a value from the state.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        debug!(key = %key, "Removing state value");
        self.data.remove(key)
    }

    /// Checks if a key exists in the state.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Gets all keys in the state.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.data.keys().map(|s| s.as_str())
    }

    /// Clears all state.
    pub fn clear(&mut self) {
        debug!("Clearing all state");
        self.data.clear();
    }
}

/// Manages narrative state persistence.
#[derive(Debug, Clone, Getters)]
pub struct StateManager {
    /// Base directory for state files
    state_dir: PathBuf,
}

impl StateManager {
    /// Creates a new state manager.
    ///
    /// # Arguments
    ///
    /// * `state_dir` - Directory where state files will be stored
    pub fn new(state_dir: impl AsRef<Path>) -> BotticelliResult<Self> {
        let state_dir = state_dir.as_ref().to_path_buf();
        
        // Ensure the state directory exists
        if !state_dir.exists() {
            std::fs::create_dir_all(&state_dir).map_err(|e| {
                ConfigError::new(format!("Failed to create state directory: {}", e))
            })?;
        }

        debug!(path = %state_dir.display(), "Initialized state manager");
        Ok(Self { state_dir })
    }

    /// Gets the file path for a given scope.
    fn scope_path(&self, scope: &StateScope) -> PathBuf {
        let filename = match scope {
            StateScope::Global => "global.json".to_string(),
            StateScope::Narrative(name) => format!("narrative_{}.json", name),
            StateScope::Platform { platform, id } => format!("{}_{}.json", platform, id),
        };
        self.state_dir.join(filename)
    }

    /// Loads state for a given scope.
    pub fn load(&self, scope: &StateScope) -> BotticelliResult<NarrativeState> {
        let path = self.scope_path(scope);
        
        if !path.exists() {
            debug!(scope = ?scope, "No existing state file, returning empty state");
            return Ok(NarrativeState::new());
        }

        let contents = std::fs::read_to_string(&path).map_err(|e| {
            ConfigError::new(format!("Failed to read state file: {}", e))
        })?;

        let state: NarrativeState = serde_json::from_str(&contents).map_err(|e| {
            JsonError::new(format!("Failed to parse state file: {}", e))
        })?;

        debug!(scope = ?scope, keys = state.data.len(), "Loaded state");
        Ok(state)
    }

    /// Saves state for a given scope.
    pub fn save(&self, scope: &StateScope, state: &NarrativeState) -> BotticelliResult<()> {
        let path = self.scope_path(scope);
        
        let contents = serde_json::to_string_pretty(state).map_err(|e| {
            JsonError::new(format!("Failed to serialize state: {}", e))
        })?;

        std::fs::write(&path, contents).map_err(|e| {
            ConfigError::new(format!("Failed to write state file: {}", e))
        })?;

        debug!(scope = ?scope, keys = state.data.len(), "Saved state");
        Ok(())
    }

    /// Deletes state for a given scope.
    pub fn delete(&self, scope: &StateScope) -> BotticelliResult<()> {
        let path = self.scope_path(scope);
        
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                ConfigError::new(format!("Failed to delete state file: {}", e))
            })?;
            debug!(scope = ?scope, "Deleted state");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_narrative_state() {
        let mut state = NarrativeState::new();
        
        // Test set and get
        state.set("channel_id", "123456");
        assert_eq!(state.get("channel_id"), Some("123456"));
        
        // Test contains_key
        assert!(state.contains_key("channel_id"));
        assert!(!state.contains_key("nonexistent"));
        
        // Test remove
        assert_eq!(state.remove("channel_id"), Some("123456".to_string()));
        assert_eq!(state.get("channel_id"), None);
        
        // Test clear
        state.set("key1", "value1");
        state.set("key2", "value2");
        state.clear();
        assert_eq!(state.get("key1"), None);
        assert_eq!(state.get("key2"), None);
    }

    #[test]
    fn test_state_manager() {
        let temp_dir = env::temp_dir().join("botticelli_state_test");
        let manager = StateManager::new(&temp_dir).unwrap();
        
        // Test save and load
        let mut state = NarrativeState::new();
        state.set("test_key", "test_value");
        
        let scope = StateScope::Narrative("test_narrative".to_string());
        manager.save(&scope, &state).unwrap();
        
        let loaded = manager.load(&scope).unwrap();
        assert_eq!(loaded.get("test_key"), Some("test_value"));
        
        // Test delete
        manager.delete(&scope).unwrap();
        let loaded = manager.load(&scope).unwrap();
        assert_eq!(loaded.get("test_key"), None);
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
