# rmcp + Elicitation Framework Migration Plan

**Branch**: `dev`  
**Status**: 🚧 In Progress  
**Started**: 2026-06-10  

Replace `pmcp`, `mcp-server`, and `mcp-spec` with `rmcp` + the `elicitation` framework
throughout `botticelli_mcp`, and propagate `#[derive(Elicit, JsonSchema, Serialize, Deserialize)]`
to all input/output types across the workspace.

---

## Background

Botticelli previously used `pmcp` (a community MCP SDK) as its server backbone, with a
hand-rolled `McpTool` trait and `McpToolAdapter` bridge. The `elicitation` crate (our own
framework, six months in development) provides first-class `rmcp`-native tooling:

- `ElicitServer` / `ElicitClient` / `ElicitCommunicator` — structured type elicitation via
  MCP sampling (`create_message`)
- `#[derive(Elicit)]` — auto-generates elicitation logic for any Rust type, giving validation
  and formal verification "for free"
- `PluginRegistry` / `ElicitPlugin` — namespaced rmcp `ServerHandler` aggregation
- `#[tool_router]` / `#[tool]` — rmcp macros for declaring tool methods on a server struct
- `DynamicToolRegistry` — live tool registration changes with `notify_tool_list_changed()`

Every type that derives `Elicit` also becomes a first-class MCP tool exposed through the
plugin interface, giving more flexibility than the old hand-rolled approach, not less.

### Footgun Rules (apply everywhere)
Whenever you write `#[derive(Elicit)]`, you MUST also have:
- `#[derive(schemars::JsonSchema)]` — rmcp requires this at runtime (will panic without it)
- `#[derive(Serialize, Deserialize)]` — required by the elicitation framework

If you only apply one or two of these, the code will compile but fail at runtime.

---

## Migration Checklist

### Phase 1 — Workspace plumbing

- [x] Add `rmcp = "1.7"` to workspace `Cargo.toml` with features:
  `["server", "client", "transport-io", "transport-streamable-http-server",
  "transport-streamable-http-client", "transport-streamable-http-client-reqwest",
  "schemars"]`
- [x] Add `schemars = { version = "1", features = ["derive"] }` to workspace `Cargo.toml`
- [ ] Remove `pmcp` entry from workspace `Cargo.toml` *(deferred to Phase 4 with code removal)*
- [ ] Remove `mcp-spec` entry from workspace `Cargo.toml` *(deferred to Phase 4)*
- [x] Update `botticelli_mcp/Cargo.toml`:
  - pmcp + mcp-server kept with migration comment (removed in Phase 4)
  - Added `rmcp.workspace = true`
  - Added `schemars.workspace = true`
- [x] `just check -p botticelli_mcp` passes with zero errors/warnings

---

### Phase 2 — JsonSchema into botticelli_core

- [ ] Add `schemars.workspace = true` to `botticelli_core/Cargo.toml`
- [ ] `botticelli_core/src/tool_definition.rs`: add `schemars::JsonSchema`
- [ ] `botticelli_core/src/input.rs`: add `schemars::JsonSchema` to `Input` and all inner
  types (`HistoryRetention`, `TableFormat`, etc.)
- [ ] `botticelli_core/src/output.rs`: add `schemars::JsonSchema` to `Output`, `ToolCall`,
  `StopReason`
- [ ] `botticelli_core/src/budget.rs`: add `schemars::JsonSchema` to `BudgetConfig`
- [ ] `botticelli_core/src/request.rs`: add `schemars::JsonSchema` to `GenerateRequest`,
  `GenerateResponse`
- [ ] `just check -p botticelli_core` passes

---

### Phase 3 — Elicit + JsonSchema into botticelli_narrative

- [ ] Add `elicitation.workspace = true` and `schemars.workspace = true` to
  `botticelli_narrative/Cargo.toml`
- [ ] `botticelli_narrative/src/carousel.rs`:
  add `#[derive(Elicit, schemars::JsonSchema, Serialize, Deserialize)]` to `CarouselConfig`
  (already has Serialize/Deserialize — confirm and add the others)
- [ ] `botticelli_narrative/src/core.rs`:
  add `#[derive(Elicit, schemars::JsonSchema)]` to `NarrativeMetadata`
  (already has Serialize/Deserialize — confirm and add the others)
- [ ] `botticelli_narrative/src/provider.rs`:
  add `#[derive(Elicit, schemars::JsonSchema)]` to `ActConfig`
  (check Serialize/Deserialize — add if missing)

#### Move PartialNarrative / PartialAct here
- [ ] Copy `botticelli_mcp/src/elicitation/partial.rs` →
  `botticelli_narrative/src/partial.rs`
- [ ] Add `#[derive(Elicit, schemars::JsonSchema)]` to `PartialNarrative` and `PartialAct`
  (Serialize/Deserialize already present — confirm)
- [ ] Export `PartialNarrative`, `PartialAct`, `PartialNarrativeBuilder` from
  `botticelli_narrative/src/lib.rs`
- [ ] Remove `partial.rs` from `botticelli_mcp/src/elicitation/` and update its `mod.rs`
- [ ] `just check -p botticelli_narrative` passes

---

### Phase 4 — New rmcp server struct in botticelli_mcp

Pattern reference: `strictly_games/crates/strictly_server/src/server.rs` (GameServer)

- [ ] Create `botticelli_mcp/src/server/mod.rs` with a `BotticelliServer` struct
  ```rust
  pub struct BotticelliServer {
      tool_router: ToolRouter<Self>,
      dynamic: DynamicToolRegistry,
  }
  ```
- [ ] Implement `rmcp::ServerHandler` for `BotticelliServer`
  - `get_info()` returns server name/version/capabilities
  - `list_tools()` delegates to `tool_router` + `dynamic`
  - `call_tool()` delegates to `tool_router` + `dynamic`
- [ ] Delete dead infrastructure:
  - `src/pmcp_adapters.rs`
  - `src/pmcp_middleware.rs`
  - `src/pmcp_server.rs`
  - `src/pmcp_http_server.rs`
  - `src/server.rs` (old `BotticelliRouter` using `mcp-server`)
  - `src/dialog_resource.rs`
  - `src/elicitation/dialog.rs` (replaced by `ElicitServer`)
  - `src/elicitation/elicitor.rs` (replaced by `#[derive(Elicit)]`)
- [ ] Update `src/lib.rs` — remove all pmcp/mcp_server re-exports, add new server exports
- [ ] `just check -p botticelli_mcp` passes (may have unused-import warnings from tools — fix
  them)

---

### Phase 5 — Port tools to #[tool] methods

Delete `McpTool` trait and `ToolRegistry` struct from `tools/mod.rs` and convert each tool
to a `#[tool(description = "...")]` method on `BotticelliServer`.

For tools with complex input types, add `#[derive(Elicit, schemars::JsonSchema, Serialize,
Deserialize)]` to the input struct.

For tools that need to elicit from the user, receive `RequestContext<RoleServer>`, wrap in
`ElicitServer::new(ctx.peer)`, then call `.elicit()` on the appropriate type.

#### Core tools
- [ ] `echo.rs` → `#[tool] async fn echo(...)` on `BotticelliServer`
- [ ] `server_info.rs` → `#[tool] async fn server_info(...)` on `BotticelliServer`

#### Narrative creation tools
- [ ] The 8-tool session stack (`create_narrative_session`, `elicit_metadata`, `elicit_act`,
  `elicit_carousel`, `get_narrative_state`, `validate_narrative_session`,
  `apply_validation_fixes`, `finalize_narrative`) collapses to a **single**
  `#[tool] async fn create_narrative(...)` that calls
  `PartialNarrative::elicit(&ElicitServer::new(ctx.peer)).await`
- [ ] Delete `tools/elicitation/` directory entirely (all session tools replaced)
- [ ] Delete `tools/elicitation_primitives.rs` (primitive elicit tools replaced by framework)
- [ ] Delete `tools/narrative_creation.rs`, `tools/narrative_utils.rs` (merged into server)

#### Narrative manipulation tools (keep, convert to #[tool])
- [ ] `create_narrative.rs` → `#[tool] async fn save_narrative_toml(...)`
- [ ] `validate_narrative.rs` → `#[tool] async fn validate_narrative(...)`
- [ ] `save_narrative.rs` → `#[tool] async fn write_narrative(...)`
- [ ] `modify_narrative.rs` → `#[tool] async fn modify_narrative(...)`

#### Narrative execution tools (feature-gated on `execution`)
- [ ] `generate.rs` → `#[tool] async fn generate(...)` on `BotticelliServer`
- [ ] `execute_act.rs` → `#[tool] async fn execute_act(...)` on `BotticelliServer`
- [ ] `execute_narrative.rs` → `#[tool] async fn execute_narrative(...)` on `BotticelliServer`

#### LLM provider tools (feature-gated)
- [ ] `generate_llm.rs` — convert `GenerateAnthropicTool`, `GenerateGeminiTool`, etc. to
  `#[tool]` methods (each behind its `#[cfg(feature = "...")]`)

#### Scene management tools
- [ ] `scene.rs` — convert `CreateSceneTool`, `ListScenesTool`, `UpdateSceneTool`,
  `DeleteSceneTool` to `#[tool]` methods; add `#[derive(Elicit, JsonSchema, Serialize,
  Deserialize)]` to scene input structs

#### Discord tools (feature-gated on `discord`)
- [ ] `discord.rs` / `discord_workflow.rs` / `social.rs` / `bot_commands.rs` — convert to
  `#[tool]` methods on `BotticelliServer`; add required derives to all input structs

#### Database tools (feature-gated on `database`)
- [ ] `database.rs` → `#[tool] async fn query_content(...)` on `BotticelliServer`

#### Metrics / prometheus (if keeping)
- [ ] `export_metrics.rs` / `prometheus.rs` — convert or remove

#### Sampling infrastructure
- [ ] `sampling.rs` / `sampling_session_manager.rs` — evaluate: keep as internal helpers or
  replace with elicitation approach; these are not MCP tools directly

- [ ] Delete `McpTool` trait from `tools/mod.rs`
- [ ] Delete `ToolRegistry` struct from `tools/mod.rs`
- [ ] Delete `ToolRegistry::Default` impl
- [ ] Update `tools/mod.rs` to export only surviving types
- [ ] `just check -p botticelli_mcp` passes

---

### Phase 6 — InProcTransport rebuild on rmcp

- [ ] Evaluate whether rmcp provides an in-process transport natively
- [ ] If not: rebuild `transport/in_proc.rs` using rmcp's channel transport primitives
  (rmcp has `transport::io::TokioChildProcess` and similar — check for pair())
- [ ] Update `transport/mod.rs` exports
- [ ] All in-proc transport tests in `tests/in_proc_transport_test.rs` still pass

---

### Phase 7 — Single binary with clap subcommands

Pattern reference: `strictly_games/crates/strictly_games/src/main.rs`

- [ ] Create `src/bin/botticelli-mcp.rs` with clap:
  ```
  botticelli-mcp serve          # stdio transport
  botticelli-mcp http [--host] [--port]   # streamable-http transport
  ```
- [ ] stdio: `rmcp::service::serve_server(BotticelliServer::new(), rmcp::transport::stdio())`
- [ ] http: `StreamableHttpService` + axum Router (matching strictly_games pattern)
- [ ] Delete `src/bin/botticelli-mcp-http.rs`
- [ ] Delete `src/bin/botticelli-mcp-pmcp.rs`
- [ ] Delete `src/bin/botticelli-mcp-pmcp-http.rs`
- [ ] Update `[[bin]]` entries in `Cargo.toml` (single entry, no `required-features`)
- [ ] `just check -p botticelli_mcp` passes with all feature combinations

---

### Phase 8 — ToolRegistry in botticelli_interface

- [ ] Audit `botticelli_interface` for any remaining `McpTool` / old `ToolRegistry`
  references
- [ ] `ToolDefinition` in `botticelli_core` stays — it is the LLM tool-calling concept,
  not the MCP server concept
- [ ] If `ElicitationRegistryOperations<T>` in `botticelli_interface/src/registry_traits.rs`
  can be simplified or aligned to elicitation's plugin model, do so
- [ ] Remove any interface traits that were purely bridging to the old pmcp stack
- [ ] `just check` (full workspace) passes

---

### Phase 9 — Test suite cleanup

- [ ] Delete or rewrite tests that used pmcp-specific test helpers:
  - `tests/pmcp_server_test.rs`
  - `tests/pmcp_http_test.rs`
  - `tests/in_proc_transport_test.rs` (rebuild for rmcp transport)
- [ ] Rewrite `tests/elicitation_integration_test.rs` using `ElicitClient` + rmcp in-proc
- [ ] Keep all non-pmcp tests: `tests/conversation_test.rs`, `tests/database_tool_test.rs`,
  `tests/discord_tools_test.rs`, `tests/execution_test.rs`, etc.
- [ ] `just test-package botticelli_mcp` passes

---

### Phase 10 — Final verification

- [ ] `just check-all` (clippy + fmt + tests) passes with zero warnings
- [ ] `just audit` passes
- [ ] `just check-features` passes (all feature combinations)
- [ ] `markdownlint-cli2 "**/*.md"` passes (after updating this doc)
- [ ] Single binary smoke test: `cargo run -p botticelli_mcp -- serve` starts and responds
  to MCP initialize
- [ ] HTTP smoke test: `cargo run -p botticelli_mcp -- http` starts and responds on the
  default port
- [ ] PR to main

---

## File Deletion List

These files are removed entirely during the migration:

```
crates/botticelli_mcp/src/pmcp_adapters.rs
crates/botticelli_mcp/src/pmcp_middleware.rs
crates/botticelli_mcp/src/pmcp_server.rs
crates/botticelli_mcp/src/pmcp_http_server.rs
crates/botticelli_mcp/src/server.rs               (old BotticelliRouter)
crates/botticelli_mcp/src/dialog_resource.rs
crates/botticelli_mcp/src/elicitation/dialog.rs
crates/botticelli_mcp/src/elicitation/elicitor.rs
crates/botticelli_mcp/src/elicitation/partial.rs  (moved to botticelli_narrative)
crates/botticelli_mcp/src/tools/elicitation/      (entire directory)
crates/botticelli_mcp/src/tools/elicitation_primitives.rs
crates/botticelli_mcp/src/tools/narrative_creation.rs
crates/botticelli_mcp/src/bin/botticelli-mcp-http.rs
crates/botticelli_mcp/src/bin/botticelli-mcp-pmcp.rs
crates/botticelli_mcp/src/bin/botticelli-mcp-pmcp-http.rs
crates/botticelli_mcp/tests/pmcp_server_test.rs
crates/botticelli_mcp/tests/pmcp_http_test.rs
```

---

## Key Reference Files

- `rmcp` server pattern: `/home/erik/repos/strictly_games/crates/strictly_server/src/server.rs`
- HTTP transport: `/home/erik/repos/strictly_games/crates/strictly_games/src/main.rs`
- `ElicitPlugin` pattern: `/home/erik/repos/elicitation/crates/elicit_server/src/emit_plugin.rs`
- `ElicitServer` usage: `/home/erik/repos/elicitation/crates/elicitation/src/server.rs`
- `PluginRegistry`: `/home/erik/repos/elicitation/crates/elicitation/src/plugin_registry.rs`

---

## Derive Rule Reference

| Goal | Required derives |
|------|-----------------|
| Tool input type (MCP) | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Tool output type | `Serialize, Deserialize, schemars::JsonSchema` |
| Domain type user can elicit | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Enum for Select paradigm | `Elicit, schemars::JsonSchema, Serialize, Deserialize` |
| Internal-only type | existing derives unchanged |
