# Elicitation MCP Integration Plan

## Executive Summary

Based on exploration of `botticelli_mcp`, we have a clear path to integrate the elicitation crate's paradigm-based system with the existing MCP architecture.

**Key Insight**: Add primitive elicitation tools (`elicit_text`, `elicit_select`, etc.) to the MCP server, allowing the elicitation crate's `#[derive(Elicit)]` macros to work seamlessly.

---

## Current Architecture

### botticelli_mcp Structure

```
crates/botticelli_mcp/src/
├── tools/
│   ├── mod.rs                    # McpTool trait, ToolRegistry
│   ├── echo.rs                   # Simple example
│   ├── elicitation/              # LLM-driven narrative creation
│   │   ├── session_tools.rs      # CreateNarrativeSession, ElicitMetadata, etc.
│   │   └── registry.rs           # PartialNarrativeRegistry
│   ├── execution/                # ExecuteAct, ExecuteNarrative
│   └── llm/                      # Backend-specific tools (Anthropic, Gemini, etc.)
├── resources/                    # MCP resources (narrative://, content://)
├── transport/                    # HTTP client transport
├── pmcp_server.rs                # Modern PMCP server setup
├── pmcp_adapters.rs              # McpToolAdapter bridge
└── server.rs                     # Legacy custom router
```

### Key Components

**1. McpTool Trait** (`tools/mod.rs`):
```rust
#[async_trait]
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, input: Value) -> McpResult<Value>;
}
```

**2. Tool Registration** (`pmcp_server.rs`):
```rust
pub fn register_all_tools(
    mut builder: pmcp::ServerBuilder,
    #[cfg(feature = "database")] db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
) -> pmcp::ServerBuilder {
    builder = builder.tool("echo", McpToolAdapter::new(EchoTool));
    // ... more tools
    builder
}
```

**3. Resource Injection Pattern**:
```rust
// DatabaseRegistryOperations passed to tools that need it
#[cfg(feature = "database")]
pub trait DatabaseRegistryOperations: Send + Sync {
    fn get_connection(&self) -> diesel::PgConnection;
}
```

---

## Implementation Plan

### Phase 1: Add Primitive Elicitation Tools

**Goal**: Add `elicit_text`, `elicit_select`, `elicit_number`, `elicit_bool` tools that delegate to `ElicitationDialog`.

#### 1.1 Create ElicitationDialog Resource

```rust
// crates/botticelli_mcp/src/dialog.rs

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Dialog resource for MCP tools to interact with UI.
///
/// This is passed to elicitation tools at server construction time.
pub struct DialogResource {
    dialog: Arc<Mutex<Box<dyn ElicitationDialog>>>,
}

impl DialogResource {
    pub fn new(dialog: Box<dyn ElicitationDialog>) -> Self {
        Self {
            dialog: Arc::new(Mutex::new(dialog)),
        }
    }

    pub async fn ask_text(&self, prompt: &str) -> BotticelliResult<String> {
        self.dialog.lock().await.ask_text(prompt).await
    }

    pub async fn ask_choice(&self, prompt: &str, options: &[&str]) -> BotticelliResult<usize> {
        self.dialog.lock().await.ask_choice(prompt, options).await
    }

    pub async fn ask_number(&self, prompt: &str, min: i64, max: i64) -> BotticelliResult<i64> {
        self.dialog.lock().await.ask_number(prompt, min, max).await
    }

    pub async fn ask_confirmation(&self, prompt: &str, default: bool) -> BotticelliResult<bool> {
        self.dialog.lock().await.ask_confirmation(prompt, default).await
    }
}

/// Re-export ElicitationDialog trait from botticelli_mcp module.
pub use crate::elicitation::dialog::ElicitationDialog;
```

#### 1.2 Create Primitive Elicitation Tools

```rust
// crates/botticelli_mcp/src/tools/elicitation_primitives.rs

use crate::dialog::DialogResource;
use crate::tools::{McpResult, McpTool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::instrument;

/// Tool for eliciting text input.
pub struct ElicitTextTool {
    dialog: Arc<DialogResource>,
}

impl ElicitTextTool {
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitTextTool {
    fn name(&self) -> &str {
        "elicit_text"
    }

    fn description(&self) -> &str {
        "Elicit free-form text input from the user"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to display to the user"
                }
            },
            "required": ["prompt"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_text"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt' parameter"))?;

        let text = self
            .dialog
            .ask_text(prompt)
            .await
            .map_err(|e| McpError::execution(format!("Dialog error: {}", e)))?;

        Ok(json!({ "value": text }))
    }
}

/// Tool for selecting from finite options.
pub struct ElicitSelectTool {
    dialog: Arc<DialogResource>,
}

impl ElicitSelectTool {
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitSelectTool {
    fn name(&self) -> &str {
        "elicit_select"
    }

    fn description(&self) -> &str {
        "Select one option from a finite list"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to display"
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Array of valid options"
                }
            },
            "required": ["prompt", "options"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_select"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt'"))?;

        let options: Vec<String> = input
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .ok_or_else(|| McpError::invalid_input("Missing or invalid 'options'"))?;

        // Convert to &str array for dialog API
        let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();

        let index = self
            .dialog
            .ask_choice(prompt, &option_refs)
            .await
            .map_err(|e| McpError::execution(format!("Dialog error: {}", e)))?;

        let selected = options
            .get(index)
            .ok_or_else(|| McpError::execution(format!("Invalid index: {}", index)))?;

        Ok(json!({ "value": selected }))
    }
}

/// Tool for eliciting numeric input with range constraints.
pub struct ElicitNumberTool {
    dialog: Arc<DialogResource>,
}

impl ElicitNumberTool {
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitNumberTool {
    fn name(&self) -> &str {
        "elicit_number"
    }

    fn description(&self) -> &str {
        "Elicit a number within a specified range"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "min": { "type": "integer" },
                "max": { "type": "integer" }
            },
            "required": ["prompt", "min", "max"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_number"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt'"))?;

        let min = input
            .get("min")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::invalid_input("Missing 'min'"))?;

        let max = input
            .get("max")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::invalid_input("Missing 'max'"))?;

        let num = self
            .dialog
            .ask_number(prompt, min, max)
            .await
            .map_err(|e| McpError::execution(format!("Dialog error: {}", e)))?;

        Ok(json!({ "value": num }))
    }
}

/// Tool for boolean confirmation.
pub struct ElicitBoolTool {
    dialog: Arc<DialogResource>,
}

impl ElicitBoolTool {
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitBoolTool {
    fn name(&self) -> &str {
        "elicit_bool"
    }

    fn description(&self) -> &str {
        "Elicit a yes/no confirmation"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "default": { "type": "boolean" }
            },
            "required": ["prompt"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_bool"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt'"))?;

        let default = input
            .get("default")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let confirmed = self
            .dialog
            .ask_confirmation(prompt, default)
            .await
            .map_err(|e| McpError::execution(format!("Dialog error: {}", e)))?;

        Ok(json!({ "value": confirmed }))
    }
}
```

#### 1.3 Register Tools in PMCP Server

```rust
// crates/botticelli_mcp/src/pmcp_server.rs

pub fn register_all_tools(
    mut builder: pmcp::ServerBuilder,
    dialog: Option<Arc<DialogResource>>,  // NEW parameter
    #[cfg(feature = "database")] db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
) -> pmcp::ServerBuilder {
    // Existing tools
    builder = builder.tool("echo", McpToolAdapter::new(EchoTool));

    // NEW: Primitive elicitation tools (if dialog provided)
    if let Some(dialog_res) = dialog {
        builder = builder.tool(
            "elicit_text",
            McpToolAdapter::new(ElicitTextTool::new(dialog_res.clone())),
        );
        builder = builder.tool(
            "elicit_select",
            McpToolAdapter::new(ElicitSelectTool::new(dialog_res.clone())),
        );
        builder = builder.tool(
            "elicit_number",
            McpToolAdapter::new(ElicitNumberTool::new(dialog_res.clone())),
        );
        builder = builder.tool(
            "elicit_bool",
            McpToolAdapter::new(ElicitBoolTool::new(dialog_res)),
        );
    }

    // Database tools
    #[cfg(feature = "database")]
    if let Some(db) = db_ops {
        builder = builder.tool("query_content", McpToolAdapter::new(QueryContentTool::new(db)));
    }

    builder
}
```

---

### Phase 2: Create In-Process Transport

**Goal**: Allow botticelli_chat to connect to the MCP server in-process without stdio/HTTP overhead.

```rust
// crates/botticelli_mcp/src/transport/in_proc.rs

use pmcp::shared::transport::{Transport, TransportMessage};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// In-process transport using channels.
///
/// Connects a pmcp::Client to an in-process MCP server
/// without external I/O overhead.
#[derive(Debug, Clone)]
pub struct InProcTransport {
    request_tx: mpsc::Sender<TransportMessage>,
    response_rx: Arc<Mutex<mpsc::Receiver<TransportMessage>>>,
}

/// Server-side handle for in-process transport.
pub struct InProcServerHandle {
    server: Arc<pmcp::Server>,
    request_rx: mpsc::Receiver<TransportMessage>,
    response_tx: mpsc::Sender<TransportMessage>,
}

impl InProcTransport {
    /// Create a paired transport and server handle.
    pub fn pair(server: pmcp::Server) -> (Self, InProcServerHandle) {
        let (req_tx, req_rx) = mpsc::channel(100);
        let (resp_tx, resp_rx) = mpsc::channel(100);

        let handle = InProcServerHandle {
            server: Arc::new(server),
            request_rx: req_rx,
            response_tx: resp_tx,
        };

        let transport = Self {
            request_tx: req_tx,
            response_rx: Arc::new(Mutex::new(resp_rx)),
        };

        (transport, handle)
    }
}

impl Transport for InProcTransport {
    async fn send(&mut self, message: TransportMessage) -> pmcp::error::Result<()> {
        self.request_tx
            .send(message)
            .await
            .map_err(|e| pmcp::error::Error::Transport(e.to_string()))
    }

    async fn receive(&mut self) -> pmcp::error::Result<TransportMessage> {
        self.response_rx
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| pmcp::error::Error::Transport("Channel closed".into()))
    }

    async fn close(&mut self) -> pmcp::error::Result<()> {
        Ok(())
    }

    fn transport_type(&self) -> &'static str {
        "in-process"
    }
}

impl InProcServerHandle {
    /// Run the server loop handling requests from the paired transport.
    pub async fn run(mut self) -> pmcp::error::Result<()> {
        while let Some(message) = self.request_rx.recv().await {
            match message {
                TransportMessage::Request { id, request } => {
                    let response = self.server.handle_request(request).await?;
                    self.response_tx
                        .send(TransportMessage::Response(response))
                        .await
                        .map_err(|e| pmcp::error::Error::Transport(e.to_string()))?;
                }
                _ => {
                    tracing::warn!("Unexpected message type in InProcTransport");
                }
            }
        }
        Ok(())
    }
}
```

---

### Phase 3: Update botticelli_chat to Use Elicitation Derive Macros

**Goal**: Refactor elicitors to use `#[derive(Elicit)]` with real MCP server.

```rust
// crates/botticelli_chat/src/elicitation/metadata.rs

use botticelli_chat::elicitation::NarrativeMetadata;
use botticelli_error::BotticelliResult;
use botticelli_mcp::{DialogResource, pmcp_server, transport::InProcTransport};
use elicitation::Elicitation;
use std::sync::Arc;

pub async fn elicit_metadata(
    dialog: Box<dyn ElicitationDialog>,
) -> BotticelliResult<NarrativeMetadata> {
    // 1. Create DialogResource
    let dialog_res = Arc::new(DialogResource::new(dialog));

    // 2. Build MCP server with elicitation tools
    let server = pmcp_server::register_all_tools(
        pmcp::ServerBuilder::new("botticelli-elicitation", "0.1.0"),
        Some(dialog_res),
        #[cfg(feature = "database")] None,
    )
    .build();

    // 3. Create in-process transport
    let (transport, server_handle) = InProcTransport::pair(server);

    // 4. Spawn server task
    tokio::spawn(async move {
        if let Err(e) = server_handle.run().await {
            tracing::error!("Server error: {}", e);
        }
    });

    // 5. Create MCP client
    let client = pmcp::Client::new(transport);

    // 6. Use elicitation derive macros! 🎉
    let metadata = NarrativeMetadata::elicit(&client)
        .await
        .map_err(|e| BotticelliError::new(format!("Elicitation failed: {}", e)))?;

    Ok(metadata)
}
```

---

## Benefits

✅ **Leverage existing infrastructure**: Uses botticelli_mcp's tool system
✅ **Separation of concerns**: Dialog trait separates UI from logic
✅ **Multiple deployment modes**: Same code works TUI, web, external MCP
✅ **Derive macros work**: `#[derive(Elicit)]` works with any transport
✅ **Type safety**: Compile-time guarantees from paradigm types
✅ **Testable**: Mock dialog for tests, in-proc transport for integration tests

---

## Migration Path

### Week 1: Infrastructure
- [ ] Add `DialogResource` to botticelli_mcp
- [ ] Implement primitive elicitation tools
- [ ] Update `register_all_tools()` signature
- [ ] Create `InProcTransport`

### Week 2: Integration Testing
- [ ] Test primitive tools with mock dialog
- [ ] Test in-process transport
- [ ] Verify elicitation crate integration

### Week 3: Refactor Elicitors
- [ ] Refactor `MetadataElicitor` to use derives
- [ ] Measure code reduction
- [ ] Document patterns

### Week 4: Complete Migration
- [ ] Refactor remaining elicitors
- [ ] Update tests
- [ ] Documentation and examples

---

## Open Questions

1. **Should DialogResource be Clone?** For sharing across multiple tool instances
2. **Error handling strategy?** Map BotticelliError to pmcp::Error or vice versa?
3. **Server lifecycle?** Should server shutdown when client drops, or live for session?
4. **Thread safety?** Arc<Mutex<...>> vs Arc<RwLock<...>> for DialogResource?

---

## Next Steps

1. Implement `DialogResource` in botticelli_mcp
2. Add primitive elicitation tools
3. Test with mock dialog
4. Create in-process transport
5. Refactor one elicitor as proof of concept
