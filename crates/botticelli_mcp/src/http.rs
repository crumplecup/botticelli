//! HTTP server for MCP using axum.
//!
//! Provides a REST API similar to OpenAI's API pattern for calling MCP tools.

use crate::{BotticelliRouter, Router};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router as AxumRouter,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info};

/// HTTP server state.
#[derive(Clone)]
struct AppState {
    router: Arc<BotticelliRouter>,
}

/// Tool call request (OpenAI-style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Tool name to call.
    pub name: String,
    /// Tool parameters.
    pub parameters: Value,
}

/// Tool call response (OpenAI-style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// Response content.
    pub content: Vec<ContentBlock>,
    /// Whether the call succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Content block in response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    /// Content type (always "text" for now).
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content.
    pub text: String,
}

/// Error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error message.
    pub message: String,
    /// Error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Server info response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Available tools.
    pub tools: Vec<String>,
    /// Available resources.
    pub resources: Vec<String>,
}

/// Create HTTP server.
#[tracing::instrument(skip(router))]
pub async fn create_server(router: BotticelliRouter, port: u16) -> Result<(), std::io::Error> {
    let state = AppState {
        router: Arc::new(router),
    };

    let app = AxumRouter::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/info", get(server_info))
        .route("/tools/call", post(call_tool))
        .route("/tools/list", get(list_tools))
        .route("/resources/list", get(list_resources))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true)),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!(addr = %addr, "Starting HTTP server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await
}

/// Health check endpoint.
#[tracing::instrument]
async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "healthy" }))
}

/// Server info endpoint.
#[tracing::instrument(skip(state))]
async fn server_info(State(state): State<AppState>) -> impl IntoResponse {
    let tools = state
        .router
        .list_tools()
        .iter()
        .map(|t| t.name.clone())
        .collect();

    let resources = state
        .router
        .list_resources()
        .iter()
        .map(|r| r.uri.clone())
        .collect();

    Json(ServerInfo {
        name: state.router.name(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tools,
        resources,
    })
}

/// List tools endpoint.
#[tracing::instrument(skip(state))]
async fn list_tools(State(state): State<AppState>) -> impl IntoResponse {
    let tools = state.router.list_tools();
    Json(json!({ "tools": tools }))
}

/// List resources endpoint.
#[tracing::instrument(skip(state))]
async fn list_resources(State(state): State<AppState>) -> impl IntoResponse {
    let resources = state.router.list_resources();
    Json(json!({ "resources": resources }))
}

/// Call tool endpoint.
#[tracing::instrument(skip(state))]
async fn call_tool(
    State(state): State<AppState>,
    Json(request): Json<ToolCallRequest>,
) -> Response {
    debug!(
        tool = %request.name,
        params = ?request.parameters,
        "Calling tool"
    );

    // Find the tool
    let tools = state.router.list_tools();
    let tool = tools.iter().find(|t| t.name == request.name);

    if tool.is_none() {
        error!(tool = %request.name, "Tool not found");
        return (
            StatusCode::NOT_FOUND,
            Json(ToolCallResponse {
                content: vec![],
                error: Some(ErrorInfo {
                    message: format!("Tool '{}' not found", request.name),
                    code: Some("tool_not_found".to_string()),
                }),
            }),
        )
            .into_response();
    }

    // Call the tool through router
    match state.router.call_tool(&request.name, request.parameters).await {
        Ok(content) => {
            debug!(content_blocks = content.len(), "Tool call succeeded");
            
            let blocks = content
                .into_iter()
                .map(|c| {
                    let text = match serde_json::to_value(&c) {
                        Ok(val) => {
                            // Extract text from content
                            if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
                                t.to_string()
                            } else {
                                format!("{}", val)
                            }
                        }
                        Err(_) => "[Content serialization error]".to_string(),
                    };
                    
                    ContentBlock {
                        content_type: "text".to_string(),
                        text,
                    }
                })
                .collect();

            (
                StatusCode::OK,
                Json(ToolCallResponse {
                    content: blocks,
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!(error = ?e, "Tool call failed");
            (
                StatusCode::BAD_REQUEST,
                Json(ToolCallResponse {
                    content: vec![],
                    error: Some(ErrorInfo {
                        message: e.to_string(),
                        code: Some("tool_error".to_string()),
                    }),
                }),
            )
                .into_response()
        }
    }
}
