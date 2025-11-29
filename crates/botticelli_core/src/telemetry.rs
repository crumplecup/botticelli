//! OpenTelemetry integration for distributed tracing and observability.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_stdout::SpanExporter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize OpenTelemetry with stdout exporter for development.
///
/// This sets up tracing with OpenTelemetry integration, exporting spans to stdout.
/// The tracing subscriber will respect RUST_LOG environment variable.
///
/// # Errors
///
/// Returns error if subscriber initialization fails.
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Create stdout exporter for development
    let exporter = SpanExporter::default();

    // Build tracer provider with resource attributes
    let provider = TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(Resource::default())
        .build();

    // Get a tracer
    let tracer = provider.tracer("botticelli");

    // Create OpenTelemetry tracing layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(EnvFilter::from_default_env());

    // Create fmt layer for human-readable logs
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_filter(EnvFilter::from_default_env());

    // Initialize subscriber with both layers
    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(fmt_layer)
        .try_init()?;

    Ok(())
}

/// Shutdown OpenTelemetry and flush pending spans.
///
/// Call this before application exit to ensure all spans are exported.
pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}
