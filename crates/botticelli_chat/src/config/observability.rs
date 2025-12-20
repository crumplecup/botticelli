use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct ObservabilityConfig {
    /// Rust log level.
    #[serde(default = "default_rust_log")]
    rust_log: String,

    /// OpenTelemetry exporter type.
    #[serde(default = "default_otel_exporter")]
    otel_exporter: String,

    /// OpenTelemetry endpoint.
    #[serde(default = "default_otel_endpoint")]
    otel_endpoint: String,
}

fn default_rust_log() -> String {
    "info".to_string()
}

fn default_otel_exporter() -> String {
    "stdout".to_string()
}

fn default_otel_endpoint() -> String {
    "http://localhost:4318".to_string()
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            rust_log: default_rust_log(),
            otel_exporter: default_otel_exporter(),
            otel_endpoint: default_otel_endpoint(),
        }
    }
}
