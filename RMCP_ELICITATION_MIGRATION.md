# rmcp + Elicitation Framework Migration Plan

**Branch**: `dev`  
**Status**: 🚧 In Progress  
**Started**: 2026-06-10  

Replace `pmcp`, `mcp-server`, and `mcp-spec` with `rmcp` + the `elicitation` framework
throughout `botticelli_mcp`, hollow out `botticelli_chat`, and rewrite `botticelli_tui`
to talk through a thin `botticelli_mcp_client`.

---

## Architecture

The target architecture has three layers:

```
botticelli_tui  (user-facing layer)
     │
     ▼
botticelli_mcp_client  (thin rmcp client — view + input)
     │  rmcp transport (stdio or in-proc)
     ▼
botticelli_mcp  (server — owns ALL state, orchestration, tools, LLM loop)
     │
     ▼ (domain crates)
botticelli_narrative / botticelli_models / botticelli_database / …
```

**Server (`botticelli_mcp`)** is the application. It owns:
- Session state and lifecycle
- `BotticelliServer` with all `#[tool]` methods
- Orchestration (conversation loop, LLM calls, sampling)
- `DynamicToolRegistry` for phase-progression tool gating
- `ElicitServer` wrapping each `RequestContext` peer for elicitation

**Client (`botticelli_mcp_client`)** is a thin view. It:
- Holds an rmcp client connection to the server
- Submits user input as tool calls
- Answers sampling requests (`create_message`) from the server — this is how
  elicitation prompts reach the user
- Renders server-side state for display

**TUI (`botticelli_tui`)** calls only into `botticelli_mcp_client`. It never
imports domain crates directly.

**`botticelli_chat`** is hollowed out then deleted. All orchestration moves to the
server. All UI moves to `botticelli_tui`. Nothing is lost — the features migrate.

### ElicitCommunicator Pattern

The `elicitation` crate (upstreamed from `strictly_games`) exports `ElicitCommunicator`:
a trait that abstracts over "given a prompt, return an answer." Implementations:

| Implementation | Where | Usage |
| --- | --- | --- |
| `TuiCommunicator` | client-side sampling handler | human answers from terminal |
| `LlmElicitCommunicator` | client-side sampling handler | agent auto-answers via LLM |

On the server, `ElicitServer::new(ctx.peer)` wraps the `RequestContext` peer and sends
elicitation prompts back over the MCP `create_message` (sampling) protocol. The client's
sampling handler is the communicator implementation — it reads from terminal or calls the
LLM and returns the answer.

This means **no special wire protocol** is needed. The MCP sampling channel is the
transport. The communicator is the client-side policy for answering.

---

## Background

Botticelli previously used `pmcp` (a community MCP SDK) as its server backbone, with a
hand-rolled `McpTool` trait and `McpToolAdapter` bridge. The `elicitation` crate (our own
framework) provides first-class `rmcp`-native tooling:

- `ElicitServer` / `ElicitClient` / `ElicitCommunicator` — structured type elicitation via
  MCP sampling (`create_message`)
- `#[derive(Elicit)]` — auto-generates elicitation logic for any Rust type, giving validation
  and formal verification "for free"
- `PluginRegistry` / `ElicitPlugin` — namespaced rmcp `ServerHandler` aggregation
- `#[tool_router]` / `#[tool]` — rmcp macros for declaring tool methods on a server struct
- `DynamicToolRegistry` — live tool registration changes with `notify_tool_list_changed()`

The `elicitation` library was developed to solve exactly the problems encountered while
building botticelli. It incorporates all lessons learned and its patterns are the
authoritative "right way" for this codebase.

### Footgun Rules (apply everywhere)
Whenever you write `#[derive(Elicit)]`, you MUST also have:
- `#[derive(schemars::JsonSchema)]` — rmcp requires this at runtime (will panic without it)
- `#[derive(Serialize, Deserialize)]` — required by the elicitation framework

If you only apply one or two of these, the code will compile but fail at runtime.

---

## Migration Checklist

### Phase 1 — Workspace plumbing ✅

- [x] Add `rmcp = "1.7"` to workspace `Cargo.toml` with features:
  `["server", "client", "transport-io", "transport-streamable-http-server",
  "transport-streamable-http-client", "transport-streamable-http-client-reqwest",
  "schemars"]`
- [x] Add `schemars = { version = "1", features = ["derive"] }` to workspace `Cargo.toml`
- [x] Remove `mcp-spec` entry from workspace `Cargo.toml`
- [x] Update `botticelli_mcp/Cargo.toml`: added `rmcp.workspace = true`, `schemars.workspace = true`
- [x] `just check -p botticelli_mcp` passes with zero errors/warnings

---

### Phase 2 — JsonSchema into botticelli_core ✅

- [x] Add `schemars.workspace = true` to `botticelli_core/Cargo.toml`
- [x] `botticelli_core/src/media.rs`: add `JsonSchema` to `MediaSource`
- [x] `botticelli_core/src/tool_definition.rs`: add `JsonSchema` to `ToolDefinition`
- [x] `botticelli_core/src/input.rs`: add `JsonSchema` to `Input`, `HistoryRetention`, `TableFormat`
- [x] `botticelli_core/src/output.rs`: add `JsonSchema` to `Output`, `ToolCall`, `StopReason`
- [x] `botticelli_core/src/budget.rs`: add `JsonSchema` to `BudgetConfig`
- [x] `just check -p botticelli_core` passes

---

### Phase 3 — Elicit + JsonSchema into botticelli_narrative ✅

- [x] Add `elicitation.workspace = true`, `schemars.workspace = true`,
  `rmcp.workspace = true`, `derive-new.workspace = true` to
  `botticelli_narrative/Cargo.toml`
- [x] Add `elicitation.workspace = true` and `rmcp.workspace = true` to
  `botticelli_core/Cargo.toml`
- [x] Enable `elicitation` feature `"serde_json"` in workspace `Cargo.toml`
  so `serde_json::Value` implements `Elicitation` (needed by `Input` variants)
- [x] `botticelli_core`: derive `elicitation::Elicit` on `MediaSource`,
  `HistoryRetention`, `Input`, `TableFormat`, `Output`, `ToolCall`, `StopReason`,
  `ToolDefinition`, `BudgetConfig`
- [x] `botticelli_narrative/src/carousel.rs`: add `Elicit + JsonSchema` to `CarouselConfig`
- [x] `botticelli_narrative/src/core.rs`: add `Elicit + JsonSchema` to `NarrativeMetadata`
- [x] `botticelli_narrative/src/provider.rs`: add `Elicit + JsonSchema` to `ActConfig`

#### Move PartialNarrative / PartialAct here
- [x] Create `botticelli_narrative/src/partial.rs` with moved `PartialNarrative`/`PartialAct`
- [x] Add `Elicit + JsonSchema` to both types; fields changed to `pub` (transitional)
- [x] `escape_toml_string` inlined as private fn (breaks circular dep on narrative_utils)
- [x] Export `PartialNarrative`, `PartialAct`, `PartialNarrativeBuilder` from
  `botticelli_narrative/src/lib.rs`
- [x] Remove `partial.rs` from `botticelli_mcp/src/elicitation/` and update its `mod.rs`
- [x] All botticelli_mcp internal imports updated to `botticelli_narrative::{…}` (no re-export)
- [x] `just lint botticelli_core` and `just lint botticelli_narrative` and
  `just lint botticelli_mcp` all pass with zero warnings/errors

---

### Phase 4 — New rmcp server struct in botticelli_mcp ✅

Pattern reference: `strictly_games/crates/strictly_server/src/server.rs` (GameServer)

- [x] Create `botticelli_mcp/src/server/mod.rs` with `BotticelliServer` struct
  ```rust
  pub struct BotticelliServer {
      tool_router: ToolRouter<Self>,
      dynamic: DynamicToolRegistry,
  }
  ```
- [x] Implement `rmcp::ServerHandler` for `BotticelliServer`
  - `get_info()` returns server name/version/capabilities
  - `list_tools()` delegates to `tool_router` + `dynamic`
  - `call_tool()` delegates to `tool_router` + `dynamic`
- [x] Delete dead infrastructure (pmcp adapters, old server, dialog, elicitation, HTTP layer)
- [x] Updated `src/bin/botticelli-mcp.rs` to use `BotticelliServer` + rmcp stdio
- [x] Update `src/lib.rs` — remove all pmcp/mcp_server re-exports, add `BotticelliServer`
- [x] Remove `pmcp`, `mcp-server`, `mcp-spec` from `botticelli_mcp/Cargo.toml`
- [x] `just test-all botticelli_mcp` passes — zero warnings, all tests passing

---

### Phase 5 — Port tools to #[tool] methods ✅

All 24 tools ported to `#[tool]` methods on `BotticelliServer` in
`src/server/tool_impls.rs`. The 8-tool elicitation session stack replaced by a
single `create_narrative` tool that calls
`PartialNarrative::elicit(&ElicitServer::new(ctx.peer))`.

- [x] `echo` → `#[tool] async fn echo(…)` on `BotticelliServer`
- [x] `server_info` → `#[tool] async fn server_info(…)` on `BotticelliServer`
- [x] 8-tool session stack collapses to single `#[tool] async fn create_narrative(…)`
  via `PartialNarrative::elicit(&ElicitServer::new(ctx.peer)).await`
- [x] All narrative manipulation tools ported: `validate_narrative`, `save_narrative`,
  `modify_narrative`, `generate_narrative`
- [x] All execution tools ported: `generate`, `execute_act`, `execute_narrative`
- [x] All scene management tools ported: `create_scene`, `list_scenes`, `update_scene`,
  `delete_scene`
- [x] All LLM provider tools ported (feature-gated inside bodies, not on declarations):
  `generate_gemini`, `generate_anthropic`, `generate_ollama`, `generate_huggingface`,
  `generate_groq`, `generate_llm`
- [x] All Discord tools ported (feature-gated): `discord_post_message`,
  `discord_get_messages`, `discord_get_guild_info`, `discord_get_channels`
- [x] Metrics tool ported: `export_metrics`
- [x] `McpTool` trait and `ToolRegistry` retained with `#[doc]` backward-compat note
  (removed in Phase 11 when `botticelli_chat` dependency is severed)
- [x] `tools/elicitation/` directory deleted entirely (session tools replaced)
- [x] Stale pmcp-era test files deleted
- [x] `just check-all botticelli_mcp` passes — zero warnings, zero errors

---

### Phase 6 — Rewrite botticelli_mcp_client as thin rmcp client

The current `botticelli_mcp_client` is a fat orchestration layer: it holds LLM adapters,
duplicate tool implementations, approval managers, and its own orchestrator. This all moves
server-side. The new client is a thin view + input relay.

#### Drop orchestration
- [ ] Remove `pmcp` from `botticelli_mcp_client/Cargo.toml`
- [ ] Delete `src/tools/` directory (all tool logic is server-side now)
- [ ] Delete `src/orchestrator.rs` (orchestration is server-side)
- [ ] Delete `src/llm_adapter.rs` (LLM calls are server-side)
- [ ] Delete `src/adapter_bridge.rs`, `src/approval.rs`, `src/retry.rs` (server-side concerns)
- [ ] Delete `src/context.rs` (context management server-side)
- [ ] Remove `botticelli_narrative`, `botticelli_database`, `botticelli_social` deps
  (client talks only to the server, never domain crates directly)

#### Add thin rmcp client
- [ ] Add `rmcp.workspace = true` (client features) to `botticelli_mcp_client/Cargo.toml`
- [ ] Add `elicitation.workspace = true` to `botticelli_mcp_client/Cargo.toml`
- [ ] Create `src/connection.rs` — wraps rmcp transport + session lifecycle
  - `BotticelliClient::connect_stdio(command)` → spawns server process, returns client
  - `BotticelliClient::connect_in_proc(server)` → in-process rmcp (no subprocess)
  - Holds `rmcp::service::RunningService<RoleClient, _>` or equivalent
- [ ] Create `src/communicator.rs` — `TuiCommunicator` implementing `ElicitCommunicator`
  - Handles sampling (`create_message`) requests from the server
  - Reads user input from terminal (crossterm / readline)
  - Returns the answer back over the MCP sampling protocol
- [ ] Create `src/session.rs` — high-level session facade:
  - `call_tool(name, args)` — submits a tool call to the server
  - `list_tools()` — returns available tools (server-driven)
  - `subscribe_state()` — poll / stream state changes from server resources
- [ ] Update `src/lib.rs` — export only: `BotticelliClient`, `TuiCommunicator`,
  `McpClientError`; remove all old exports
- [ ] `just check -p botticelli_mcp_client` passes — zero warnings

#### Remove ExternalMcpClient duplication
- [ ] Evaluate `src/external_client.rs` — if it is a generic rmcp wrapper for connecting
  to non-Botticelli MCP servers, keep it; otherwise delete
- [ ] Remove `src/schema/` if schema inference moves server-side

---

### Phase 7 — Hollow out botticelli_chat

`botticelli_chat` currently owns: `ConversationLoop`, `McpChatHost`, `ServiceContainer`,
`SamplingCoordinator`, the `Executor`, startup logic, and command parsing. All orchestration
moves to `botticelli_mcp`. All UI moves to `botticelli_tui`. The crate becomes an empty
shell, ready for deletion.

#### Move orchestration into botticelli_mcp server
- [ ] Audit `botticelli_chat/src/conversation_loop.rs` — extract the core turn-taking
  logic as a server-side async task; delete the file
- [ ] Audit `botticelli_chat/src/mcp_chat_host.rs` — fold capabilities into
  `BotticelliServer` session management; delete the file
- [ ] Audit `botticelli_chat/src/services.rs` (`ServiceContainer`) — services are
  already held on `BotticelliServer` fields; delete the file
- [ ] Audit `botticelli_chat/src/sampling.rs` / `src/sampling_integration.rs` —
  sampling is now `ElicitServer` in `BotticelliServer`; delete both files
- [ ] Audit `botticelli_chat/src/executor.rs` — execution logic moves to the
  `execute_act` / `execute_narrative` tools on the server; delete the file
- [ ] Audit `botticelli_chat/src/startup.rs` — server startup moves to `botticelli-mcp`
  binary; delete the file

#### Move command / input parsing into botticelli_tui
- [ ] Move `botticelli_chat/src/command.rs` and `src/parser.rs` into
  `botticelli_tui/src/` (they are UI concerns, not orchestration)
- [ ] Move `botticelli_chat/src/input.rs`, `src/interface.rs`, `src/message.rs`,
  `src/response.rs` to `botticelli_tui/src/`

#### Sever botticelli_chat dependencies
- [ ] Remove `botticelli_chat` from `botticelli_tui/Cargo.toml`
- [ ] Ensure `botticelli_tui` imports only from `botticelli_mcp_client`
- [ ] `cargo tree -p botticelli_chat --invert` must show nothing

---

### Phase 8 — Rewrite botticelli_tui to use botticelli_mcp_client

The TUI currently shells out through `botticelli_chat`. After Phase 7, it calls directly
into the thin `botticelli_mcp_client`.

- [ ] Replace all `use botticelli_chat::…` imports with `use botticelli_mcp_client::…`
- [ ] Rewrite main event loop:
  - Connect: `BotticelliClient::connect_stdio("botticelli-mcp")` or in-proc variant
  - Accept input → `client.call_tool("…", args)`
  - Receive sampling requests from server → `TuiCommunicator` handles them (elicitation)
  - Render state from server resources / tool results
- [ ] Integrate `TuiCommunicator` as the sampling handler at connection time so the
  server's `ElicitServer` can prompt the user through the established channel
- [ ] Smoke test: `cargo run -p botticelli_tui` starts, connects to server, accepts a
  command, returns a response
- [ ] `just check-all botticelli_tui` passes

---

### Phase 9 — Delete botticelli_chat

- [ ] Verify `cargo tree -p botticelli_chat --invert` is empty
- [ ] Remove `crates/botticelli_chat` from workspace `Cargo.toml` members
- [ ] Delete `crates/botticelli_chat/` directory
- [ ] `just check` (full workspace) passes — zero references to deleted crate

---

### Phase 10 — Single binary with clap subcommands

Pattern reference: `strictly_games/crates/strictly_games/src/main.rs`

- [ ] Update `src/bin/botticelli-mcp.rs` with clap subcommands:
  ```
  botticelli-mcp serve          # stdio transport (default)
  botticelli-mcp http [--host] [--port]   # streamable-http transport
  ```
- [ ] stdio: `rmcp::service::serve_server(BotticelliServer::new(), rmcp::transport::stdio())`
- [ ] http: `StreamableHttpService` + axum Router (matching strictly_games pattern)
- [ ] Update `[[bin]]` entries in `Cargo.toml` (single entry, no `required-features`)
- [ ] `just check-features -p botticelli_mcp` passes with all feature combinations

---

### Phase 11 — ToolRegistry cleanup in botticelli_interface

- [ ] Audit `botticelli_interface` for any remaining `McpTool` / old `ToolRegistry`
  references
- [ ] `ToolDefinition` in `botticelli_core` stays — it is the LLM tool-calling concept,
  not the MCP server concept
- [ ] Simplify or remove `ElicitationRegistryOperations<T>` in
  `botticelli_interface/src/registry_traits.rs` if superseded by elicitation plugin model
- [ ] Remove interface traits that were purely bridging to the old pmcp stack
- [ ] Delete `McpTool` trait and `ToolRegistry` from `botticelli_mcp/src/tools/mod.rs`
  (`botticelli_chat` is gone; backward-compat note no longer needed)
- [ ] `just check` (full workspace) passes

---

### Phase 12 — Test suite

- [ ] Write `tests/server_tools_test.rs` — each `#[tool]` method on `BotticelliServer`
  using rmcp in-process transport
- [ ] Write `tests/client_connection_test.rs` — `BotticelliClient` connecting to a local
  server, calling a tool, receiving a result
- [ ] Write `tests/elicitation_round_trip_test.rs` — in-process: server calls
  `ElicitServer`, `TuiCommunicator` on the client side answers, result returns
- [ ] Delete or rewrite any remaining pmcp-era test stubs
- [ ] `just test-package botticelli_mcp` passes
- [ ] `just test-package botticelli_mcp_client` passes

---

### Phase 13 — Final verification

- [ ] `just check-all` (clippy + fmt + tests) passes with zero warnings
- [ ] `just audit` passes
- [ ] `just check-features` passes (all feature combinations)
- [ ] `markdownlint-cli2 "**/*.md"` passes (after updating this doc)
- [ ] Smoke tests:
  - `cargo run -p botticelli_mcp -- serve` starts and responds to MCP initialize
  - `cargo run -p botticelli_mcp -- http` starts and responds on the default port
  - `cargo run -p botticelli_tui` connects, submits a command, returns a response
- [ ] PR to main

---

## File Deletion List

```
# Phase 4 — completed
crates/botticelli_mcp/src/pmcp_adapters.rs           ✅ deleted
crates/botticelli_mcp/src/pmcp_middleware.rs          ✅ deleted
crates/botticelli_mcp/src/pmcp_server.rs              ✅ deleted
crates/botticelli_mcp/src/pmcp_http_server.rs         ✅ deleted
crates/botticelli_mcp/src/server.rs                   ✅ deleted (old BotticelliRouter)
crates/botticelli_mcp/src/dialog_resource.rs          ✅ deleted
crates/botticelli_mcp/src/elicitation/dialog.rs       ✅ deleted
crates/botticelli_mcp/src/elicitation/elicitor.rs     ✅ deleted
crates/botticelli_mcp/src/elicitation/partial.rs      ✅ moved to botticelli_narrative
crates/botticelli_mcp/src/http.rs                     ✅ deleted
crates/botticelli_mcp/src/transport/in_proc.rs        ✅ deleted (not needed)
crates/botticelli_mcp/src/bin/botticelli-mcp-http.rs  ✅ deleted
crates/botticelli_mcp/src/bin/botticelli-mcp-pmcp.rs  ✅ deleted
crates/botticelli_mcp/src/bin/botticelli-mcp-pmcp-http.rs  ✅ deleted
crates/botticelli_mcp/tests/pmcp_http_test.rs         ✅ deleted
crates/botticelli_mcp/tests/elicitation_integration_test.rs  ✅ deleted
crates/botticelli_mcp/tests/in_proc_transport_test.rs  ✅ deleted

# Phase 5 — completed
crates/botticelli_mcp/src/tools/elicitation/          ✅ deleted (entire directory)
crates/botticelli_mcp/src/tools/elicitation_primitives.rs  ✅ deleted
crates/botticelli_mcp/src/tools/narrative_creation.rs  ✅ deleted
crates/botticelli_mcp/src/tools/create_narrative.rs   ✅ deleted
crates/botticelli_mcp/src/tools/database.rs           ✅ deleted
crates/botticelli_mcp/src/tools/echo.rs               ✅ deleted
crates/botticelli_mcp/src/tools/execute_act.rs        ✅ deleted
crates/botticelli_mcp/src/tools/execute_narrative.rs  ✅ deleted
crates/botticelli_mcp/src/tools/export_metrics.rs     ✅ deleted
crates/botticelli_mcp/src/tools/generate.rs           ✅ deleted
crates/botticelli_mcp/src/tools/save_narrative.rs     ✅ deleted
crates/botticelli_mcp/src/tools/scene.rs              ✅ deleted
crates/botticelli_mcp/src/tools/server_info.rs        ✅ deleted
crates/botticelli_mcp/src/tools/validate_narrative.rs ✅ deleted
crates/botticelli_mcp/src/tools/bot_commands.rs       ✅ deleted
crates/botticelli_mcp/src/tools/sampling_session_manager.rs  ✅ deleted
crates/botticelli_mcp/src/tools/get_narrative_state.rs  ✅ deleted
crates/botticelli_mcp/src/tools/validate_narrative_session.rs  ✅ deleted
crates/botticelli_mcp/tests/validate_narrative_test.rs  ✅ deleted
crates/botticelli_mcp/tests/execution_tools_test.rs   ✅ deleted
crates/botticelli_mcp/tests/integration_workflow_test.rs  ✅ deleted
crates/botticelli_mcp/tests/narrative_validation_test.rs  ✅ deleted
crates/botticelli_mcp/tests/database_tool_test.rs     ✅ deleted
crates/botticelli_mcp/tests/narrative_sampling_test.rs  ✅ deleted
crates/botticelli_mcp/tests/pmcp_server_test.rs       ✅ deleted
crates/botticelli_mcp/tests/narrative_generation_test.rs  ✅ deleted

# Phase 6 — pending
crates/botticelli_mcp_client/src/tools/              (entire directory)
crates/botticelli_mcp_client/src/orchestrator.rs
crates/botticelli_mcp_client/src/llm_adapter.rs
crates/botticelli_mcp_client/src/adapter_bridge.rs
crates/botticelli_mcp_client/src/approval.rs
crates/botticelli_mcp_client/src/retry.rs
crates/botticelli_mcp_client/src/context.rs

# Phase 7 — pending (as orchestration migrates out)
crates/botticelli_chat/src/conversation_loop.rs
crates/botticelli_chat/src/mcp_chat_host.rs
crates/botticelli_chat/src/services.rs
crates/botticelli_chat/src/sampling.rs
crates/botticelli_chat/src/sampling_integration.rs
crates/botticelli_chat/src/executor.rs
crates/botticelli_chat/src/startup.rs

# Phase 9 — pending
crates/botticelli_chat/                              (entire crate)
```

---

## Key Reference Files

- `rmcp` server pattern: `/home/erik/repos/strictly_games/crates/strictly_server/src/server.rs`
- HTTP transport: `/home/erik/repos/strictly_games/crates/strictly_games/src/main.rs`
- `ElicitCommunicator` trait: `/home/erik/repos/elicitation/crates/elicitation/src/communicator.rs`
- `ElicitPlugin` pattern: `/home/erik/repos/elicitation/crates/elicit_server/src/emit_plugin.rs`
- `ElicitServer` usage: `/home/erik/repos/elicitation/crates/elicitation/src/server.rs`
- `PluginRegistry`: `/home/erik/repos/elicitation/crates/elicitation/src/plugin_registry.rs`

---

## Derive Rule Reference

| Goal | Required derives |
| --- | --- |
| Tool input type (MCP) | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Tool output type | `Serialize, Deserialize, schemars::JsonSchema` |
| Domain type user can elicit | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Enum for Select paradigm | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Internal-only type | existing derives unchanged |
