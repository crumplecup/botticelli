//! Registry operations trait for data types.

use serde_json::Value;

/// Registry operations for data types.
///
/// Implementors can be stored, retrieved, updated, and deleted from a registry.
pub trait RegistryOperations: Sized + Send + Sync {
    /// The key type for registry lookups.
    type Key: Clone + Send + Sync;
    
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get the registry key for this item.
    fn registry_key(&self) -> Self::Key;

    /// Create a new instance from JSON arguments.
    ///
    /// # Errors
    ///
    /// Returns error if JSON cannot be deserialized to this type.
    fn from_json_args(args: Value) -> Result<Self, Self::Error>;

    /// Convert to JSON for storage/retrieval.
    fn to_json(&self) -> Result<Value, Self::Error>;

    /// Update this instance from JSON arguments.
    ///
    /// # Errors
    ///
    /// Returns error if JSON fields are invalid for this type.
    fn update_from_json(&mut self, args: Value) -> Result<(), Self::Error>;
}
