use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize OpenTelemetry with OTLP export
///
/// This bridges our existing `tracing` instrumentation to OpenTelemetry,
/// allowing export to backends like Jaeger, SigNoz, or Grafana Cloud.
///
/// # Arguments
///
/// * `service_name` - Name of the service for telemetry attribution
/// * `otlp_endpoint` - OTLP gRPC endpoint (e.g., "http://localhost:4317")
/// * `export_console` - Whether to also log to console (useful for development)
///
/// # Returns
///
/// Result indicating success or failure of initialization.
///
/// # Note
///
/// This is a placeholder implementation. OpenTelemetry v0.31+ requires
/// significant API changes. For now, this initializes basic tracing.
/// Full OpenTelemetry integration tracked in OPENTELEMETRY_INTEGRATION_PLAN.md
pub fn init_telemetry(
    service_name: &str,
    _otlp_endpoint: &str,
    export_console: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        service_name = service_name,
        "Initializing telemetry (console-only placeholder)"
    );

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,botticelli=debug"));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if export_console {
        subscriber.with(tracing_subscriber::fmt::layer()).init();
    } else {
        subscriber.init();
    }

    info!("Telemetry initialized successfully (console mode)");

    Ok(())
}

/// Initialize console-only telemetry (no OTLP export)
///
/// This is useful for testing or when running without an observability backend.
pub fn init_console_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,botticelli=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
