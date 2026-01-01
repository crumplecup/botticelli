//! Embeddings generation capability.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for models that can generate embeddings.
#[async_trait]
pub trait Embeddings: BotticelliDriver {
    /// Generate embeddings for one or more text inputs.
    ///
    /// Returns a vector of embedding vectors, one per input.
    async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, Self::Error>;

    /// Dimensionality of the embedding vectors.
    fn embedding_dimensions(&self) -> usize;
}
