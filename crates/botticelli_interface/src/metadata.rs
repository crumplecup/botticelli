//! Model metadata trait.

use crate::BotticelliDriver;

/// Trait for querying model metadata and capabilities.
pub trait Metadata: BotticelliDriver {
    /// Metadata type containing model information.
    type ModelMetadata: Send + Sync;
    
    /// Get comprehensive metadata about this model.
    fn metadata(&self) -> &Self::ModelMetadata;
}
