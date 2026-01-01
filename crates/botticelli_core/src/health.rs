//! Health status types.

use serde::{Deserialize, Serialize};

/// Health status of a backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is fully operational
    Healthy,
    /// System is operational but with reduced performance
    Degraded {
        /// Description of the degradation
        message: String,
    },
    /// System is not operational
    Unhealthy {
        /// Description of the problem
        message: String,
    },
}
