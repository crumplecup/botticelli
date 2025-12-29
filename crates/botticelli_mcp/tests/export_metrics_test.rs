//! Tests for export_metrics tool.

use botticelli_mcp::{
    BotticelliServer, ExportMetricsParams, MetricsFormat, PrometheusMetrics,
};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

#[tokio::test]
async fn test_export_metrics_without_collector() {
    let server = BotticelliServer::builder().build();
    let params = ExportMetricsParams {
        format: MetricsFormat::Prometheus,
    };

    let result = server.export_metrics(Parameters(params)).await;

    // Should fail when metrics collector not configured
    assert!(
        result.is_err(),
        "Should fail when metrics collector not configured"
    );
}

#[tokio::test]
async fn test_export_metrics_prometheus_format() {
    let metrics = Arc::new(PrometheusMetrics::new());
    let server = BotticelliServer::builder().metrics(metrics).build();

    let params = ExportMetricsParams {
        format: MetricsFormat::Prometheus,
    };

    let result = server
        .export_metrics(Parameters(params))
        .await
        .expect("Should succeed");

    assert_eq!(result.0.format, "prometheus");
    assert!(result.0.metrics.is_some());
    // Summary fields should be None
    assert!(result.0.total_executions.is_none());
    assert!(result.0.total_tokens.is_none());
}

#[tokio::test]
async fn test_export_metrics_summary_format() {
    let metrics = Arc::new(PrometheusMetrics::new());
    let server = BotticelliServer::builder().metrics(metrics).build();

    let params = ExportMetricsParams {
        format: MetricsFormat::Summary,
    };

    let result = server
        .export_metrics(Parameters(params))
        .await
        .expect("Should succeed");

    assert_eq!(result.0.format, "summary");
    assert_eq!(result.0.total_executions, Some(0));
    assert_eq!(result.0.total_tokens, Some(0));
    assert_eq!(result.0.total_cost_usd, Some(0.0));
    assert_eq!(result.0.avg_duration_ms, Some(0));
    // Metrics field should be None
    assert!(result.0.metrics.is_none());
}

#[tokio::test]
async fn test_export_metrics_params_default_format() {
    use serde_json::json;

    let json_value = json!({});

    let params: ExportMetricsParams =
        serde_json::from_value(json_value).expect("Should deserialize with default format");

    assert_eq!(params.format, MetricsFormat::Prometheus);
}

#[tokio::test]
async fn test_export_metrics_params_custom_format() {
    use serde_json::json;

    let json_value = json!({
        "format": "summary"
    });

    let params: ExportMetricsParams =
        serde_json::from_value(json_value).expect("Should deserialize with custom format");

    assert_eq!(params.format, MetricsFormat::Summary);
}
