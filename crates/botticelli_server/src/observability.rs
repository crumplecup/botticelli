///! OpenTelemetry-based observability infrastructure for Botticelli bot server.
///!
///! Provides metrics, traces, and structured logging via OpenTelemetry protocol (OTLP).

use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::MeterProvider,
    trace::{self, TracerProvider},
    Resource,
};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

/// Configuration for observability infrastructure.
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// OTLP endpoint for traces and metrics (e.g., "http://localhost:4317")
    pub otlp_endpoint: String,
    /// Log level filter (e.g., "info", "debug")
    pub log_level: String,
    /// Enable JSON-formatted logs for production
    pub json_logs: bool,
    /// Service name identifier
    pub service_name: String,
    /// Service version
    pub service_version: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: "http://localhost:4317".to_string(),
            log_level: "info".to_string(),
            json_logs: false,
            service_name: "botticelli-bot-server".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Initialize OpenTelemetry observability stack.
///
/// Sets up:
/// - Distributed tracing via OTLP
/// - Metrics export via OTLP
/// - Structured logging with trace correlation
pub fn init_observability(
    config: &ObservabilityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create resource (identifies this service)
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
    ]);

    // Initialize tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint),
        )
        .with_trace_config(trace::Config::default().with_resource(resource.clone()))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Initialize metrics
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint),
        )
        .with_resource(resource)
        .build()?;

    global::set_meter_provider(meter_provider);

    // Configure tracing subscriber
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;

    let fmt_layer = if config.json_logs {
        tracing_subscriber::fmt::layer().json().boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    Ok(())
}

/// Shutdown observability providers gracefully.
///
/// Flushes any pending traces and metrics before shutdown.
pub fn shutdown_observability() {
    global::shutdown_tracer_provider();
}
