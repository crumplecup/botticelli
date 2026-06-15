//! `BotController` — the top-level event loop driving the screen state machine.
//!
//! The controller owns two kinds of screen state:
//! - `chat`: a persistent [`ChatScreen`] that survives navigation away and back.
//! - `current`: which screen is currently displayed — either the chat screen or
//!   some other `Box<dyn BotScreen>`.
//!
//! Chat state (history, waiting flag, model status) lives exclusively in `chat`.
//! All I/O and shared-state mutations live here; screens are pure.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;
use tracing::{debug, info, instrument};

#[cfg(feature = "cli")]
use botticelli_core::{GenerateRequest, Input, Message, Output, Role};
#[cfg(feature = "cli")]
use botticelli_interface::{BotStorage, BotticelliDriver};
#[cfg(feature = "cli")]
use std::sync::Arc;
#[cfg(feature = "cli")]
use tokio::sync::oneshot;

use crate::context::BotScreenContext;
use crate::contracts::{render_resize_prompt, verified_draw};
use crate::error::TuiResult;
use crate::screen::{BotScreen, BotTransition};
use crate::screens::{
    BotStatusScreen, ChatScreen, DatabaseBrowserScreen, LogViewerScreen, NarrativeBrowserScreen,
    NarrativeEditorScreen, ScheduleScreen, SettingsScreen,
};

/// Which screen is currently rendered.
pub enum CurrentScreen {
    /// The persistent chat screen.
    Chat,
    /// Any other screen (bots, narratives, settings, …).
    Other(Box<dyn BotScreen>),
}

/// Top-level controller for the botticelli TUI.
#[derive(derive_getters::Getters)]
pub struct BotController {
    /// Which screen is currently rendered.
    current: CurrentScreen,
    /// Persistent chat screen — survives navigation away and back.
    chat: ChatScreen,
    /// Shared context passed to screens on each key event.
    ctx: BotScreenContext,
    /// The active LLM driver, once loaded.
    #[cfg(feature = "cli")]
    driver: Option<Arc<dyn BotticelliDriver>>,
    #[cfg(feature = "cli")]
    #[getter(skip)] // Consuming the receiver would break the poll loop.
    driver_rx: Option<oneshot::Receiver<crate::error::TuiResult<Arc<dyn BotticelliDriver>>>>,
    #[cfg(feature = "cli")]
    #[getter(skip)] // Consuming the receiver would break the poll loop.
    chat_rx: Option<oneshot::Receiver<String>>,
    /// Prompt queued while the driver was loading; dispatched automatically on arrival.
    #[cfg(feature = "cli")]
    pending_prompt: Option<String>,
}

impl BotController {
    /// Construct the controller, starting on the Bots screen.
    #[instrument(skip(ctx))]
    pub fn new(ctx: BotScreenContext) -> Self {
        info!("BotController starting on Bots screen");
        Self {
            current: CurrentScreen::Other(Box::new(BotStatusScreen::new())),
            chat: ChatScreen::new(),
            ctx,
            #[cfg(feature = "cli")]
            driver: None,
            #[cfg(feature = "cli")]
            driver_rx: None,
            #[cfg(feature = "cli")]
            chat_rx: None,
            #[cfg(feature = "cli")]
            pending_prompt: None,
        }
    }

    /// Supply a pre-built LLM driver directly.
    ///
    /// Use this when the driver is already available — e.g. in tests or for
    /// providers that don't require a slow load step.
    #[cfg(feature = "cli")]
    pub fn with_driver(mut self, driver: Arc<dyn BotticelliDriver>) -> Self {
        self.chat.on_model_ready();
        self.driver = Some(driver);
        self
    }

    /// Supply a receiver for an LLM driver being loaded in the background.
    ///
    /// The TUI opens immediately; once the driver is ready it is wired in
    /// automatically and chat becomes available.
    #[cfg(feature = "cli")]
    pub fn with_driver_receiver(
        mut self,
        rx: oneshot::Receiver<crate::error::TuiResult<Arc<dyn BotticelliDriver>>>,
    ) -> Self {
        self.driver_rx = Some(rx);
        self
    }

    /// Apply one transition and await any spawned background task to completion.
    ///
    /// Drives the controller without a terminal — useful for integration tests.
    pub async fn drive(&mut self, transition: BotTransition) -> TuiResult<()> {
        self.apply(transition).await?;
        #[cfg(feature = "cli")]
        if let Some(rx) = self.chat_rx.take() {
            let text = rx
                .await
                .unwrap_or_else(|_| "Generate task dropped".to_string());
            self.chat.on_chat_response(text);
        }
        Ok(())
    }

    /// Run the event loop until the user quits.
    #[instrument(skip(self, terminal))]
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> TuiResult<()> {
        loop {
            // ── poll background model load ───────────────────────────────────
            #[cfg(feature = "cli")]
            if let Some(rx) = self.driver_rx.as_mut() {
                match rx.try_recv() {
                    Ok(Ok(driver)) => {
                        info!(
                            provider = driver.provider_name(),
                            model = driver.model_name(),
                            "LLM driver ready"
                        );
                        self.driver = Some(driver);
                        self.driver_rx = None;
                        self.chat.on_model_ready();
                        if let Some(prompt) = self.pending_prompt.take() {
                            self.dispatch_generate(prompt);
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!(error = %e, "Failed to load LLM driver");
                        self.chat.on_model_failed(e.to_string());
                        self.driver_rx = None;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        tracing::error!("LLM driver task dropped without sending");
                        self.chat
                            .on_model_failed("Driver task exited unexpectedly".to_string());
                        self.driver_rx = None;
                    }
                }
            }

            // ── poll background chat generate ────────────────────────────────
            #[cfg(feature = "cli")]
            if let Some(rx) = self.chat_rx.as_mut() {
                match rx.try_recv() {
                    Ok(text) => {
                        info!(chars = text.len(), "Chat response delivered to screen");
                        self.chat.on_chat_response(text);
                        self.chat_rx = None;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        tracing::error!("Chat generate task dropped without sending");
                        self.chat
                            .on_chat_response("Generate task exited unexpectedly".to_string());
                        self.chat_rx = None;
                    }
                }
            }

            // ── render ──────────────────────────────────────────────────────
            let root = match &self.current {
                CurrentScreen::Chat => self.chat.to_tui_node(),
                CurrentScreen::Other(s) => s.to_tui_node(),
            };

            terminal.draw(|frame| {
                let area = frame.area();
                if let Err(e) = verified_draw(frame, area, &root) {
                    render_resize_prompt(frame, &e);
                }
            })?;

            // ── input ───────────────────────────────────────────────────────
            if !event::poll(Duration::from_millis(16))? {
                continue;
            }

            let ev = event::read()?;

            // Global Ctrl-C / Ctrl-Q exit guard
            if let Event::Key(key) = &ev
                && is_global_quit(key)
            {
                info!("Global quit received");
                break;
            }

            if let Event::Key(key) = ev {
                debug!(code = ?key.code, "key event");
                let transition = match &mut self.current {
                    CurrentScreen::Chat => self.chat.handle_key(key, &self.ctx),
                    CurrentScreen::Other(s) => s.handle_key(key, &self.ctx),
                };
                if self.apply(transition).await? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Spawn a background generate task for `content` and wire the result back.
    ///
    /// Caller must ensure `self.driver` is `Some`.
    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    fn dispatch_generate(&mut self, content: String) {
        let driver = Arc::clone(self.driver.as_ref().expect("driver must be Some"));
        info!(provider = driver.provider_name(), model = driver.model_name(), prompt = %content, "Dispatching generate");
        let (tx, rx) = oneshot::channel::<String>();
        self.chat_rx = Some(rx);
        self.chat.on_chat_replying();
        tokio::spawn(async move {
            let msg = Message::new(Role::User, vec![Input::Text(content)]);
            let req = GenerateRequest::new(vec![msg]);
            let text = match driver.generate(&req).await {
                Ok(resp) => {
                    let t = resp
                        .outputs()
                        .iter()
                        .filter_map(|o| {
                            if let Output::Text(t) = o {
                                Some(t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    tracing::info!(chars = t.len(), "Generate task completed");
                    t
                }
                Err(e) => {
                    tracing::error!(error = %e, "Generate task failed");
                    format!("Error: {e}")
                }
            };
            tx.send(text).ok();
        });
    }

    /// Apply a `BotTransition`. Returns `true` if the app should exit.
    #[instrument(skip(self))]
    async fn apply(&mut self, transition: BotTransition) -> TuiResult<bool> {
        match transition {
            BotTransition::Stay => {}
            BotTransition::Quit => return Ok(true),

            BotTransition::GoToBots => {
                self.current = CurrentScreen::Other(Box::new(BotStatusScreen::new()));
            }
            BotTransition::GoToChat => {
                #[cfg(feature = "cli")]
                if self.driver.is_some()
                    && matches!(
                        self.chat.model_status(),
                        crate::screens::ModelStatus::Loading
                    )
                {
                    self.chat.on_model_ready();
                }
                self.current = CurrentScreen::Chat;
            }

            BotTransition::ChatSend { content } => {
                #[cfg(feature = "cli")]
                if self.driver.is_some() {
                    self.dispatch_generate(content);
                } else if self.driver_rx.is_some() {
                    // Queue the prompt — dispatched automatically when the driver is ready.
                    info!("Driver not yet ready; queuing prompt");
                    self.pending_prompt = Some(content);
                } else {
                    self.chat.on_chat_response(
                        "No LLM driver configured. Use --provider to select a backend.".to_string(),
                    );
                }
                #[cfg(not(feature = "cli"))]
                {
                    let _ = content;
                    self.chat
                        .on_chat_response("Build with --features cli to enable chat.".to_string());
                }
            }

            BotTransition::GoToNarratives => {
                let dir = self.ctx.narratives_dir.clone();
                self.current = CurrentScreen::Other(Box::new(NarrativeBrowserScreen::new(dir)));
            }
            BotTransition::GoToNarrativeEditor { path } => {
                info!(?path, "Opening narrative editor");
                let screen = match path {
                    Some(p) => NarrativeEditorScreen::open(p),
                    None => NarrativeEditorScreen::new_file(),
                };
                self.current = CurrentScreen::Other(Box::new(screen));
            }
            BotTransition::GoToDatabase => {
                #[cfg(feature = "cli")]
                let has_storage = self.ctx.storage.is_some();
                #[cfg(not(feature = "cli"))]
                let has_storage = false;
                self.current =
                    CurrentScreen::Other(Box::new(DatabaseBrowserScreen::new(has_storage)));
            }
            BotTransition::GoToSchedule => {
                #[cfg(feature = "cli")]
                let (task_rows, task_ids, has_storage) =
                    if let Some(ref storage) = self.ctx.storage {
                        let (rows, ids) = fetch_schedule_data(Arc::clone(storage)).await;
                        (rows, ids, true)
                    } else {
                        (vec![], vec![], false)
                    };
                #[cfg(not(feature = "cli"))]
                let (task_rows, task_ids, has_storage) = (vec![], vec![], false);
                self.current = CurrentScreen::Other(Box::new(ScheduleScreen::new(
                    task_rows, task_ids, has_storage,
                )));
            }
            BotTransition::LoadScheduleData => {
                #[cfg(feature = "cli")]
                if let Some(ref storage) = self.ctx.storage {
                    let (task_rows, task_ids) = fetch_schedule_data(Arc::clone(storage)).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_schedule_loaded(task_rows, task_ids);
                    }
                }
            }
            BotTransition::LoadTaskExecutions { task_id } => {
                #[cfg(feature = "cli")]
                if let Some(ref storage) = self.ctx.storage {
                    let exec_rows =
                        fetch_task_executions(Arc::clone(storage), &task_id).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_task_executions_loaded(exec_rows);
                    }
                }
                #[cfg(not(feature = "cli"))]
                {
                    let _ = task_id;
                }
            }
            BotTransition::GoToLogViewer => {
                #[cfg(feature = "cli")]
                let lines = if let Some(ref path) = self.ctx.log_file {
                    read_log_lines(path).await
                } else {
                    vec![]
                };
                #[cfg(not(feature = "cli"))]
                let lines: Vec<String> = vec![];
                self.current =
                    CurrentScreen::Other(Box::new(LogViewerScreen::new(lines)));
            }
            BotTransition::LoadLogLines => {
                #[cfg(feature = "cli")]
                if let Some(ref path) = self.ctx.log_file {
                    let lines = read_log_lines(path).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_log_lines_loaded(lines);
                    }
                }
            }
            BotTransition::GoToSettings => {
                let narratives_dir = self
                    .ctx
                    .narratives_dir
                    .as_ref()
                    .map(|p| p.display().to_string());
                let log_file = self
                    .ctx
                    .log_file
                    .as_ref()
                    .map(|p| p.display().to_string());
                #[cfg(feature = "cli")]
                let storage_path = self
                    .ctx
                    .storage
                    .as_ref()
                    .map(|_| std::env::var("REDB_PATH").unwrap_or_else(|_| "./botticelli.redb".to_string()));
                #[cfg(not(feature = "cli"))]
                let storage_path: Option<String> = None;
                self.current = CurrentScreen::Other(Box::new(SettingsScreen::new(
                    narratives_dir,
                    log_file,
                    storage_path,
                )));
            }

            BotTransition::SaveSettings { rust_log } => {
                let content = format!("RUST_LOG={}\n", rust_log);
                info!(%rust_log, "Saving settings to botticelli-settings.env");
                if let Err(e) = tokio::fs::write("botticelli-settings.env", &content).await {
                    tracing::error!(error = %e, "Failed to write botticelli-settings.env");
                }
            }

            BotTransition::LoadDatabaseTable { table } => {
                #[cfg(feature = "cli")]
                if let Some(ref storage) = self.ctx.storage {
                    let storage = Arc::clone(storage);
                    let rows = fetch_table_rows(storage, &table).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_table_loaded(&table, rows);
                    }
                }
                #[cfg(not(feature = "cli"))]
                {
                    let _ = table;
                }
            }

            BotTransition::SaveNarrative { path, toml } => {
                info!(?path, bytes = toml.len(), "Saving narrative");
                tokio::fs::write(&path, &toml).await?;
            }

            #[cfg(feature = "cli")]
            BotTransition::StartBot(kind) => {
                info!(%kind, "StartBot requested (not yet wired to BotServer)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, true);
                }
            }
            #[cfg(feature = "cli")]
            BotTransition::StopBot(kind) => {
                info!(%kind, "StopBot requested (not yet wired to BotServer)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, false);
                }
            }
            #[cfg(feature = "cli")]
            BotTransition::RestartBot(kind) => {
                info!(%kind, "RestartBot (not yet wired to BotServer) — marking running");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, true);
                }
            }
        }
        Ok(false)
    }
}

fn is_global_quit(key: &KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Char('c') | KeyCode::Char('q')
            if key.modifiers.contains(KeyModifiers::CONTROL)
    )
}

/// Fetch actor state list and return parallel (display rows, task IDs) vecs.
#[cfg(feature = "cli")]
#[tracing::instrument(skip(storage))]
async fn fetch_schedule_data(storage: Arc<dyn BotStorage>) -> (Vec<String>, Vec<String>) {
    let states = storage.list_actor_states().await.unwrap_or_default();
    let rows = states
        .iter()
        .map(|r| {
            let ts: String = r.next_run.to_string().chars().take(19).collect();
            let paused = if r.is_paused { "PAUSED" } else { "active" };
            format!("{} | {} | {} | next: {}", paused, r.task_id, r.actor_name, ts)
        })
        .collect();
    let ids = states.into_iter().map(|r| r.task_id).collect();
    (rows, ids)
}

/// Fetch recent executions for a single task, pre-formatted as one-liners.
#[cfg(feature = "cli")]
#[tracing::instrument(skip(storage), fields(task_id))]
async fn fetch_task_executions(storage: Arc<dyn BotStorage>, task_id: &str) -> Vec<String> {
    storage
        .list_actor_executions(task_id, 30)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let ts: String = r.started_at.to_string().chars().take(19).collect();
            let ok = if r.success { "ok" } else { "FAIL" };
            format!(
                "{} {} | s={} f={}",
                ok, ts, r.skills_succeeded, r.skills_failed
            )
        })
        .collect()
}

/// Read the last 500 lines of the log file.
#[cfg(feature = "cli")]
#[tracing::instrument(skip_all)]
async fn read_log_lines(path: &std::path::Path) -> Vec<String> {
    tokio::fs::read_to_string(path)
        .await
        .unwrap_or_default()
        .lines()
        .rev()
        .take(500)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(String::from)
        .collect()
}

/// Fetch a page of pre-formatted row summaries from the named BotStorage table.
///
/// Each returned string is a single-line human-readable summary of one record.
#[cfg(feature = "cli")]
#[tracing::instrument(skip(storage), fields(table))]
async fn fetch_table_rows(storage: Arc<dyn BotStorage>, table: &str) -> Vec<String> {
    match table {
        "narrative_executions" => storage
            .list_narrative_executions(100)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| format!("{} | {} | {}", r.id, r.narrative_name, r.status))
            .collect(),
        "actor_states" => storage
            .list_actor_states()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| format!("{} | {} | paused={}", r.task_id, r.actor_name, r.is_paused))
            .collect(),
        "actor_executions" => storage
            .list_all_actor_executions(100)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| format!("{} | {} | success={}", r.id, r.actor_name, r.success))
            .collect(),
        "content" => storage
            .list_content("content", 100)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                let preview: String = r.content_json.to_string().chars().take(60).collect();
                format!("{} | {}", r.id, preview)
            })
            .collect(),
        "model_responses" => storage
            .list_model_responses(100)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| format!("{} | {} | {}", r.id, r.provider, r.model_name))
            .collect(),
        _ => vec![],
    }
}
