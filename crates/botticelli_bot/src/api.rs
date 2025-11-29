//! HTTP API for exposing bot metrics.

use crate::BotMetrics;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;
use std::sync::Arc;

/// API state containing metrics collector.
#[derive(Clone)]
pub struct ApiState {
    metrics: Arc<BotMetrics>,
}

impl ApiState {
    /// Creates new API state.
    pub fn new(metrics: Arc<BotMetrics>) -> Self {
        Self { metrics }
    }
}

/// Creates the metrics API router.
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

/// Get current metrics snapshot.
async fn get_metrics(State(state): State<ApiState>) -> impl IntoResponse {
    let snapshot = state.metrics.snapshot();
    (StatusCode::OK, Json(snapshot))
}
