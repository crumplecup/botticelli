//! `BotController` — the top-level event loop driving the screen state machine.
//!
//! The controller owns the current `Box<dyn BotScreen>`, calls `to_tui_node`
//! each frame, runs `verified_draw`, and applies `BotTransition` values
//! returned by `handle_key`. All I/O and shared-state mutations live here;
//! screens are pure.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;
use tracing::{debug, info, instrument};

use crate::context::BotScreenContext;
use crate::contracts::{render_resize_prompt, verified_draw};
use crate::error::TuiResult;
use crate::screen::{BotScreen, BotTransition};
use crate::screens::{BotStatusScreen, PlaceholderScreen};

/// Top-level controller for the botticelli TUI.
pub struct BotController {
    screen: Box<dyn BotScreen>,
    ctx: BotScreenContext,
}

impl BotController {
    /// Construct the controller, starting on the Bots screen.
    #[instrument(skip(ctx))]
    pub fn new(ctx: BotScreenContext) -> Self {
        info!("BotController starting on Bots screen");
        Self {
            screen: Box::new(BotStatusScreen::new()),
            ctx,
        }
    }

    /// Run the event loop until the user quits.
    #[instrument(skip(self, terminal))]
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> TuiResult<()> {
        loop {
            // ── render ──────────────────────────────────────────────────────
            let root = self.screen.to_tui_node();

            terminal.draw(|frame| {
                let area = frame.area();
                // Attempt layout-verified draw; fall back to resize prompt.
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
                let transition = self.screen.handle_key(key, &self.ctx);
                if self.apply(transition).await? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Apply a `BotTransition`. Returns `true` if the app should exit.
    #[instrument(skip(self))]
    async fn apply(&mut self, transition: BotTransition) -> TuiResult<bool> {
        match transition {
            BotTransition::Stay => {}
            BotTransition::Quit => return Ok(true),

            BotTransition::GoToBots => {
                self.screen = Box::new(BotStatusScreen::new());
            }
            BotTransition::GoToChat => {
                self.screen = Box::new(PlaceholderScreen::new("Chat"));
            }
            BotTransition::GoToNarratives => {
                self.screen = Box::new(PlaceholderScreen::new("Narratives"));
            }
            BotTransition::GoToNarrativeEditor { path } => {
                info!(?path, "Opening narrative editor");
                self.screen = Box::new(PlaceholderScreen::new("Narrative Editor"));
            }
            BotTransition::GoToDatabase => {
                self.screen = Box::new(PlaceholderScreen::new("Database"));
            }
            BotTransition::GoToSchedule => {
                self.screen = Box::new(PlaceholderScreen::new("Schedule"));
            }
            BotTransition::GoToLogViewer => {
                self.screen = Box::new(PlaceholderScreen::new("Log Viewer"));
            }
            BotTransition::GoToSettings => {
                self.screen = Box::new(PlaceholderScreen::new("Settings"));
            }

            BotTransition::SaveNarrative { path, toml } => {
                info!(?path, bytes = toml.len(), "Saving narrative");
                tokio::fs::write(&path, &toml).await?;
            }

            #[cfg(feature = "cli")]
            BotTransition::StartBot(kind) => {
                info!(%kind, "StartBot requested (not yet wired to BotServer)");
                self.screen.on_bot_state_changed(kind, true);
            }
            #[cfg(feature = "cli")]
            BotTransition::StopBot(kind) => {
                info!(%kind, "StopBot requested (not yet wired to BotServer)");
                self.screen.on_bot_state_changed(kind, false);
            }
            #[cfg(feature = "cli")]
            BotTransition::RestartBot(kind) => {
                info!(%kind, "RestartBot (not yet wired to BotServer) — marking running");
                self.screen.on_bot_state_changed(kind, true);
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
