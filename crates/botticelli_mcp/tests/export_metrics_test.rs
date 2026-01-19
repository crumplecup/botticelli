//! Tests for export_metrics tool.

mod helpers;

use botticelli_mcp::{BotticelliServer, ExportMetricsParams, MetricsFormat, PrometheusMetrics};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

#[tokio::test]
async fn test_export_metrics_without_collector() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing export_metrics without collector");

    let server = BotticelliServer::builder().build()?;
    let params = ExportMetricsParams::new(MetricsFormat::Prometheus);

    let result = server.export_metrics(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Call completed");

    // Should fail when metrics collector not configured
    assert!(
        result.is_err(),
        "Should fail when metrics collector not configured"
    );

    tracing::info!("Without collector test passed");
    Ok(())
}

#[tokio::test]
async fn test_export_metrics_prometheus_format() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing export_metrics with Prometheus format");

    let metrics = Arc::new(PrometheusMetrics::new());
    let server = BotticelliServer::builder().metrics(Some(metrics)).build()?;
    tracing::debug!("Created server with metrics collector");

    let params = ExportMetricsParams::new(MetricsFormat::Prometheus);

    let result = server.export_metrics(Parameters(params)).await?;
    tracing::debug!(format = %result.0.format(), has_metrics = result.0.metrics().is_some(), "Received metrics");

    assert_eq!(result.0.format(), "prometheus");
    assert!(result.0.metrics().is_some());
    // Summary fields should be None
    assert!(result.0.total_executions().is_none());
    assert!(result.0.total_tokens().is_none());

    tracing::info!("Prometheus format test passed");
    Ok(())
}

#[tokio::test]
async fn test_export_metrics_summary_format() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing export_metrics with Summary format");

    let metrics = Arc::new(PrometheusMetrics::new());
    let server = BotticelliServer::builder().metrics(Some(metrics)).build()?;

    let params = ExportMetricsParams::new(MetricsFormat::Summary);

    let result = server.export_metrics(Parameters(params)).await?;
    tracing::debug!(
        format = %result.0.format(),
        executions = ?result.0.total_executions(),
        tokens = ?result.0.total_tokens(),
        "Received summary"
    );

    assert_eq!(result.0.format(), "summary");
    assert_eq!(*result.0.total_executions(), Some(0));
    assert_eq!(*result.0.total_tokens(), Some(0));
    assert_eq!(*result.0.total_cost_usd(), Some(0.0));
    assert_eq!(*result.0.avg_duration_ms(), Some(0));
    // Metrics field should be None
    assert!(result.0.metrics().is_none());

    tracing::info!("Summary format test passed");
    Ok(())
}

#[tokio::test]
async fn test_export_metrics_params_default_format() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExportMetricsParams default format");

    use serde_json::json;

    let json_value = json!({});

    let params: ExportMetricsParams = serde_json::from_value(json_value)?;
    tracing::debug!(format = ?params.format(), "Deserialized with default");

    assert_eq!(params.format(), &MetricsFormat::Prometheus);

    tracing::info!("Default format test passed");
    Ok(())
}

#[tokio::test]
async fn test_export_metrics_params_custom_format() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExportMetricsParams custom format");

    use serde_json::json;

    let json_value = json!({
        "format": "summary"
    });

    let params: ExportMetricsParams = serde_json::from_value(json_value)?;
    tracing::debug!(format = ?params.format(), "Deserialized with custom format");

    assert_eq!(params.format(), &MetricsFormat::Summary);

    tracing::info!("Custom format test passed");
    Ok(())
}
