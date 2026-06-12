//! Generates narrative TOML files needed for the smoke test via the MCP
//! `create_narrative` tool.
//!
//! `create_narrative` uses `ElicitJson`: it sends the `TomlNarrativeFile` JSON
//! schema to the connected MCP client and expects a JSON blob back.  Here the
//! "client" is this test — it pre-bakes the JSON responses that a real agent
//! would produce, exercises the full `ElicitJson → toml::to_string_pretty`
//! path, and writes the resulting TOML files to the narratives directory.

use botticelli_mcp::BotticelliServer;
use rmcp::handler::client::ClientHandler;
use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, CreateMessageRequestParams, CreateMessageResult,
    Implementation, InitializeRequestParams, ProtocolVersion, Role, SamplingMessage,
};
use rmcp::service::RequestContext;
use rmcp::{RoleClient, service};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::split;
use tokio::sync::Mutex;

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
enum GenErrorKind {
    #[display("path resolution failed: {}", _0)]
    PathResolution(String),
    #[display("MCP call failed: {}", _0)]
    McpCall(String),
    #[display("server startup failed: {}", _0)]
    ServerStartup(String),
    #[display("narrative write failed: {}", _0)]
    WriteFailed(String),
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("narrative gen error: {} at {}:{}", kind, file, line)]
struct GenError {
    kind: GenErrorKind,
    line: u32,
    file: &'static str,
}

impl GenError {
    #[track_caller]
    fn new(kind: GenErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
    #[track_caller]
    fn path(msg: impl Into<String>) -> Self {
        Self::new(GenErrorKind::PathResolution(msg.into()))
    }
    #[track_caller]
    fn mcp(msg: impl Into<String>) -> Self {
        Self::new(GenErrorKind::McpCall(msg.into()))
    }
    #[track_caller]
    fn startup(msg: impl Into<String>) -> Self {
        Self::new(GenErrorKind::ServerStartup(msg.into()))
    }
    #[track_caller]
    fn write(msg: impl Into<String>) -> Self {
        Self::new(GenErrorKind::WriteFailed(msg.into()))
    }
}

// ============================================================================
// Pre-baking client handler
// ============================================================================

/// MCP client handler that responds to `create_message` with pre-baked JSON.
///
/// `create_narrative` sends one `create_message` per tool call (ElicitJson
/// makes a single round-trip). This handler pops the next pre-baked JSON
/// blob and returns it as the sampling response.
#[derive(Debug, Clone)]
struct PrebakedHandler {
    responses: Arc<Mutex<VecDeque<String>>>,
}

impl PrebakedHandler {
    fn new(responses: impl IntoIterator<Item = serde_json::Value>) -> Self {
        let queue = responses
            .into_iter()
            .map(|v| v.to_string())
            .collect::<VecDeque<_>>();
        Self {
            responses: Arc::new(Mutex::new(queue)),
        }
    }
}

impl ClientHandler for PrebakedHandler {
    #[tracing::instrument(skip(self))]
    fn get_info(&self) -> InitializeRequestParams {
        InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("narrative-gen-test-client", "0.0.0"),
        )
        .with_protocol_version(ProtocolVersion::V_2025_06_18)
    }

    #[tracing::instrument(skip(self, _request, _context))]
    async fn create_message(
        &self,
        _request: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, rmcp::ErrorData> {
        let json = self.responses.lock().await.pop_front().ok_or_else(|| {
            rmcp::ErrorData::internal_error("no pre-baked response available", None)
        })?;
        let msg = SamplingMessage::new(Role::Assistant, json);
        Ok(CreateMessageResult::new(msg, "prebaked".to_string()))
    }
}

// ============================================================================
// Helpers
// ============================================================================

#[tracing::instrument]
async fn in_proc_server(
    handler: PrebakedHandler,
) -> Result<service::RunningService<RoleClient, PrebakedHandler>, GenError> {
    let (client_stream, server_stream) = tokio::io::duplex(65536);
    let (sr, sw) = split(server_stream);
    let (cr, cw) = split(client_stream);
    tokio::spawn(async move {
        if let Ok(svc) = service::serve_server(BotticelliServer::new(), (sr, sw)).await {
            svc.waiting().await.ok();
        }
    });
    service::serve_client(handler, (cr, cw))
        .await
        .map_err(|e| GenError::startup(e.to_string()))
}

#[tracing::instrument]
fn workspace_path(relative: &str) -> Result<String, GenError> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest
        .parent()
        .ok_or_else(|| GenError::path("CARGO_MANIFEST_DIR has no parent"))?
        .parent()
        .ok_or_else(|| GenError::path("crates/ dir has no parent"))?;
    Ok(workspace_root.join(relative).to_string_lossy().into_owned())
}

/// Call `create_narrative` and write the returned TOML to `relative_path`.
/// Skips if the file already exists.
#[tracing::instrument(skip(svc))]
async fn generate_narrative(
    svc: &service::RunningService<RoleClient, PrebakedHandler>,
    relative_path: &str,
) -> Result<(), GenError> {
    let path = workspace_path(relative_path)?;

    if std::path::Path::new(&path).exists() {
        return Ok(());
    }

    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("create_narrative"))
        .await
        .map_err(|e| GenError::mcp(e.to_string()))?;

    let toml = result
        .content
        .iter()
        .filter_map(|c| c.raw.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    if result.is_error.unwrap_or(false) {
        return Err(GenError::write(format!(
            "server error for {relative_path}: {toml}"
        )));
    }

    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| GenError::write(e.to_string()))?;
    }
    tokio::fs::write(&path, &toml)
        .await
        .map_err(|e| GenError::write(e.to_string()))
}

// ============================================================================
// Pre-baked narrative JSON values
// ============================================================================

/// generation_carousel.toml — `batch_generate` narrative key
fn generation_carousel_json() -> serde_json::Value {
    json!({
        "acts": {
            "generate_content": "Generate a short, engaging piece of content suitable for a Discord community. Keep it concise and interesting."
        },
        "bots": {},
        "tables": {},
        "media": {},
        "narrative": {
            "batch_generate": {
                "description": "Batch content generation carousel for Discord",
                "skip_content_generation": false,
                "toc": ["generate_content"],
                "acts": {}
            }
        },
        "narratives": {}
    })
}

/// curation.toml — `curate_and_approve` narrative key
fn curation_json() -> serde_json::Value {
    json!({
        "acts": {
            "review_content": "Review the following generated content and determine whether it is appropriate and engaging for a Discord community. Respond with APPROVE or REJECT, followed by a brief reason."
        },
        "bots": {},
        "tables": {},
        "media": {},
        "narrative": {
            "curate_and_approve": {
                "description": "Review and approve generated content before posting",
                "skip_content_generation": false,
                "toc": ["review_content"],
                "acts": {}
            }
        },
        "narratives": {}
    })
}

/// posting.toml — `post_approved` narrative key
fn posting_json() -> serde_json::Value {
    json!({
        "acts": {
            "prepare_post": "Format the following approved content for posting to Discord. Ensure it fits within Discord's 2000 character limit and reads naturally."
        },
        "bots": {},
        "tables": {},
        "media": {},
        "narrative": {
            "post_approved": {
                "description": "Format and post approved content to Discord",
                "skip_content_generation": false,
                "toc": ["prepare_post"],
                "acts": {}
            }
        },
        "narratives": {}
    })
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_generate_generation_carousel() -> Result<(), GenError> {
    let svc = in_proc_server(PrebakedHandler::new([generation_carousel_json()])).await?;
    generate_narrative(
        &svc,
        "crates/botticelli_narrative/narratives/discord/generation_carousel.toml",
    )
    .await
}

#[tokio::test]
async fn test_generate_curation() -> Result<(), GenError> {
    let svc = in_proc_server(PrebakedHandler::new([curation_json()])).await?;
    generate_narrative(
        &svc,
        "crates/botticelli_narrative/narratives/discord/curation.toml",
    )
    .await
}

#[tokio::test]
async fn test_generate_posting() -> Result<(), GenError> {
    let svc = in_proc_server(PrebakedHandler::new([posting_json()])).await?;
    generate_narrative(
        &svc,
        "crates/botticelli_narrative/narratives/discord/posting.toml",
    )
    .await
}
