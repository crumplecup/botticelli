//! Health check trait.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for backends that support health checks.
#[async_trait]
pub trait Health: BotticelliDriver {
    /// Health status type for this backend.
    type HealthStatus: Send + Sync;
    
    /// Check if the backend is available and functioning.
    async fn health(&self) -> Result<Self::HealthStatus, Self::Error>;
}
