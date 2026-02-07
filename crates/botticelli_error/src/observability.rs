//! Observability-related errors.
use serde::{Deserialize, Serialize};

/// Specific observability error conditions.
#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::{Prompt, Select};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, schemars::JsonSchema, elicitation::Elicit)]
pub enum ObservabilityErrorKind {
    /// Failed to initialize tracer provider
    #[display("Failed to initialize tracer provider: {}", _0)]
    TracerInitFailed(String),

    /// Failed to initialize meter provider
    #[display("Failed to initialize meter provider: {}", _0)]
    MeterInitFailed(String),

    /// Failed to build exporter
    #[display("Failed to build exporter: {}", _0)]
    ExporterBuildFailed(String),

    /// Invalid configuration
    #[display("Invalid observability configuration: {}", _0)]
    InvalidConfig(String),

    /// Environment filter error
    #[display("Failed to parse environment filter: {}", _0)]
    EnvFilterError(String),
}

/// Observability error with location tracking.
#[derive(
    Debug,
    Clone,
    derive_more::Display,
    derive_more::Error,
    derive_getters::Getters,
    elicitation::Elicit,
)]
#[display("Observability Error: {} at {}:{}", kind, file, line)]
pub struct ObservabilityError {
    kind: ObservabilityErrorKind,
    line: u32,
    file: String,
}

impl ObservabilityError {
    /// Create a new observability error with caller location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: ObservabilityErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

crate::impl_error_from_kind!(ObservabilityErrorKind => ObservabilityError);

/// Convert from string error messages.
impl From<String> for ObservabilityError {
    #[track_caller]
    fn from(msg: String) -> Self {
        Self::new(ObservabilityErrorKind::InvalidConfig(msg))
    }
}

/// Convert from &str error messages.
impl From<&str> for ObservabilityError {
    #[track_caller]
    fn from(msg: &str) -> Self {
        Self::new(ObservabilityErrorKind::InvalidConfig(msg.to_string()))
    }
}

/// Result type for observability operations.
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;
