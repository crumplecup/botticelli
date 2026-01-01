//! RegistryOperations implementations for database types.

use botticelli_error::{McpError, McpErrorKind, McpResult};
use botticelli_interface::RegistryOperations;
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
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
        }
    }
}

impl RegistryOperations for ActorRow {
    type Error = botticelli_error::BotticelliError;
    type Key = Uuid;

    fn registry_key(&self) -> Self::Key {
        self.id
    }

    fn from_json_args(args: Value) -> McpResult<Self> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
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

        Ok(Self {
            id,
            name,
            description,
        })
    }

    fn to_json(&self) -> McpResult<Value> {
        serde_json::to_value(self)
            .map_err(|e| McpError::new(McpErrorKind::ExecutionError(e.to_string())))
    }

    fn update_from_json(&mut self, args: Value) -> McpResult<()> {
        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            self.name = name.to_string();
        }
        if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
            self.description = Some(description.to_string());
        }
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
    pub fn new(title: String, content_type: String, data: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            content_type,
            data,
        }
    }
}

impl RegistryOperations for ContentEntry {
    type Error = botticelli_error::BotticelliError;
    type Key = Uuid;

    fn registry_key(&self) -> Self::Key {
        self.id
    }

    fn from_json_args(args: Value) -> McpResult<Self> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
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

        Ok(Self {
            id,
            title,
            content_type,
            data,
        })
    }

    fn to_json(&self) -> McpResult<Value> {
        serde_json::to_value(self)
            .map_err(|e| McpError::new(McpErrorKind::ExecutionError(e.to_string())))
    }

    fn update_from_json(&mut self, args: Value) -> McpResult<()> {
        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            self.title = title.to_string();
        }
        if let Some(content_type) = args.get("content_type").and_then(|v| v.as_str()) {
            self.content_type = content_type.to_string();
        }
        if let Some(data) = args.get("data") {
            self.data = data.clone();
        }
        Ok(())
    }
}
