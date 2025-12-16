//! Generic registry operation tools for types implementing RegistryOperations.

use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use botticelli_interface::RegistryOperations;
use pmcp::{Content, ToolInfo};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

/// Generic registry for managing items implementing RegistryOperations.
#[derive(Debug, Clone)]
pub struct GenericRegistry<T: RegistryOperations>
where
    T::Key: std::fmt::Debug,
{
    items: Arc<RwLock<HashMap<T::Key, T>>>,
}

impl<T: RegistryOperations> GenericRegistry<T>
where
    T::Key: Eq + Hash + std::fmt::Debug,
{
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add or update an item in the registry.
    pub fn upsert(&self, item: T) -> McpClientResult<T::Key> {
        let key = item.registry_key();
        let mut items = self.items.write().map_err(|_| {
            McpClientError::new(McpClientErrorKind::RegistryLockPoisoned)
        })?;
        items.insert(key.clone(), item);
        tracing::info!("Upserted item in registry");
        Ok(key)
    }

    /// Get an item by key.
    pub fn get(&self, key: &T::Key) -> McpClientResult<Option<T>>
    where
        T: Clone,
    {
        let items = self.items.read().map_err(|_| {
            McpClientError::new(McpClientErrorKind::RegistryLockPoisoned)
        })?;
        Ok(items.get(key).cloned())
    }

    /// Update an existing item.
    pub fn update(&self, key: &T::Key, item: T) -> McpClientResult<bool> {
        let mut items = self.items.write().map_err(|_| {
            McpClientError::new(McpClientErrorKind::RegistryLockPoisoned)
        })?;
        if items.contains_key(key) {
            items.insert(key.clone(), item);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Remove an item by key.
    pub fn remove(&self, key: &T::Key) -> McpClientResult<Option<T>> {
        let mut items = self.items.write().map_err(|_| {
            McpClientError::new(McpClientErrorKind::RegistryLockPoisoned)
        })?;
        Ok(items.remove(key))
    }

    /// List all keys in the registry.
    pub fn list_keys(&self) -> McpClientResult<Vec<T::Key>>
    where
        T::Key: Clone,
    {
        let items = self.items.read().map_err(|_| {
            McpClientError::new(McpClientErrorKind::RegistryLockPoisoned)
        })?;
        Ok(items.keys().cloned().collect())
    }
}

impl<T: RegistryOperations> Default for GenericRegistry<T>
where
    T::Key: Eq + Hash + std::fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Tool for creating/updating registry items.
#[derive(Debug, Clone)]
pub struct UpsertRegistryItemTool<T: RegistryOperations>
where
    T::Key: std::fmt::Debug,
{
    registry: GenericRegistry<T>,
    tool_name: String,
    description: String,
}

impl<T: RegistryOperations> UpsertRegistryItemTool<T>
where
    T::Key: Eq + Hash + std::fmt::Debug,
{
    /// Create tool with registry.
    pub fn new(registry: GenericRegistry<T>, tool_name: String, description: String) -> Self {
        Self {
            registry,
            tool_name,
            description,
        }
    }
}

#[async_trait]
impl<T: RegistryOperations + Clone + Send + Sync + 'static> ToolHandler
    for UpsertRegistryItemTool<T>
where
    T::Key: Eq + Hash + Clone + std::fmt::Display + std::fmt::Debug + Send + Sync,
{
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            &self.tool_name,
            Some(self.description.clone()),
            json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "object",
                        "description": "Item data to store/update"
                    }
                },
                "required": ["data"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, arguments: Value) -> McpClientResult<Vec<Content>> {
        let data = arguments
            .get("data")
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'data' field".to_string(),
                ))
            })?
            .clone();

        let item = T::from_json_args(data).map_err(|e| {
            McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                "Failed to create item: {}",
                e
            )))
        })?;

        let key = self.registry.upsert(item)?;

        Ok(vec![Content::Text {
            text: format!("Item stored with key: {}", key),
        }])
    }
}

/// Tool for retrieving registry items.
#[derive(Debug, Clone)]
pub struct GetRegistryItemTool<T: RegistryOperations>
where
    T::Key: std::fmt::Debug,
{
    registry: GenericRegistry<T>,
    tool_name: String,
    description: String,
}

impl<T: RegistryOperations> GetRegistryItemTool<T>
where
    T::Key: Eq + Hash + std::fmt::Debug,
{
    /// Create tool with registry.
    pub fn new(registry: GenericRegistry<T>, tool_name: String, description: String) -> Self {
        Self {
            registry,
            tool_name,
            description,
        }
    }
}

#[async_trait]
impl<T: RegistryOperations + Clone + Send + Sync + 'static> ToolHandler for GetRegistryItemTool<T>
where
    T::Key: Eq + Hash + Clone + std::fmt::Debug + Send + Sync + for<'de> serde::Deserialize<'de>,
{
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            &self.tool_name,
            Some(self.description.clone()),
            json!({
                "type": "object",
                "properties": {
                    "key": {
                        "description": "Key to retrieve"
                    }
                },
                "required": ["key"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, arguments: Value) -> McpClientResult<Vec<Content>> {
        let key: T::Key = serde_json::from_value(
            arguments
                .get("key")
                .ok_or_else(|| {
                    McpClientError::new(McpClientErrorKind::InvalidToolCall(
                        "Missing 'key' field".to_string(),
                    ))
                })?
                .clone(),
        )
        .map_err(|e| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                "Invalid key format: {}",
                e
            )))
        })?;

        let item = self.registry.get(&key)?.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ToolNotFound(
                "Item not found".to_string(),
            ))
        })?;

        let json = item.to_json().map_err(|e| {
            McpClientError::new(McpClientErrorKind::SerializationError(format!(
                "Failed to serialize item: {}",
                e
            )))
        })?;

        Ok(vec![Content::Text {
            text: json.to_string(),
        }])
    }
}

/// Tool for listing registry keys.
#[derive(Debug, Clone)]
pub struct ListRegistryKeysTool<T: RegistryOperations>
where
    T::Key: std::fmt::Debug,
{
    registry: GenericRegistry<T>,
    tool_name: String,
    description: String,
}

impl<T: RegistryOperations> ListRegistryKeysTool<T>
where
    T::Key: Eq + Hash + std::fmt::Debug,
{
    /// Create tool with registry.
    pub fn new(registry: GenericRegistry<T>, tool_name: String, description: String) -> Self {
        Self {
            registry,
            tool_name,
            description,
        }
    }
}

#[async_trait]
impl<T: RegistryOperations + Clone + Send + Sync + 'static> ToolHandler for ListRegistryKeysTool<T>
where
    T::Key: Eq + Hash + Clone + std::fmt::Display + std::fmt::Debug + Send + Sync,
{
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            &self.tool_name,
            Some(self.description.clone()),
            json!({
                "type": "object",
                "properties": {},
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, _arguments: Value) -> McpClientResult<Vec<Content>> {
        let keys = self.registry.list_keys()?;
        let key_strings: Vec<String> = keys.iter().map(|k| k.to_string()).collect();

        Ok(vec![Content::Text {
            text: format!("Registry keys: [{}]", key_strings.join(", ")),
        }])
    }
}
