use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use opentelemetry_stdout::SpanExporter;
use std::env;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Configuration for OpenTelemetry observability.
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Service name for telemetry attribution
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Log level filter (e.g., "info", "debug")
    pub log_level: String,
    /// Enable JSON-formatted logs for structured logging
    pub json_logs: bool,
}

impl ObservabilityConfig {
    /// Create a new configuration with the given service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            json_logs: false,
        }
    }

    /// Set the service version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Set the log level.
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Enable JSON-formatted logs.
    pub fn with_json_logs(mut self, enabled: bool) -> Self {
        self.json_logs = enabled;
        self
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self::new(env!("CARGO_PKG_NAME"))
    }
}

/// Initialize OpenTelemetry observability stack with default configuration.
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Stdout exporter for development
/// - Service name and version metadata
///
/// For more control, use `init_observability_with_config()`.
pub fn init_observability() -> Result<(), Box<dyn std::error::Error>> {
    init_observability_with_config(ObservabilityConfig::default())
}

/// Initialize OpenTelemetry observability stack with custom configuration.
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Stdout exporter for development
/// - Service name and version metadata
/// - Configurable log format (text or JSON)
pub fn init_observability_with_config(
    config: ObservabilityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create resource with service metadata
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes(vec![KeyValue::new(
            "service.version",
            config.service_version.clone(),
        )])
        .build();

    // Create stdout exporter for development
    let exporter = SpanExporter::default();

    // Build tracer provider with resource
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(resource)
        .build();

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // Create OpenTelemetry tracing layer
    let tracer = provider.tracer(config.service_name.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Setup environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;

    // Create fmt layer based on configuration
    let fmt_layer = if config.json_logs {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_level(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .boxed()
    };

    // Initialize subscriber with all layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    Ok(())
}

/// Shutdown OpenTelemetry gracefully
///
/// This ensures all spans are flushed before exit.
/// In OpenTelemetry SDK v0.31+, providers flush automatically on drop,
/// so this is primarily for API compatibility.
pub fn shutdown_observability() {
    // Providers are dropped automatically and flush on drop
    // No explicit shutdown needed for stdout exporter
}
