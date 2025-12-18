//! Conversation persistence and storage.
//!
//! Handles saving and loading conversations to/from disk.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use tracing::{debug, error, info, warn};

use crate::{ChatMessage, ConversationId, TuiError, TuiErrorKind, TuiResult};

/// Storage for conversation persistence.
///
/// Manages saving and loading conversations to/from the filesystem.
/// Conversations are stored as JSON files in `~/.config/botticelli/conversations/`.
#[derive(Clone)]
pub struct ConversationStorage {
    /// Directory where conversations are stored.
    conversations_dir: PathBuf,
}

impl ConversationStorage {
    /// Creates a new conversation storage.
    ///
    /// Initializes the storage directory if it doesn't exist.
    pub fn new() -> TuiResult<Self> {
        // Get config directory: ~/.config/botticelli/conversations/
        let config_dir = dirs::config_dir()
            .ok_or_else(|| {
                TuiError::new(TuiErrorKind::Storage(
                    "Failed to get config directory".to_string(),
                ))
            })?
            .join("botticelli")
            .join("conversations");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                TuiError::new(TuiErrorKind::Storage(format!(
                    "Failed to create conversations directory: {}",
                    e
                )))
            })?;
            info!(path = ?config_dir, "Created conversations directory");
        }

        Ok(Self {
            conversations_dir: config_dir,
        })
    }

    /// Saves a conversation to disk.
    ///
    /// Writes the conversation messages to a JSON file named `{uuid}.json`.
    pub fn save(&self, id: &ConversationId, messages: &[ChatMessage]) -> TuiResult<()> {
        let file_path = self.conversations_dir.join(format!("{}.json", id));

        let json = serde_json::to_string_pretty(messages).map_err(|e| {
            TuiError::new(TuiErrorKind::Storage(format!(
                "Failed to serialize conversation: {}",
                e
            )))
        })?;

        fs::write(&file_path, json).map_err(|e| {
            TuiError::new(TuiErrorKind::Storage(format!(
                "Failed to write conversation file: {}",
                e
            )))
        })?;

        debug!(
            conversation_id = %id,
            path = ?file_path,
            message_count = messages.len(),
            "Saved conversation"
        );

        Ok(())
    }

    /// Loads a specific conversation from disk.
    ///
    /// Returns `None` if the conversation file doesn't exist.
    pub fn load(&self, id: &ConversationId) -> TuiResult<Option<Vec<ChatMessage>>> {
        let file_path = self.conversations_dir.join(format!("{}.json", id));

        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path).map_err(|e| {
            TuiError::new(TuiErrorKind::Storage(format!(
                "Failed to read conversation file: {}",
                e
            )))
        })?;

        let messages: Vec<ChatMessage> = serde_json::from_str(&json).map_err(|e| {
            TuiError::new(TuiErrorKind::Storage(format!(
                "Failed to deserialize conversation: {}",
                e
            )))
        })?;

        debug!(
            conversation_id = %id,
            path = ?file_path,
            message_count = messages.len(),
            "Loaded conversation"
        );

        Ok(Some(messages))
    }

    /// Loads all conversations from disk.
    ///
    /// Returns a HashMap of conversation IDs to messages.
    /// Skips files that fail to load (with warning log).
    pub fn load_all(&self) -> TuiResult<HashMap<ConversationId, Vec<ChatMessage>>> {
        let mut conversations = HashMap::new();

        // Read directory entries
        let entries = fs::read_dir(&self.conversations_dir).map_err(|e| {
            TuiError::new(TuiErrorKind::Storage(format!(
                "Failed to read conversations directory: {}",
                e
            )))
        })?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "Failed to read directory entry");
                    continue;
                }
            };

            let path = entry.path();

            // Skip non-JSON files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Parse UUID from filename
            let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };

            let conversation_id = match uuid::Uuid::parse_str(file_stem) {
                Ok(id) => id,
                Err(e) => {
                    warn!(file = ?path, error = %e, "Invalid UUID in filename");
                    continue;
                }
            };

            // Load conversation
            match self.load(&conversation_id) {
                Ok(Some(messages)) => {
                    conversations.insert(conversation_id, messages);
                }
                Ok(None) => {
                    warn!(conversation_id = %conversation_id, "Conversation file not found");
                }
                Err(e) => {
                    error!(conversation_id = %conversation_id, error = %e, "Failed to load conversation");
                }
            }
        }

        info!(
            count = conversations.len(),
            "Loaded conversations from disk"
        );

        Ok(conversations)
    }

    /// Deletes a conversation from disk.
    pub fn delete(&self, id: &ConversationId) -> TuiResult<()> {
        let file_path = self.conversations_dir.join(format!("{}.json", id));

        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| {
                TuiError::new(TuiErrorKind::Storage(format!(
                    "Failed to delete conversation file: {}",
                    e
                )))
            })?;

            info!(conversation_id = %id, path = ?file_path, "Deleted conversation");
        }

        Ok(())
    }
}

impl Default for ConversationStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create conversation storage")
    }
}
