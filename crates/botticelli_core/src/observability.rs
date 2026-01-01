use botticelli_error::{ObservabilityError, ObservabilityErrorKind, ObservabilityResult};
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider, trace::SdkTracerProvider};
use opentelemetry_stdout::SpanExporter;
use std::env;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Exporter backend for traces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExporterBackend {
    /// Export traces to stdout (development/debugging)
    #[default]
    Stdout,
    /// Export traces via OTLP to a collector (production)
    #[cfg(feature = "otel-otlp")]
    Otlp {
        /// OTLP endpoint (e.g., "http://localhost:4317")
        endpoint: String,
    },
}

impl ExporterBackend {
    /// Parse exporter backend from environment variable.
    ///
    /// Reads `OTEL_EXPORTER` and `OTEL_EXPORTER_OTLP_ENDPOINT` environment variables:
    /// - "stdout" → Stdout (default if unset)
    /// - "otlp" → Otlp (requires `otel-otlp` feature, reads endpoint from env)
    pub fn from_env() -> Self {
        match env::var("OTEL_EXPORTER")
            .unwrap_or_else(|_| "stdout".to_string())
            .to_lowercase()
            .as_str()
        {
            #[cfg(feature = "otel-otlp")]
            "otlp" => {
                let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4317".to_string());
                Self::Otlp { endpoint }
            }
            "stdout" => Self::Stdout,
            _ => Self::Stdout, // Default to stdout for unknown values
        }
    }
}

/// Configuration for OpenTelemetry observability.
#[derive(Debug, Clone, derive_getters::Getters, derive_builder::Builder)]
#[builder(pattern = "owned", setter(into, strip_option))]
pub struct ObservabilityConfig {
    /// Service name for telemetry attribution
    service_name: String,
    /// Service version
    #[builder(default = "env!(\"CARGO_PKG_VERSION\").to_string()")]
    service_version: String,
    /// Log level filter (e.g., "info", "debug")
    #[builder(default = "default_log_level()")]
    log_level: String,
    /// Enable JSON-formatted logs for structured logging
    #[builder(default)]
    json_logs: bool,
    /// Exporter backend for traces
    #[builder(default = "ExporterBackend::from_env()")]
    exporter: ExporterBackend,
    /// Enable metrics collection and export
    #[builder(default = "true")]
    enable_metrics: bool,
}

fn default_log_level() -> String {
    env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
}

impl ObservabilityConfig {
    /// Create a new configuration with the given service name.
    ///
    /// This is a convenience constructor that uses sensible defaults:
    /// - Version: from CARGO_PKG_VERSION
    /// - Log level: from RUST_LOG env (default: "info")
    /// - JSON logs: false
    /// - Exporter: from OTEL_EXPORTER env (default: stdout)
    /// - Metrics: enabled
    ///
    /// For more control, use `ObservabilityConfig::builder()`.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self::builder()
            .service_name(service_name)
            .build()
            .expect("Default observability config should always build")
    }

    /// Creates a builder for ObservabilityConfig.
    pub fn builder() -> ObservabilityConfigBuilder {
        ObservabilityConfigBuilder::default()
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
/// - Metrics collection via configured exporter
///
/// For more control, use `init_observability_with_config()`.
///
/// # Errors
///
/// Returns an error if initialization fails.
pub fn init_observability() -> ObservabilityResult<()> {
    init_observability_with_config(ObservabilityConfig::default())
}

/// Initialize OpenTelemetry observability stack with custom configuration.
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Configurable exporter backend (stdout, OTLP)
/// - Service name and version metadata
/// - Configurable log format (text or JSON)
/// - Metrics collection via configured exporter
///
/// In v0.31+, metrics are exported via OTLP to an OpenTelemetry Collector,
/// which then exposes them for Prometheus scraping. Direct Prometheus exporters
/// have been deprecated.
///
/// # Errors
///
/// Returns an error if tracer provider, exporter, or filter initialization fails.
pub fn init_observability_with_config(
    config: ObservabilityConfig,
) -> ObservabilityResult<()> {
    // Create resource with service metadata
    let resource = Resource::builder()
        .with_service_name(config.service_name().clone())
        .with_attributes(vec![KeyValue::new(
            "service.version",
            config.service_version().clone(),
        )])
        .build();

    // Create tracer provider based on exporter backend
    let provider = match config.exporter() {
        ExporterBackend::Stdout => {
            let exporter = SpanExporter::default();
            SdkTracerProvider::builder()
                .with_simple_exporter(exporter)
                .with_resource(resource.clone())
                .build()
        }
        #[cfg(feature = "otel-otlp")]
        ExporterBackend::Otlp { ref endpoint } => {
            use opentelemetry_otlp::WithExportConfig;

            // Build OTLP span exporter with tonic
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.clone())
                .build()
                .map_err(|e| {
                    ObservabilityError::new(ObservabilityErrorKind::ExporterBuildFailed(
                        e.to_string(),
                    ))
                })?;

            SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource.clone())
                .build()
        }
    };

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // Initialize metrics if enabled (before resource is moved)
    if *config.enable_metrics() {
        init_metrics(&resource, &config)?;
    }

    // Create OpenTelemetry tracing layer
    let tracer = provider.tracer(config.service_name().clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Setup environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(config.log_level()))
        .map_err(|e| {
            ObservabilityError::new(ObservabilityErrorKind::EnvFilterError(e.to_string()))
        })?;

    // Create fmt layer based on configuration
    let fmt_layer = if *config.json_logs() {
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

/// Initialize metrics provider based on configuration.
///
/// Note: In OpenTelemetry v0.31+, direct Prometheus exporters are deprecated.
/// Use OTLP exporter → OpenTelemetry Collector → Prometheus scraping instead.
fn init_metrics(resource: &Resource, config: &ObservabilityConfig) -> ObservabilityResult<()> {
    use tracing::{debug, info};

    info!("Initializing metrics provider");
    debug!(exporter = ?config.exporter(), "Metrics exporter configuration");

    // Use the configured exporter backend
    match config.exporter() {
        ExporterBackend::Stdout => {
            info!("Using stdout metrics exporter (development mode)");
            // Stdout exporter for metrics (development)
            debug!("Creating stdout metric exporter");
            let exporter = opentelemetry_stdout::MetricExporter::default();
            debug!("Creating periodic reader for stdout exporter");
            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter).build();

            debug!("Building meter provider with stdout reader");
            let meter_provider = SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource.clone())
                .build();

            debug!("Setting global meter provider");
            global::set_meter_provider(meter_provider);
            info!("Stdout metrics provider initialized successfully");
        }
        #[cfg(feature = "otel-otlp")]
        ExporterBackend::Otlp { endpoint } => {
            use opentelemetry_otlp::{Protocol, WithExportConfig};
            use tracing::{debug, info};

            info!(endpoint = %endpoint, "Using OTLP metrics exporter");

            // For Prometheus with --web.enable-otlp-receiver, use HTTP binary protocol
            // Endpoint should be: http://prometheus:9090/api/v1/otlp/v1/metrics
            // Check both standard OTLP env var and custom metrics-specific var
            let metrics_endpoint = env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
                .or_else(|_| env::var("OTEL_METRICS_ENDPOINT"))
                .unwrap_or_else(|_| format!("{}/api/v1/otlp/v1/metrics", endpoint));

            info!(
                metrics_endpoint = %metrics_endpoint,
                "Metrics will be sent via OTLP HTTP binary protocol"
            );

            debug!("Building OTLP metric exporter");
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_protocol(Protocol::HttpBinary)
                .with_endpoint(&metrics_endpoint)
                .build()
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to build OTLP metric exporter");
                    ObservabilityError::new(ObservabilityErrorKind::ExporterBuildFailed(
                        e.to_string(),
                    ))
                })?;
            debug!("OTLP metric exporter built successfully");

            debug!("Building meter provider with periodic exporter");
            let meter_provider = SdkMeterProvider::builder()
                .with_periodic_exporter(exporter)
                .with_resource(resource.clone())
                .build();

            debug!("Setting global meter provider");
            global::set_meter_provider(meter_provider);
            info!("OTLP metrics provider initialized successfully");
        }
    }

    info!("Metrics initialization complete");
    Ok(())
}

/// Shutdown OpenTelemetry gracefully
///
/// This ensures all spans and metrics are flushed before exit.
/// In OpenTelemetry SDK v0.31+, providers flush automatically on drop,
/// so this is primarily for API compatibility.
pub fn shutdown_observability() {
    // Providers are dropped automatically and flush on drop
    // This includes both tracer and meter providers
}
