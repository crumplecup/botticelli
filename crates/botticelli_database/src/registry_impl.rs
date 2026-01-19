//! RegistryOperations implementations for database types.

use botticelli_error::{BotticelliResult, McpError, McpErrorKind};
use botticelli_interface::RegistryOperations;
use rmcp::tool;
use serde_json::Value;
use uuid::Uuid;

/// Actor registry entry for MCP tool operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, derive_getters::Getters)]
pub struct ActorRow {
    id: Uuid,
    name: String,
    description: Option<String>,
}

impl ActorRow {
    /// Create a new actor entry.
    #[tool]
    #[tracing::instrument(skip(name, description), fields(name_len = name.len(), has_description = description.is_some()))]
    pub fn new(name: String, description: Option<String>) -> Self {
        let id = Uuid::new_v4();
        tracing::debug!(id = %id, "Created new ActorRow");
        Self {
            id,
            name,
            description,
        }
    }
}

impl RegistryOperations for ActorRow {
    type Error = botticelli_error::BotticelliError;
    type Key = Uuid;

    #[tracing::instrument(skip(self))]
    fn registry_key(&self) -> Self::Key {
        self.id
    }

    #[tracing::instrument(skip(args), fields(has_id = args.get("id").is_some(), has_name = args.get("name").is_some()))]
    fn from_json_args(args: Value) -> BotticelliResult<Self> {
        tracing::debug!("Parsing ActorRow from JSON args");
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("Missing 'name' in arguments");
                McpError::new(McpErrorKind::InvalidArguments {
                    tool: "ActorRow".to_string(),
                    reason: "Missing 'name'".to_string(),
                })
            })?
            .to_string();

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        tracing::debug!(id = %id, name = %name, "Parsed ActorRow");
        Ok(Self {
            id,
            name,
            description,
        })
    }

    #[tracing::instrument(skip(self))]
    fn to_json(&self) -> BotticelliResult<Value> {
        tracing::debug!(id = %self.id, "Converting ActorRow to JSON");
        serde_json::to_value(self).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize ActorRow");
            McpError::new(McpErrorKind::ExecutionError(e.to_string())).into()
        })
    }

    #[tracing::instrument(skip(self, args), fields(has_name = args.get("name").is_some(), has_description = args.get("description").is_some()))]
    fn update_from_json(&mut self, args: Value) -> BotticelliResult<()> {
        tracing::debug!(id = %self.id, "Updating ActorRow from JSON");
        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            self.name = name.to_string();
            tracing::debug!("Updated name");
        }
        if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
            self.description = Some(description.to_string());
            tracing::debug!("Updated description");
        }
        tracing::info!(id = %self.id, "Updated ActorRow");
        Ok(())
    }
}

/// Content registry entry for MCP tool operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, derive_getters::Getters)]
pub struct ContentEntry {
    id: Uuid,
    title: String,
    content_type: String,
    data: Value,
}

impl ContentEntry {
    /// Create a new content entry.
    #[tool]
    #[tracing::instrument(skip(title, content_type, data), fields(title_len = title.len(), content_type_len = content_type.len()))]
    pub fn new(title: String, content_type: String, data: Value) -> Self {
        let id = Uuid::new_v4();
        tracing::debug!(id = %id, "Created new ContentEntry");
        Self {
            id,
            title,
            content_type,
            data,
        }
    }
}

impl RegistryOperations for ContentEntry {
    type Error = botticelli_error::BotticelliError;
    type Key = Uuid;

    #[tracing::instrument(skip(self))]
    fn registry_key(&self) -> Self::Key {
        self.id
    }

    #[tracing::instrument(skip(args), fields(has_id = args.get("id").is_some(), has_title = args.get("title").is_some(), has_content_type = args.get("content_type").is_some()))]
    fn from_json_args(args: Value) -> BotticelliResult<Self> {
        tracing::debug!("Parsing ContentEntry from JSON args");
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("Missing 'title' in arguments");
                McpError::new(McpErrorKind::InvalidArguments {
                    tool: "ContentEntry".to_string(),
                    reason: "Missing 'title'".to_string(),
                })
            })?
            .to_string();

        let content_type = args
            .get("content_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("Missing 'content_type' in arguments");
                McpError::new(McpErrorKind::InvalidArguments {
                    tool: "ContentEntry".to_string(),
                    reason: "Missing 'content_type'".to_string(),
                })
            })?
            .to_string();

        let data = args
            .get("data")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        tracing::debug!(id = %id, title = %title, content_type = %content_type, "Parsed ContentEntry");
        Ok(Self {
            id,
            title,
            content_type,
            data,
        })
    }

    #[tracing::instrument(skip(self))]
    fn to_json(&self) -> BotticelliResult<Value> {
        tracing::debug!(id = %self.id, "Converting ContentEntry to JSON");
        serde_json::to_value(self).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize ContentEntry");
            McpError::new(McpErrorKind::ExecutionError(e.to_string())).into()
        })
    }

    #[tracing::instrument(skip(self, args), fields(has_title = args.get("title").is_some(), has_content_type = args.get("content_type").is_some(), has_data = args.get("data").is_some()))]
    fn update_from_json(&mut self, args: Value) -> BotticelliResult<()> {
        tracing::debug!(id = %self.id, "Updating ContentEntry from JSON");
        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            self.title = title.to_string();
            tracing::debug!("Updated title");
        }
        if let Some(content_type) = args.get("content_type").and_then(|v| v.as_str()) {
            self.content_type = content_type.to_string();
            tracing::debug!("Updated content_type");
        }
        if let Some(data) = args.get("data") {
            self.data = data.clone();
            tracing::debug!("Updated data");
        }
        tracing::info!(id = %self.id, "Updated ContentEntry");
        Ok(())
    }
}
