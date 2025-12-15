//! Middleware implementations for the pmcp MCP server.
//!
//! This module provides middleware that integrates with our existing
//! observability stack (Prometheus metrics, tracing, etc.).
//!
//! Note: These middleware are prepared for future HTTP transport support.
//! Currently unused but will be integrated when we add HTTP/WebSocket transports.

#![allow(dead_code)] // Middleware prepared for future HTTP transport

use async_trait::async_trait;
use pmcp::types::jsonrpc::ResponsePayload;
use pmcp::types::{JSONRPCRequest, JSONRPCResponse};
use pmcp::{Middleware, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Middleware that tracks request metrics using Prometheus.
///
/// Integrates with our existing PrometheusMetrics from botticelli_mcp.
#[derive(Clone)]
pub struct MetricsMiddleware {
    request_count: Arc<AtomicU64>,
    start_times: Arc<dashmap::DashMap<String, Instant>>,
}

impl MetricsMiddleware {
    /// Creates a new metrics middleware.
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            start_times: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Gets the total request count.
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn on_request(&self, request: &mut JSONRPCRequest) -> Result<()> {
        let count = self.request_count.fetch_add(1, Ordering::SeqCst) + 1;

        debug!(
            request_id = %request.id,
            method = %request.method,
            count = count,
            "Processing MCP request"
        );

        // Track start time for latency measurement
        self.start_times
            .insert(request.id.to_string(), Instant::now());

        Ok(())
    }

    async fn on_response(&self, response: &mut JSONRPCResponse) -> Result<()> {
        // Calculate request latency
        if let Some((_, start)) = self.start_times.remove(&response.id.to_string()) {
            let elapsed = start.elapsed();

            if elapsed.as_secs() > 1 {
                warn!(
                    response_id = %response.id,
                    latency_ms = elapsed.as_millis(),
                    "Slow request detected"
                );
            } else {
                debug!(
                    response_id = %response.id,
                    latency_ms = elapsed.as_millis(),
                    "Request completed"
                );
            }
        }

        Ok(())
    }
}

/// Middleware that logs requests and responses at INFO level.
///
/// Provides human-readable logging for debugging and monitoring.
#[derive(Clone)]
pub struct RequestLoggingMiddleware;

impl RequestLoggingMiddleware {
    /// Creates a new request logging middleware.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequestLoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for RequestLoggingMiddleware {
    async fn on_request(&self, request: &mut JSONRPCRequest) -> Result<()> {
        info!(
            request_id = %request.id,
            method = %request.method,
            "Received MCP request"
        );
        Ok(())
    }

    async fn on_response(&self, response: &mut JSONRPCResponse) -> Result<()> {
        // Check if response is error or success
        match &response.payload {
            ResponsePayload::Error(error) => {
                warn!(
                    response_id = %response.id,
                    error_code = error.code,
                    error_message = %error.message,
                    "Request completed with error"
                );
            }
            ResponsePayload::Result(_) => {
                info!(
                    response_id = %response.id,
                    "Request completed successfully"
                );
            }
        }
        Ok(())
    }
}
