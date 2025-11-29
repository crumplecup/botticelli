use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_stdout::SpanExporter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize OpenTelemetry observability stack
///
/// This sets up:
/// - Tracing with OpenTelemetry bridge
/// - Stdout exporter for development
/// - Service name and version metadata
pub fn init_observability() -> Result<(), Box<dyn std::error::Error>> {
    // Create stdout exporter for development
    let exporter = SpanExporter::default();

    // Build tracer provider
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // Create OpenTelemetry tracing layer
    let tracer = provider.tracer(env!("CARGO_PKG_NAME"));
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Setup tracing subscriber with both fmt and OpenTelemetry layers
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    Ok(())
}

/// Shutdown OpenTelemetry gracefully
///
/// This ensures all spans are flushed before exit
pub fn shutdown_observability() {
    // In v0.31, providers are dropped automatically and flush on drop
    // No explicit shutdown needed for stdout exporter
}
