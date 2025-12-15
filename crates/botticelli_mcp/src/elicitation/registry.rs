//! Registry operations trait for elicitation data types.

use botticelli_error::McpResult;
use serde_json::Value;

/// Registry operations for elicitation data types.
///
/// Implementors can be stored, retrieved, updated, and deleted from a registry.
pub trait RegistryOperations: Sized + Send + Sync {
    /// The key type for registry lookups.
    type Key: Clone + Send + Sync;

    /// Get the registry key for this item.
    fn registry_key(&self) -> Self::Key;

    /// Create a new instance from JSON arguments.
    ///
    /// # Errors
    ///
    /// Returns error if JSON cannot be deserialized to this type.
    fn from_json_args(args: Value) -> McpResult<Self>;

    /// Convert to JSON for storage/retrieval.
    fn to_json(&self) -> McpResult<Value>;

    /// Update this instance from JSON arguments.
    ///
    /// # Errors
    ///
    /// Returns error if JSON fields are invalid for this type.
    fn update_from_json(&mut self, args: Value) -> McpResult<()>;
}
