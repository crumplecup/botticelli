//! Health status types.

use elicitation::{Prompt, Select};
use serde::{Deserialize, Serialize};

/// Health status of a backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, elicitation::Elicit)]
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
