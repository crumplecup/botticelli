//! Batch generation capability.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for backends that support batch processing.
#[async_trait]
pub trait BatchGeneration: BotticelliDriver {
    /// Batch job identifier type.
    type BatchId: Send + Sync;

    /// Submit a batch of requests for processing.
    async fn submit_batch(&self, requests: &[Self::Request]) -> Result<Self::BatchId, Self::Error>;

    /// Check the status of a batch job.
    async fn check_batch_status(&self, batch_id: &Self::BatchId) -> Result<String, Self::Error>;

    /// Retrieve results from a completed batch.
    async fn get_batch_results(
        &self,
        batch_id: &Self::BatchId,
    ) -> Result<Vec<Self::Response>, Self::Error>;

    /// Maximum number of requests per batch.
    fn max_batch_size(&self) -> usize {
        10
    }
}
