# botticelli_server Overhaul Plan

**Branch**: `dev`  
**Status**: 📋 Ready to Implement  
**Started**: 2026-06-11  

Replace the dead infrastructure in `botticelli_server` with a clean, backend-agnostic
design where the inference backend is selected at startup via a CLI flag. `mistral-rs`
(embedded GGUF inference) is the default for zero-dependency operation; Ollama is the
alternative for lighter binaries or multi-model setups.

---

## Architecture

```
botticelli-server --backend mistral --model-path ./models/llama-3.2.gguf
botticelli-server --backend ollama  --url http://localhost:11434 --model llama3.2
```

```
CLI (clap)
  │
  ├── Backend::Mistral ──► MistralDriver ─┐
  │                                       ├──► Arc<dyn BotticelliDriver>
  └── Backend::Ollama  ──► OllamaClient  ─┘
                                          │
                                    BotServer::new(driver)
                                          │
                              ┌───────────┼───────────┐
                         GenerationBot  CurationBot  PostingBot
                         (ractor actor, receives Arc<dyn BotticelliDriver>)
```

### Why mistral-rs as default

mistral-rs loads a GGUF model and runs it entirely within the process. Once loaded, all
inference goes through an internal mpsc channel to a background task managed by the library
— no external process needed, no port to manage, no health-check race on startup. A
single `cargo run -p botticelli-server -- --backend mistral --model-path ...` is the
entire deployment.

Ollama remains a first-class alternative: lighter binary (model stays out of process),
hot-swap models, multi-model serving. The bots don't care which is wired in.

### Isolation model

mistral-rs already runs the model in its own internal tokio task. The `get_sender()`
channel decouples the caller from the inference hot-path — a panic in model execution
surfaces as an error through the oneshot receiver, not a server crash. For startup,
GGUF loading is heavy (seconds, potentially GB of I/O); wrap it in `tokio::spawn` so
the main server task is not blocked.

---

## What Gets Deleted

The current crate has two concerns that conflict:

**Dead inference infrastructure** (designed for an external `botticelli_mistral` crate
that never materialized):
- `traits.rs` — `InferenceServer`, `ServerLauncher`, `ModelManager` traits
- `client.rs` / `ServerClient` — hand-rolled OpenAI-compat HTTP client (duplicates
  `botticelli_models::OllamaClient` exactly)
- `request.rs`, `response.rs`, `convert.rs` — OpenAI chat completion types only used
  by `ServerClient`
- `config.rs` / `ServerConfig` — URL + model config for the deleted HTTP client

**Hard-coded provider coupling in bots**:
- `use botticelli_models::GeminiClient` in all three bots
- `GeminiClient::new()` baked into `run_generation_cycle`, `run_curation_cycle`, etc.

**Stale Cargo.toml**:
- `axum = "0.7"` — workspace is 0.8, not going through workspace
- `botticelli_models = { ..., features = ["gemini"] }` — hard-coded backend
- Direct path deps that should go through workspace

---

## Migration Checklist

### Phase 1 — Workspace plumbing ✅

- [x] Add `mistralrs = { version = "0.8", default-features = false }` to workspace `Cargo.toml`
  - Version 0.8.1 (latest); CPU-only by default; hardware features (cuda, metal, mkl) opt-in
  - Bumped workspace `rust-version` from `"1.85"` → `"1.88"` (mistralrs requirement)
- [x] Fix `botticelli_server/Cargo.toml`:
  - All path deps moved to `workspace = true` form
  - `axum = "0.7"` → `axum.workspace = true` (workspace is 0.8)
  - `async-trait`, `reqwest`, `futures` moved to workspace
  - `cron = "0.12"` and `rand = "0.8"` kept as local deps (only used here)
  - `botticelli_models = { workspace = true, features = ["gemini"] }` kept for now (bots still use GeminiClient until Phase 4)
  - Added `mistralrs = { workspace = true, optional = true }` behind `mistral` feature
  - Added `ollama = ["botticelli_models/ollama"]` feature
  - `default` stays `["models"]` until Phase 2 delivers `MistralDriver`
- [x] `just check botticelli_server` passes — zero warnings, zero errors

---

### Phase 2 — MistralDriver: Arc<dyn BotticelliDriver> over mistral-rs ✅

New file: `botticelli_server/src/drivers/mistral.rs` (behind `#[cfg(feature = "mistral")]`)

```rust
/// Embedded GGUF model inference via mistral-rs.
///
/// The model loads once and runs in mistral-rs's internal task.
/// All inference calls go through a channel — non-blocking for the caller.
pub struct MistralDriver {
    pipeline: Arc<MistralRs>,
    model_id:  String,
}

impl MistralDriver {
    /// Load a GGUF model. Spawns loading in a background task so the
    /// caller is not blocked during multi-GB file I/O.
    pub async fn load(config: MistralConfig) -> ServerResult<Self> { ... }
}

impl BotticelliDriver for MistralDriver {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        // Build NormalRequest from GenerateRequest
        // Send via pipeline.get_sender()
        // Receive via oneshot, convert to GenerateResponse
    }
}
```

`MistralConfig`:
```rust
#[derive(Debug, Clone, derive_builder::Builder)]
pub struct MistralConfig {
    /// Path to a local .gguf file.
    pub model_path: PathBuf,
    /// Human-readable model identifier (used in responses).
    pub model_id: String,
    /// Number of concurrent inference slots (default: 1).
    #[builder(default = "1")]
    pub concurrency: usize,
}
```

- [x] Create `botticelli_server/src/drivers/mod.rs`
- [x] Implement `MistralDriver` + `MistralConfig` in `drivers/mistral.rs`
  - mistralrs 0.8.x uses `GgufModelBuilder::new(dir, files).build()` → `Model`
  - `Model::send_chat_request(TextMessages)` → `ChatCompletionResponse`
  - Model loading runs via `tokio::task::spawn_blocking` (prevents blocking async executor during GB-scale I/O)
  - Non-text inputs (images, audio) silently skipped — text-only model
- [x] Implement `BotticelliDriver` for `MistralDriver`
- [x] Implement `provider_name()` → `"mistral-rs"`, `model_name()` → `config.model_id`
- [x] `rate_limits()` returns `RateLimitConfig::unlimited("mistral-rs")`
- [x] `cargo check -p botticelli_server --features mistral` — zero warnings, zero errors
- [x] `just check botticelli_server` (default, no mistral) — still clean

---

### Phase 3 — OllamaDriver: wire botticelli_models::OllamaClient ✅

`botticelli_models::OllamaClient` already implements `BotticelliDriver + Streaming + TokenCounting`.
No new wrapper type needed — CLI uses `OllamaClient::new_with_url(model, url)` directly.

- [x] `ollama = ["dep:botticelli_models", "botticelli_models/ollama"]` in botticelli_server features
- [x] `OllamaClient` confirmed `BotticelliDriver + Send + Sync + 'static`
- [x] `cargo check -p botticelli_server --features ollama` passes — zero warnings

---

### Phase 4 — Refactor bots to accept Arc<dyn BotticelliDriver> ✅

All three bots currently call `GeminiClient::new()` directly. Replace with injected driver.

**GenerationBot** (generation.rs):
```rust
pub struct GenerationBot {
    driver: Arc<dyn BotticelliDriver>,
    args:   GenerationBotArgs,
}

pub struct GenerationBotArgs {
    pub interval:       Duration,
    pub narrative_path: PathBuf,
    pub narrative_name: String,
    // Remove: no API key, no model field — driver owns that
}
```

Same pattern for `CurationBot` and `PostingBot`.

**NarrativeExecutor** currently takes a concrete `GeminiClient`. Confirm it accepts
`impl BotticelliDriver` or `Arc<dyn BotticelliDriver>` — if not, that's a
`botticelli_narrative` fix needed first (note in Phase 4 if blocked).

- [x] `GenerationBotArgs`, `CurationBotArgs`, `PostingBotArgs` — added `#[derive(derive_new::new)]`; removed model/key fields
- [x] `GenerationBot::new(driver, args)`, `CurationBot::new(driver, args)`, `PostingBot::new(driver, args)`
- [x] `GeminiClient::new()` removed; replaced with `NarrativeExecutor::new(self.driver.clone())`
  - `NarrativeExecutor<D: BotticelliDriver>` is generic — accepts `Arc<dyn BotticelliDriver>` via blanket impl
- [x] `BotServer::new(driver: Arc<dyn BotticelliDriver>)` — stores driver, passes to all bots in `start()`
- [x] `Default` impl removed from `BotServer` (no-arg construction no longer meaningful)
- [x] Duplicate `#[async_trait::async_trait]` in CurationBot removed
- [x] Struct literals in `BotServer::start()` replaced with `BotXArgs::new(...)` constructors
- [x] `botticelli_models` made `optional = true` (gemini feature no longer forced)
- [x] `botticelli_database`, `reqwest`, `futures` removed from deps (no longer used)
- [x] `just check botticelli_server` passes, zero warnings

---

### Phase 5 — Delete dead infrastructure ✅

Once Phase 4 passes, nothing uses the old HTTP client layer.

Files to delete:
```
crates/botticelli_server/src/traits.rs          # InferenceServer, ServerLauncher, ModelManager
crates/botticelli_server/src/client.rs          # ServerClient
crates/botticelli_server/src/request.rs         # ChatCompletionRequest
crates/botticelli_server/src/response.rs        # ChatCompletionResponse, chunks
crates/botticelli_server/src/convert.rs         # OpenAI ↔ botticelli conversion
crates/botticelli_server/src/config.rs          # ServerConfig (HTTP client config)
```

Remove from `lib.rs`:
- `pub use traits::...`
- `pub use client::ServerClient`
- `pub use config::{ServerConfig, ServerConfigBuilder}`
- `pub use request::...`
- `pub use response::...`

- [x] Deleted: `traits.rs`, `client.rs`, `request.rs`, `response.rs`, `convert.rs`, `config.rs`
- [x] Removed corresponding `mod` and `pub use` from `lib.rs`
- [x] `just check` (full workspace) — zero warnings, zero errors
- Note: tests deferred to Phase 7

---

### Phase 6 — Binary: clap CLI with backend selection ✅

New file: `crates/botticelli_server/src/bin/botticelli-server.rs`

```
botticelli-server --backend mistral --model-path ./models/llama-3.2.gguf [--model-id llama-3.2]
botticelli-server --backend ollama  [--url http://localhost:11434] --model llama3.2
```

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run with embedded mistral-rs inference (default, self-contained).
    Mistral {
        #[arg(long)]
        model_path: PathBuf,
        #[arg(long, default_value = "local-model")]
        model_id: String,
    },
    /// Run with Ollama as the inference backend.
    Ollama {
        #[arg(long, default_value = "http://localhost:11434")]
        url: String,
        #[arg(long)]
        model: String,
    },
}
```

At startup:
```rust
let driver: Arc<dyn BotticelliDriver> = match cli.command {
    Command::Mistral { model_path, model_id } => {
        let config = MistralConfig::builder().model_path(model_path).model_id(model_id).build()?;
        Arc::new(MistralDriver::load(config).await?)
    }
    Command::Ollama { url, model } => {
        Arc::new(OllamaClient::new(url, model)?)
    }
};
BotServer::new(driver, bot_config).start().await
```

- [x] `clap.workspace = true` added to `botticelli_server/Cargo.toml`
- [x] `src/bin/botticelli-server.rs` with `Cli` (global interval flags) + `Backend` subcommands
- [x] `Backend::Mistral { model_path, model_id }` feature-gated behind `#[cfg(feature = "mistral")]`
  - `--model-path` splits into directory + filename for `GgufModelBuilder::new(dir, files)`
  - `--model-id` defaults to filename minus `.gguf` extension
- [x] `Backend::Ollama { url, model }` feature-gated behind `#[cfg(feature = "ollama")]`
- [x] `compile_error!` guard for builds with no backend feature enabled
- [x] `cfg`-gated imports to suppress unused warnings in single-feature builds
- [x] All feature combinations compile — zero warnings, zero errors
- [ ] Manual smoke test: `cargo run -p botticelli_server --features ollama -- ollama --model llama3.2`
  (Ollama installed; Mistral requires local GGUF)

---

### Phase 7 — Tests

- [ ] `tests/driver_injection_test.rs` — construct `BotServer` with a mock `BotticelliDriver`,
  verify bots accept and invoke it (no actual inference needed)
- [ ] `tests/bot_lifecycle_test.rs` — start a bot, send a message, receive lifecycle events
- [ ] Gate Mistral tests behind `#[cfg_attr(not(feature = "mistral"), ignore)]`
- [ ] Gate Ollama tests behind `#[cfg_attr(not(feature = "ollama"), ignore)]` and
  `#[cfg_attr(not(feature = "api"), ignore)]` (requires running Ollama)
- [ ] `just test-all botticelli_server` passes, zero warnings

---

### Phase 8 — Final verification

- [ ] `just check-all` (full workspace) — zero warnings, zero errors
- [ ] `just audit` passes
- [ ] `just check-features botticelli_server` — all feature combinations build
- [ ] Update `PLANNING_INDEX.md`
- [ ] PR to main

---

## Feature Flag Design

```toml
[features]
default  = ["mistral"]          # Self-contained binary out of the box
mistral  = ["dep:mistralrs"]    # Embedded GGUF inference
ollama   = ["botticelli_models/ollama"]   # External Ollama server
```

`mistral` and `ollama` can both be compiled in simultaneously — the CLI flag
picks at runtime. The binary does a compile-time guard:

```rust
#[cfg(not(any(feature = "mistral", feature = "ollama")))]
compile_error!("botticelli_server requires at least one backend feature: mistral or ollama");
```

---

## NarrativeExecutor Dependency

`NarrativeExecutor` in `botticelli_narrative` currently takes a concrete `GeminiClient`
(or `impl BotticelliDriver` — needs verification). If it only accepts concrete types,
Phase 4 will surface this. Fix plan: change `NarrativeExecutor::new` to accept
`Arc<dyn BotticelliDriver>` and update `botticelli_narrative` before the bot refactor.
This is a prerequisite for Phase 4, not a blocker for Phases 1–3.

---

## Key Reference Files

- mistral-rs crate: https://crates.io/crates/mistralrs (check current version at impl time)
- mistral-rs Rust API examples: `mistralrs/examples/` in the mistral-rs repo
- `botticelli_models/src/ollama.rs` — existing `OllamaClient: BotticelliDriver`
- `botticelli_interface/src/traits.rs` — `BotticelliDriver` trait definition
- `strictly_games/crates/strictly_games/src/main.rs` — clap subcommand pattern reference
