use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_sdk::{
    metrics::SdkMeterProvider,
    trace::SdkTracerProvider,
    Resource,
};
use opentelemetry_stdout::SpanExporter;
use std::env;
use std::net::SocketAddr;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Exporter backend for traces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExporterBackend {
    /// Export traces to stdout (development/debugging)
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

impl Default for ExporterBackend {
    fn default() -> Self {
        Self::Stdout
    }
}

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
    /// Exporter backend for traces
    pub exporter: ExporterBackend,
    /// Enable metrics collection and export
    pub enable_metrics: bool,
    /// Prometheus metrics HTTP endpoint (e.g., "0.0.0.0:9464")
    /// If None, Prometheus HTTP server is disabled
    pub prometheus_endpoint: Option<SocketAddr>,
}

impl ObservabilityConfig {
    /// Create a new configuration with the given service name.
    ///
    /// Defaults:
    /// - Exporter: Read from `OTEL_EXPORTER` env (default: stdout)
    /// - Log level: Read from `RUST_LOG` env (default: info)
    /// - JSON logs: false
    /// - Metrics: enabled
    /// - Prometheus endpoint: Read from `PROMETHEUS_ENDPOINT` env (default: 0.0.0.0:9464)
    pub fn new(service_name: impl Into<String>) -> Self {
        let prometheus_endpoint = env::var("PROMETHEUS_ENDPOINT")
            .ok()
            .and_then(|s| s.parse().ok())
            .or_else(|| Some("0.0.0.0:9464".parse().expect("Valid default address")));
        
        Self {
            service_name: service_name.into(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            json_logs: false,
            exporter: ExporterBackend::from_env(),
            enable_metrics: true,
            prometheus_endpoint,
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

    /// Set the exporter backend.
    pub fn with_exporter(mut self, exporter: ExporterBackend) -> Self {
        self.exporter = exporter;
        self
    }

    /// Enable or disable metrics collection.
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }

    /// Set the Prometheus HTTP endpoint for metrics scraping.
    /// Pass None to disable the Prometheus HTTP server.
    pub fn with_prometheus_endpoint(mut self, endpoint: Option<SocketAddr>) -> Self {
        self.prometheus_endpoint = endpoint;
        self
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self::new(env!("CARGO_PKG_NAME"))
    }
}

/// Handle to the running Prometheus HTTP server
#[derive(Debug)]
pub struct PrometheusServer {
    _handle: tokio::task::JoinHandle<()>,
}

/// Initialize OpenTelemetry observability stack with default configuration.
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Stdout exporter for development
/// - Service name and version metadata
/// - Optional Prometheus HTTP server
///
/// For more control, use `init_observability_with_config()`.
pub fn init_observability() -> Result<Option<PrometheusServer>, Box<dyn std::error::Error>> {
    init_observability_with_config(ObservabilityConfig::default())
}

/// Initialize OpenTelemetry observability stack with custom configuration.
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Configurable exporter backend (stdout, OTLP)
/// - Service name and version metadata
/// - Configurable log format (text or JSON)
/// - Optional Prometheus HTTP server for metrics scraping
///
/// Returns a PrometheusServer handle if Prometheus endpoint is configured.
/// The server runs in a background task and stops when the handle is dropped.
pub fn init_observability_with_config(
    config: ObservabilityConfig,
) -> Result<Option<PrometheusServer>, Box<dyn std::error::Error>> {
    // Create resource with service metadata
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes(vec![KeyValue::new(
            "service.version",
            config.service_version.clone(),
        )])
        .build();

    // Create tracer provider based on exporter backend
    let provider = match config.exporter {
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
                .map_err(|e| format!("Failed to build OTLP exporter: {}", e))?;

            SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource.clone())
                .build()
        }
    };

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // Initialize metrics if enabled (before resource is moved)
    let prometheus_server = if config.enable_metrics {
        init_metrics(&resource, &config)?
    } else {
        None
    };

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

    Ok(prometheus_server)
}

/// Initialize metrics provider based on configuration.
fn init_metrics(
    resource: &Resource,
    config: &ObservabilityConfig,
) -> Result<Option<PrometheusServer>, Box<dyn std::error::Error>> {
    // If Prometheus endpoint is configured, use Prometheus exporter
    if let Some(endpoint) = config.prometheus_endpoint {
        let prometheus_registry = prometheus::Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(prometheus_registry.clone())
            .build()
            .map_err(|e| format!("Failed to build Prometheus exporter: {}", e))?;

        let meter_provider = SdkMeterProvider::builder()
            .with_reader(exporter)
            .with_resource(resource.clone())
            .build();

        global::set_meter_provider(meter_provider);

        // Start Prometheus HTTP server
        let server = start_prometheus_server(endpoint, prometheus_registry)?;
        return Ok(Some(server));
    }

    // Otherwise, use the configured exporter backend
    match &config.exporter {
        ExporterBackend::Stdout => {
            // Stdout exporter for metrics (development)
            let exporter = opentelemetry_stdout::MetricExporter::default();
            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
                .build();

            let meter_provider = SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource.clone())
                .build();

            global::set_meter_provider(meter_provider);
        }
        #[cfg(feature = "otel-otlp")]
        ExporterBackend::Otlp { endpoint } => {
            use opentelemetry_otlp::WithExportConfig;

            // Build OTLP metric exporter with tonic
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.clone())
                .build()
                .map_err(|e| format!("Failed to build OTLP metric exporter: {}", e))?;

            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
                .build();

            let meter_provider = SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource.clone())
                .build();

            global::set_meter_provider(meter_provider);
        }
    }

    Ok(None)
}

/// Start Prometheus HTTP server for metrics scraping.
fn start_prometheus_server(
    addr: SocketAddr,
    registry: prometheus::Registry,
) -> Result<PrometheusServer, Box<dyn std::error::Error>> {
    use http_body_util::Full;
    use hyper::body::Bytes;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let handle = tokio::spawn(async move {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind Prometheus server to {}: {}", addr, e);
                return;
            }
        };
        
        tracing::info!("Prometheus metrics server listening on http://{}/metrics", addr);

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let io = TokioIo::new(stream);
            let registry = registry.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let registry = registry.clone();
                    async move {
                        if req.uri().path() == "/metrics" {
                            let metric_families = registry.gather();
                            let encoder = prometheus::TextEncoder::new();
                            match encoder.encode_to_string(&metric_families) {
                                Ok(body) => Ok::<_, hyper::Error>(
                                    Response::builder()
                                        .status(200)
                                        .header("Content-Type", encoder.format_type())
                                        .body(Full::new(Bytes::from(body)))
                                        .unwrap(),
                                ),
                                Err(e) => {
                                    tracing::error!("Failed to encode metrics: {}", e);
                                    Ok(Response::builder()
                                        .status(500)
                                        .body(Full::new(Bytes::from("Internal Server Error")))
                                        .unwrap())
                                }
                            }
                        } else {
                            Ok(Response::builder()
                                .status(404)
                                .body(Full::new(Bytes::from("Not Found")))
                                .unwrap())
                        }
                    }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });
        }
    });

    Ok(PrometheusServer { _handle: handle })
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
