//! `BotController` — the top-level event loop driving the screen state machine.
//!
//! The controller owns two kinds of screen state:
//! - `chat`: a persistent [`ChatScreen`] that survives navigation away and back.
//! - `current`: which screen is currently displayed — either the chat screen or
//!   some other `Box<dyn BotScreen>`.
//!
//! Chat state (history, waiting flag, model status) lives exclusively in `chat`.
//! All I/O and shared-state mutations live here; screens are pure.

use botticelli_bot::{BotConfigGenerator, UserBotConfig};
use botticelli_core::{GenerateRequest, Input, Message, Output, Role};
use botticelli_interface::{BotStorage, BotticelliDriver, StreamChunk};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use elicitation::Elicitation;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, instrument, warn};

use crate::context::BotScreenContext;
use crate::contracts::{render_resize_prompt, verified_draw};
use crate::error::TuiResult;
use crate::screen::{BotScreen, BotTransition};
use crate::screens::{
    BotStatusScreen, ChatScreen, DatabaseBrowserScreen, LogViewerScreen, NarrativeBrowserScreen,
    NarrativeEditorScreen, NarrativeWizardScreen, ScheduleScreen, SettingsScreen,
};

/// Result delivered by a background wizard-agent task.
enum WizardAgentResult {
    /// One field answered — advance the wizard one step.
    OneField(String),
    /// All remaining fields answered via per-field generation.
    AllFields(Vec<String>),
    /// Agent completed the narrative in one JSON shot; jump straight to editor.
    JsonComplete {
        file: Box<botticelli_narrative::TomlNarrativeFile>,
        path: Option<std::path::PathBuf>,
    },
}

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
    driver: Option<Arc<dyn BotticelliDriver>>,
    #[getter(skip)] // Consuming the receiver would break the poll loop.
    driver_rx: Option<oneshot::Receiver<crate::error::TuiResult<Arc<dyn BotticelliDriver>>>>,
    #[getter(skip)] // Consuming the receiver would break the poll loop.
    chat_rx: Option<mpsc::UnboundedReceiver<StreamChunk>>,
    /// Prompt queued while the driver was loading; dispatched automatically on arrival.
    pending_prompt: Option<String>,
    /// Receiver for a background wizard-agent task.
    #[getter(skip)]
    wizard_rx: Option<oneshot::Receiver<WizardAgentResult>>,
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
            driver: None,
            driver_rx: None,
            chat_rx: None,
            pending_prompt: None,
            wizard_rx: None,
        }
    }

    /// Supply a pre-built LLM driver directly.
    ///
    /// Use this when the driver is already available — e.g. in tests or for
    /// providers that don't require a slow load step.
    pub fn with_driver(mut self, driver: Arc<dyn BotticelliDriver>) -> Self {
        self.chat.on_model_ready();
        self.driver = Some(driver);
        self
    }

    /// Supply a receiver for an LLM driver being loaded in the background.
    ///
    /// The TUI opens immediately; once the driver is ready it is wired in
    /// automatically and chat becomes available.
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
        if let Some(mut rx) = self.chat_rx.take() {
            while let Some(chunk) = rx.recv().await {
                if *chunk.is_final() {
                    break;
                } else if let botticelli_core::Output::Text(t) = chunk.content() {
                    if *chunk.is_thinking() {
                        self.chat.on_thinking_chunk(t.clone());
                    } else {
                        self.chat.on_content_chunk(t.clone());
                    }
                }
            }
            self.chat.on_stream_done();
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
            if let Some(rx) = self.chat_rx.as_mut() {
                let mut done = false;
                loop {
                    match rx.try_recv() {
                        Ok(chunk) => {
                            debug!(
                                is_final = chunk.is_final(),
                                is_thinking = chunk.is_thinking(),
                                "Render loop received chunk from chat_rx"
                            );
                            if *chunk.is_final() {
                                info!("Chat stream finished");
                                self.chat.on_stream_done();
                                done = true;
                                break;
                            }
                            if let Output::Text(t) = chunk.content() {
                                if *chunk.is_thinking() {
                                    self.chat.on_thinking_chunk(t.clone());
                                } else {
                                    self.chat.on_content_chunk(t.clone());
                                }
                            } else {
                                debug!("Non-text chunk content ignored in render loop");
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            tracing::error!("Chat generate task dropped channel");
                            self.chat.on_stream_done();
                            done = true;
                            break;
                        }
                    }
                }
                if done {
                    self.chat_rx = None;
                }
            }

            // ── poll background wizard-agent task ────────────────────────────
            if let Some(rx) = self.wizard_rx.as_mut() {
                match rx.try_recv() {
                    Ok(result) => {
                        self.wizard_rx = None;
                        match result {
                            WizardAgentResult::OneField(answer) => {
                                if let CurrentScreen::Other(s) = &mut self.current {
                                    s.on_wizard_field_filled(answer);
                                }
                            }
                            WizardAgentResult::AllFields(answers) => {
                                if let CurrentScreen::Other(s) = &mut self.current {
                                    s.on_wizard_fields_filled(answers);
                                }
                            }
                            WizardAgentResult::JsonComplete { file, path } => {
                                let raw = toml::to_string_pretty(&*file).unwrap_or_default();
                                self.current = CurrentScreen::Other(Box::new(
                                    NarrativeEditorScreen::from_parsed(path, *file, raw),
                                ));
                            }
                        }
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        tracing::error!("Wizard agent task dropped without sending");
                        if let CurrentScreen::Other(s) = &mut self.current {
                            s.on_wizard_field_filled(
                                "(agent task exited unexpectedly)".to_string(),
                            );
                        }
                        self.wizard_rx = None;
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

                // GoToBotWizard is handled inline here (not in apply()) because it
                // needs the terminal handle to suspend ratatui rendering while
                // TuiCommunicator owns stdin for the elicitation session.
                if let BotTransition::GoToBotWizard = transition {
                    terminal.clear()?;
                    use elicitation::{ElicitCommunicator, Generator};
                    let comm = elicit_ratatui::TuiCommunicator::new()
                        .with_style::<String, elicitation::StringStyle>(
                            elicitation::StringStyle::Human,
                        );
                    match BotConfigGenerator::elicit(&comm).await {
                        Ok(generator) => {
                            let config = generator.generate();
                            info!(name = %config.name, "Bot wizard complete — saving user bot config");
                            save_user_bot_config(&config).await;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Bot wizard cancelled or failed");
                        }
                    }
                    terminal.clear()?;
                    let mut screen = BotStatusScreen::new();
                    screen.on_user_bots_loaded(load_user_bot_names().await);
                    self.current = CurrentScreen::Other(Box::new(screen));
                    continue;
                }

                if self.apply(transition).await? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Spawn a background generate task for `content` and wire the result back.
    #[instrument(skip(self))]
    fn dispatch_generate(&mut self, content: String) {
        let driver = Arc::clone(self.driver.as_ref().expect("driver must be Some"));
        info!(
            provider = driver.provider_name(),
            model = driver.model_name(),
            "Dispatching generate"
        );
        let (tx, rx) = mpsc::unbounded_channel::<StreamChunk>();
        self.chat_rx = Some(rx);
        self.chat.on_chat_replying();
        tokio::spawn(async move {
            use futures::StreamExt;
            let msg = Message::new(Role::User, vec![Input::Text(content)]);
            let req = GenerateRequest::new(vec![msg]);

            match driver.stream_generate(&req).await {
                Ok(Some(mut stream)) => {
                    info!(
                        provider = driver.provider_name(),
                        model = driver.model_name(),
                        "Streaming path open — forwarding chunks from driver"
                    );
                    let mut chunk_count = 0usize;
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(chunk) => {
                                chunk_count += 1;
                                debug!(
                                    chunk_count,
                                    is_final = chunk.is_final(),
                                    is_thinking = chunk.is_thinking(),
                                    "Forwarding chunk to render loop"
                                );
                                if tx.send(chunk).is_err() {
                                    warn!(
                                        chunk_count,
                                        "Render loop receiver dropped — aborting stream"
                                    );
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, chunk_count, "Stream chunk error");
                                let _ = tx.send(StreamChunk::new(
                                    Output::Text(format!("Error: {e}")),
                                    true,
                                ));
                                return;
                            }
                        }
                    }
                    info!(chunk_count, "Driver stream exhausted — sending sentinel");
                    let _ = tx.send(StreamChunk::new(Output::Text(String::new()), true));
                }
                Ok(None) => {
                    tracing::warn!(
                        provider = driver.provider_name(),
                        model = driver.model_name(),
                        "stream_generate returned None — falling back to blocking generate"
                    );
                    let text = match driver.generate(&req).await {
                        Ok(resp) => {
                            let t: String = resp
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
                            tracing::info!(chars = t.len(), "Non-streaming generate completed");
                            t
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Generate failed");
                            format!("Error: {e}")
                        }
                    };
                    let _ = tx.send(StreamChunk::new(Output::Text(text), false));
                    let _ = tx.send(StreamChunk::new(Output::Text(String::new()), true));
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        provider = driver.provider_name(),
                        model = driver.model_name(),
                        "stream_generate failed — falling back to blocking generate"
                    );
                    let text = match driver.generate(&req).await {
                        Ok(resp) => {
                            let t: String = resp
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
                            tracing::info!(chars = t.len(), "Non-streaming generate completed");
                            t
                        }
                        Err(e2) => {
                            tracing::error!(error = %e2, "Generate fallback also failed");
                            format!("Error: {e2}")
                        }
                    };
                    let _ = tx.send(StreamChunk::new(Output::Text(text), false));
                    let _ = tx.send(StreamChunk::new(Output::Text(String::new()), true));
                }
            }
        });
    }

    /// Spawn a background task asking the agent for one wizard field answer.
    #[instrument(skip(self))]
    fn dispatch_wizard_next(&mut self, prompt: String) {
        let Some(driver) = self.driver.as_ref().map(Arc::clone) else {
            if let CurrentScreen::Other(s) = &mut self.current {
                s.on_wizard_field_filled("(no agent configured)".to_string());
            }
            return;
        };
        let (tx, rx) = oneshot::channel::<WizardAgentResult>();
        self.wizard_rx = Some(rx);
        tokio::spawn(async move {
            let system = "You are filling in a narrative configuration form. \
                Answer each question with only the value requested — no explanation, no preamble.";
            let msg_system = Message::new(Role::User, vec![Input::Text(system.to_string())]);
            let msg_prompt = Message::new(Role::User, vec![Input::Text(prompt)]);
            let req = GenerateRequest::new(vec![msg_system, msg_prompt]);
            let answer = match driver.generate(&req).await {
                Ok(resp) => resp
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
                    .join(""),
                Err(e) => {
                    tracing::error!(error = %e, "Wizard next-field generate failed");
                    format!("(agent error: {e})")
                }
            };
            tx.send(WizardAgentResult::OneField(answer.trim().to_string()))
                .ok();
        });
    }

    /// Spawn a background task asking the agent to fill all remaining wizard fields.
    ///
    /// Tries one-shot JSON first; falls back to per-field generation on failure.
    #[instrument(skip(self, remaining_prompts, json_schema))]
    fn dispatch_wizard_all(
        &mut self,
        remaining_prompts: Vec<String>,
        json_schema: String,
        path: Option<std::path::PathBuf>,
    ) {
        let Some(driver) = self.driver.as_ref().map(Arc::clone) else {
            let placeholders: Vec<String> = remaining_prompts
                .iter()
                .map(|_| "(no agent)".to_string())
                .collect();
            if let CurrentScreen::Other(s) = &mut self.current {
                s.on_wizard_fields_filled(placeholders);
            }
            return;
        };
        let (tx, rx) = oneshot::channel::<WizardAgentResult>();
        self.wizard_rx = Some(rx);
        tokio::spawn(async move {
            // ── attempt one-shot JSON ────────────────────────────────────────
            let one_shot_prompt = format!(
                "Produce a complete TomlNarrativeFile JSON value matching this schema.\n\
                 Respond with ONLY valid JSON, no explanation.\n\n{json_schema}"
            );
            let req = GenerateRequest::new(vec![Message::new(
                Role::User,
                vec![Input::Text(one_shot_prompt)],
            )]);
            if let Ok(resp) = driver.generate(&req).await {
                let text: String = resp
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
                    .join("");
                if let Ok(file) =
                    serde_json::from_str::<botticelli_narrative::TomlNarrativeFile>(text.trim())
                {
                    tracing::info!("Wizard one-shot JSON succeeded");
                    tx.send(WizardAgentResult::JsonComplete {
                        file: Box::new(file),
                        path,
                    })
                    .ok();
                    return;
                }
                tracing::warn!("One-shot JSON parse failed; falling back to per-field");
            }

            // ── per-field fallback ───────────────────────────────────────────
            let system = "You are filling in a narrative configuration form. \
                Answer each question with only the value requested — no explanation, no preamble.";
            let mut answers = Vec::with_capacity(remaining_prompts.len());
            for prompt in &remaining_prompts {
                let msg_sys = Message::new(Role::User, vec![Input::Text(system.to_string())]);
                let msg_q = Message::new(Role::User, vec![Input::Text(prompt.clone())]);
                let req = GenerateRequest::new(vec![msg_sys, msg_q]);
                let answer = match driver.generate(&req).await {
                    Ok(resp) => resp
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
                        .join(""),
                    Err(e) => {
                        tracing::error!(error = %e, "Wizard per-field generate failed");
                        format!("(agent error: {e})")
                    }
                };
                answers.push(answer.trim().to_string());
            }
            tx.send(WizardAgentResult::AllFields(answers)).ok();
        });
    }

    /// Apply a `BotTransition`. Returns `true` if the app should exit.
    #[instrument(skip(self))]
    async fn apply(&mut self, transition: BotTransition) -> TuiResult<bool> {
        match transition {
            BotTransition::Stay => {}
            BotTransition::Quit => return Ok(true),

            BotTransition::GoToBots => {
                let mut screen = BotStatusScreen::new();
                screen.on_user_bots_loaded(load_user_bot_names().await);
                self.current = CurrentScreen::Other(Box::new(screen));
            }
            BotTransition::GoToChat => {
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
                if self.driver.is_some() {
                    self.dispatch_generate(content);
                } else if self.driver_rx.is_some() {
                    info!("Driver not yet ready; queuing prompt");
                    self.pending_prompt = Some(content);
                } else {
                    self.chat.on_content_chunk(
                        "No LLM driver configured. Use --provider to select a backend.".to_string(),
                    );
                    self.chat.on_stream_done();
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
                let has_storage = self.ctx.storage.is_some();
                self.current =
                    CurrentScreen::Other(Box::new(DatabaseBrowserScreen::new(has_storage)));
            }
            BotTransition::GoToSchedule => {
                let (task_rows, task_ids, has_storage) = if let Some(ref storage) = self.ctx.storage
                {
                    let (rows, ids) = fetch_schedule_data(Arc::clone(storage)).await;
                    (rows, ids, true)
                } else {
                    (vec![], vec![], false)
                };
                self.current = CurrentScreen::Other(Box::new(ScheduleScreen::new(
                    task_rows,
                    task_ids,
                    has_storage,
                )));
            }
            BotTransition::LoadScheduleData => {
                if let Some(ref storage) = self.ctx.storage {
                    let (task_rows, task_ids) = fetch_schedule_data(Arc::clone(storage)).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_schedule_loaded(task_rows, task_ids);
                    }
                }
            }
            BotTransition::LoadTaskExecutions { task_id } => {
                if let Some(ref storage) = self.ctx.storage {
                    let exec_rows = fetch_task_executions(Arc::clone(storage), &task_id).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_task_executions_loaded(exec_rows);
                    }
                }
            }
            BotTransition::GoToLogViewer => {
                let lines = if let Some(ref path) = self.ctx.log_file {
                    read_log_lines(path).await
                } else {
                    vec![]
                };
                self.current = CurrentScreen::Other(Box::new(LogViewerScreen::new(lines)));
            }
            BotTransition::LoadLogLines => {
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
                let log_file = self.ctx.log_file.as_ref().map(|p| p.display().to_string());
                let storage_path = self.ctx.storage.as_ref().map(|_| {
                    std::env::var("REDB_PATH").unwrap_or_else(|_| "./botticelli.redb".to_string())
                });
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
                if let Some(ref storage) = self.ctx.storage {
                    let storage = Arc::clone(storage);
                    let rows = fetch_table_rows(storage, &table).await;
                    if let CurrentScreen::Other(s) = &mut self.current {
                        s.on_table_loaded(&table, rows);
                    }
                }
            }

            BotTransition::SaveNarrative { path, toml } => {
                info!(?path, bytes = toml.len(), "Saving narrative");
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&path, &toml).await?;
            }

            BotTransition::GoToNarrativeWizard { path } => {
                info!(?path, "Opening narrative wizard");
                let current_model = self.driver.as_ref().map(|d| d.model_name().to_string());
                self.current =
                    CurrentScreen::Other(Box::new(NarrativeWizardScreen::new(path, current_model)));
            }

            BotTransition::AgentFillNarrativeNext { prompt } => {
                self.dispatch_wizard_next(prompt);
            }

            BotTransition::AgentFillNarrativeAll {
                remaining_prompts,
                json_schema,
            } => {
                let path = if let CurrentScreen::Other(s) = &self.current {
                    s.screen_name();
                    None
                } else {
                    None
                };
                self.dispatch_wizard_all(remaining_prompts, json_schema, path);
            }

            BotTransition::NarrativeWizardComplete { answers, path } => {
                info!(
                    fields = answers.len(),
                    ?path,
                    "Narrative wizard complete — replaying answers through NarrativeGenerator"
                );
                use elicitation::{ElicitCommunicator, Elicitation, Generator};
                let comm = BufferedCommunicator::new(answers)
                    .with_style::<String, elicitation::StringStyle>(
                        elicitation::StringStyle::Human,
                    );
                match botticelli_narrative::NarrativeGenerator::elicit(&comm).await {
                    Ok(generator) => {
                        let file = generator.generate();
                        let raw = toml::to_string_pretty(&file).unwrap_or_default();
                        let save_path = path.or_else(|| {
                            self.ctx
                                .narratives_dir
                                .as_ref()
                                .map(|d| d.join(format!("{}.toml", generator.name)))
                        });
                        if let Some(ref p) = save_path {
                            if let Some(parent) = p.parent() {
                                tokio::fs::create_dir_all(parent).await.ok();
                            }
                            tokio::fs::write(p, &raw).await.ok();
                            info!(?save_path, "Saved narrative after wizard completion");
                        }
                        let dir = self.ctx.narratives_dir.clone();
                        self.current =
                            CurrentScreen::Other(Box::new(NarrativeBrowserScreen::new(dir)));
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to rebuild narrative from wizard answers");
                        let screen = NarrativeEditorScreen::new_file();
                        self.current = CurrentScreen::Other(Box::new(screen));
                    }
                }
            }

            BotTransition::GoToBotWizard => {
                // Handled inline in run() where the terminal handle is available.
                // drive() in tests may reach this arm — treat it as a no-op.
            }

            BotTransition::StartBot(kind) => {
                info!(%kind, "StartBot requested (not yet wired to BotServer)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, true);
                }
            }
            BotTransition::StopBot(kind) => {
                info!(%kind, "StopBot requested (not yet wired to BotServer)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, false);
                }
            }
            BotTransition::RestartBot(kind) => {
                info!(%kind, "RestartBot (not yet wired to BotServer) — marking running");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_bot_state_changed(kind, true);
                }
            }

            BotTransition::StartUserBot { name } => {
                info!(%name, "StartUserBot (not yet wired to runner)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_user_bot_state_changed(&name, true);
                }
            }
            BotTransition::StopUserBot { name } => {
                info!(%name, "StopUserBot (not yet wired to runner)");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_user_bot_state_changed(&name, false);
                }
            }
            BotTransition::RestartUserBot { name } => {
                info!(%name, "RestartUserBot (not yet wired to runner) — marking running");
                if let CurrentScreen::Other(s) = &mut self.current {
                    s.on_user_bot_state_changed(&name, true);
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
#[tracing::instrument(skip(storage))]
async fn fetch_schedule_data(storage: Arc<dyn BotStorage>) -> (Vec<String>, Vec<String>) {
    let states = storage.list_actor_states().await.unwrap_or_default();
    let rows = states
        .iter()
        .map(|r| {
            let ts: String = r.next_run.to_string().chars().take(19).collect();
            let paused = if r.is_paused { "PAUSED" } else { "active" };
            format!(
                "{} | {} | {} | next: {}",
                paused, r.task_id, r.actor_name, ts
            )
        })
        .collect();
    let ids = states.into_iter().map(|r| r.task_id).collect();
    (rows, ids)
}

/// Fetch recent executions for a single task, pre-formatted as one-liners.
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

// ── User bot config persistence ──────────────────────────────────────────────

/// Append a newly elicited [`UserBotConfig`] to `botticelli-user-bots.jsonl`.
///
/// Each line is one JSON object so the file remains valid JSON-lines even as
/// bots accumulate.  Missing file is created on first save.
#[tracing::instrument(skip(config), fields(name = %config.name))]
async fn save_user_bot_config(config: &UserBotConfig) {
    let line = match serde_json::to_string(config) {
        Ok(s) => format!("{s}\n"),
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize UserBotConfig");
            return;
        }
    };
    let path = std::path::Path::new("botticelli-user-bots.jsonl");
    let existing = tokio::fs::read_to_string(path).await.unwrap_or_default();
    if let Err(e) = tokio::fs::write(path, format!("{existing}{line}")).await {
        tracing::error!(error = %e, "Failed to write botticelli-user-bots.jsonl");
    }
}

/// Read bot names from `botticelli-user-bots.jsonl`, one name per valid line.
#[tracing::instrument]
async fn load_user_bot_names() -> Vec<String> {
    let raw = tokio::fs::read_to_string("botticelli-user-bots.jsonl")
        .await
        .unwrap_or_default();
    raw.lines()
        .filter_map(|line| {
            serde_json::from_str::<UserBotConfig>(line)
                .ok()
                .map(|c| c.name)
        })
        .collect()
}

// ── BufferedCommunicator ─────────────────────────────────────────────────────

/// A pre-loaded [`ElicitCommunicator`] that drains answers from a queue.
///
/// Used by `NarrativeWizardComplete` to replay collected answers through
/// `TomlNarrativeFile::elicit()` and reconstruct the struct without another
/// round of I/O.
#[derive(Clone)]
struct BufferedCommunicator {
    answers: std::sync::Arc<Vec<String>>,
    idx: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    style_context: elicitation::StyleContext,
    elicitation_context: elicitation::ElicitationContext,
}

impl BufferedCommunicator {
    #[instrument]
    fn new(answers: Vec<String>) -> Self {
        Self {
            answers: std::sync::Arc::new(answers),
            idx: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            style_context: elicitation::StyleContext::default(),
            elicitation_context: elicitation::ElicitationContext::default(),
        }
    }
}

impl elicitation::ElicitCommunicator for BufferedCommunicator {
    #[tracing::instrument(skip(self, _prompt))]
    async fn send_prompt(&self, _prompt: &str) -> elicitation::ElicitResult<String> {
        let i = self.idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.answers.get(i).cloned().unwrap_or_default())
    }

    async fn call_tool(
        &self,
        _params: elicitation::rmcp::model::CallToolRequestParams,
    ) -> Result<elicitation::rmcp::model::CallToolResult, elicitation::rmcp::ServiceError> {
        Err(elicitation::rmcp::ServiceError::Cancelled {
            reason: Some(
                "BufferedCommunicator::call_tool is unreachable in the wizard replay path: \
                 TomlNarrativeFile elicitation uses only send_prompt"
                    .to_string(),
            ),
        })
    }

    fn style_context(&self) -> &elicitation::StyleContext {
        &self.style_context
    }

    fn with_style<
        T: 'static,
        S: elicitation::StyleMarker + elicitation::style::ElicitationStyle + 'static,
    >(
        &self,
        style: S,
    ) -> Self {
        let mut new = self.clone();
        if let Err(e) = new.style_context.set_style::<T, S>(style) {
            tracing::error!(error = %e, "BufferedCommunicator::with_style failed to set style");
        }
        new
    }

    fn elicitation_context(&self) -> &elicitation::ElicitationContext {
        &self.elicitation_context
    }
}
