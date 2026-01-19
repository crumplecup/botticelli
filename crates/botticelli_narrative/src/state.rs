//! Narrative state management for persistent runtime data.
//!
//! This module provides mechanisms for storing and retrieving state that persists
//! across narrative executions, such as Discord channel IDs, message IDs, and other
//! runtime artifacts.

use botticelli_error::{BotticelliResult, IoError, JsonError};
use derive_getters::Getters;
use elicitation::{Prompt, Select};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, instrument};

/// Represents different scopes for state storage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    elicitation::Elicit,
)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, Getters, elicitation::Elicit)]
pub struct NarrativeState {
    /// The state data
    data: HashMap<String, String>,
}

impl NarrativeState {
    /// Creates a new empty state.
    #[instrument]
    pub fn new() -> Self {
        debug!("Creating new empty state");
        Self {
            data: HashMap::new(),
        }
    }

    /// Gets a value from the state.
    #[instrument(skip(self), fields(key, found))]
    pub fn get(&self, key: &str) -> Option<&str> {
        let result = self.data.get(key).map(|s| s.as_str());
        tracing::Span::current().record("found", result.is_some());
        debug!(key = %key, found = result.is_some(), "Getting state value");
        result
    }

    /// Sets a value in the state.
    #[instrument(skip(self, value), fields(key, value_len))]
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        tracing::Span::current().record("value_len", value.len());
        debug!(key = %key, value_len = value.len(), "Setting state value");
        self.data.insert(key, value);
    }

    /// Removes a value from the state.
    #[instrument(skip(self), fields(key, removed))]
    pub fn remove(&mut self, key: &str) -> Option<String> {
        let result = self.data.remove(key);
        tracing::Span::current().record("removed", result.is_some());
        debug!(key = %key, removed = result.is_some(), "Removing state value");
        result
    }

    /// Checks if a key exists in the state.
    #[instrument(skip(self), fields(key, exists))]
    pub fn contains_key(&self, key: &str) -> bool {
        let exists = self.data.contains_key(key);
        tracing::Span::current().record("exists", exists);
        debug!(key = %key, exists, "Checking key existence");
        exists
    }

    /// Gets all keys in the state.
    #[instrument(skip(self), fields(key_count = self.data.len()))]
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        debug!(key_count = self.data.len(), "Getting all keys");
        self.data.keys().map(|s| s.as_str())
    }

    /// Clears all state.
    #[instrument(skip(self), fields(cleared_count = self.data.len()))]
    pub fn clear(&mut self) {
        let count = self.data.len();
        debug!(cleared_count = count, "Clearing all state");
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
    #[instrument(skip(state_dir), fields(path = %state_dir.as_ref().display()))]
    pub fn new(state_dir: impl AsRef<Path>) -> BotticelliResult<Self> {
        let state_dir = state_dir.as_ref().to_path_buf();

        // Ensure the state directory exists
        if !state_dir.exists() {
            debug!("Creating state directory");
            std::fs::create_dir_all(&state_dir).map_err(|e| {
                error!(path = %state_dir.display(), error = %e, "Failed to create state directory");
                IoError::new(e)
            })?;
        }

        info!("Initialized state manager");
        Ok(Self { state_dir })
    }

    /// Gets the file path for a given scope.
    #[instrument(skip(self), fields(scope = ?scope, filename))]
    fn scope_path(&self, scope: &StateScope) -> PathBuf {
        let filename = match scope {
            StateScope::Global => "global.json".to_string(),
            StateScope::Narrative(name) => format!("narrative_{}.json", name),
            StateScope::Platform { platform, id } => format!("{}_{}.json", platform, id),
        };
        tracing::Span::current().record("filename", &filename);
        debug!(filename = %filename, "Resolved scope path");
        self.state_dir.join(filename)
    }

    /// Loads state for a given scope.
    #[instrument(skip(self), fields(scope = ?scope, path, exists, key_count))]
    pub fn load(&self, scope: &StateScope) -> BotticelliResult<NarrativeState> {
        let path = self.scope_path(scope);
        tracing::Span::current().record("path", path.display().to_string());
        debug!(path = %path.display(), "Loading state from file");

        if !path.exists() {
            tracing::Span::current().record("exists", false);
            tracing::Span::current().record("key_count", 0);
            debug!("No existing state file, returning empty state");
            return Ok(NarrativeState::new());
        }

        tracing::Span::current().record("exists", true);

        let contents = std::fs::read_to_string(&path).map_err(|e| {
            error!(path = %path.display(), error = %e, "Failed to read state file");
            IoError::new(e)
        })?;

        debug!(content_len = contents.len(), "Read state file");

        let state: NarrativeState = serde_json::from_str(&contents).map_err(|e| {
            error!(path = %path.display(), error = %e, "Failed to parse state JSON");
            JsonError::from(e)
        })?;

        tracing::Span::current().record("key_count", state.data.len());
        info!(keys = state.data.len(), "Loaded state successfully");
        Ok(state)
    }

    /// Saves state for a given scope.
    #[instrument(skip(self, state), fields(scope = ?scope, keys = state.data.len(), path, content_len))]
    pub fn save(&self, scope: &StateScope, state: &NarrativeState) -> BotticelliResult<()> {
        let path = self.scope_path(scope);
        tracing::Span::current().record("path", path.display().to_string());
        debug!(path = %path.display(), "Saving state to file");

        let contents = serde_json::to_string_pretty(state).map_err(|e| {
            error!(error = %e, "Failed to serialize state");
            JsonError::from(e)
        })?;

        tracing::Span::current().record("content_len", contents.len());
        debug!(content_len = contents.len(), "Serialized state");

        std::fs::write(&path, contents).map_err(|e| {
            error!(path = %path.display(), error = %e, "Failed to write state file");
            IoError::new(e)
        })?;

        info!("Saved state successfully");
        Ok(())
    }

    /// Deletes state for a given scope.
    #[instrument(skip(self), fields(scope = ?scope, path, existed, deleted))]
    pub fn delete(&self, scope: &StateScope) -> BotticelliResult<()> {
        let path = self.scope_path(scope);
        tracing::Span::current().record("path", path.display().to_string());
        debug!(path = %path.display(), "Deleting state file");

        if path.exists() {
            tracing::Span::current().record("existed", true);
            std::fs::remove_file(&path).map_err(|e| {
                error!(path = %path.display(), error = %e, "Failed to delete state file");
                IoError::new(e)
            })?;
            tracing::Span::current().record("deleted", true);
            info!("Deleted state successfully");
        } else {
            tracing::Span::current().record("existed", false);
            tracing::Span::current().record("deleted", false);
            debug!("State file does not exist, nothing to delete");
        }

        Ok(())
    }
}
