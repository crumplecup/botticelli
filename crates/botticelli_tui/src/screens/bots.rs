//! Bot operator console — shows status of system bots and user-created bots.

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotKind, BotScreen, BotTransition};

/// Whether a bot actor is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// Actor is live and cycling.
    Running,
    /// Actor is stopped or was never started.
    Stopped,
}

impl RunState {
    fn badge(self) -> &'static str {
        match self {
            RunState::Running => "● Running",
            RunState::Stopped => "○ Stopped",
        }
    }
}

const SYSTEM_BOT_NAMES: [&str; 3] = ["generation", "curation", "posting"];

/// Bot operator console screen.
///
/// Shows the run state of the three system bots and any user-created bots.
/// Keys `j`/`k` navigate; `s`/`x`/`r` send start/stop/restart transitions.
#[derive(Debug, Clone)]
pub struct BotStatusScreen {
    /// Cursor position (0 = generation, 1 = curation, 2 = posting, 3+ = user bots).
    selected: usize,
    /// Per-system-bot run state indexed 0-2.
    states: [RunState; 3],
    /// User-created bots (name, state) loaded from `botticelli-user-bots.jsonl`.
    user_bots: Vec<(String, RunState)>,
}

impl BotStatusScreen {
    /// Create a new Bots screen with all bots shown as stopped.
    pub fn new() -> Self {
        Self {
            selected: 0,
            states: [RunState::Stopped; 3],
            user_bots: Vec::new(),
        }
    }

    /// Current run state for a system bot by index.
    pub fn state(&self, idx: usize) -> RunState {
        self.states[idx]
    }

    /// Run state for a user bot by name.
    pub fn user_state(&self, name: &str) -> Option<RunState> {
        self.user_bots
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| *s)
    }

    /// Currently selected row index (covers system + user bots).
    pub fn selected(&self) -> usize {
        self.selected
    }

    fn total_len(&self) -> usize {
        3 + self.user_bots.len()
    }

    fn kind_for(idx: usize) -> BotKind {
        match idx {
            0 => BotKind::Generation,
            1 => BotKind::Curation,
            _ => BotKind::Posting,
        }
    }

    fn idx_for(kind: BotKind) -> usize {
        match kind {
            BotKind::Generation => 0,
            BotKind::Curation => 1,
            BotKind::Posting => 2,
        }
    }

    fn bot_row(&self, row: usize, name: &str, state: RunState) -> TuiNode {
        let cursor = if row == self.selected { "▶" } else { " " };
        let text = format!("  {}  {:<18}  {}", cursor, name, state.badge());
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(text),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        }
    }
}

impl Default for BotStatusScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BotScreen for BotStatusScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let header = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain("  Bot                   Status".to_string()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: Some(BlockJson {
                    title: Some("Bots".to_string()),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        };

        let help = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(
                    "  s=start  x=stop  r=restart  n=new bot  j/k=select  1-7=screens  q=quit"
                        .to_string(),
                ),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        };

        let mut constraints = vec![
            ConstraintJson::Length { value: 3 }, // header
        ];
        let mut children = vec![header];

        // System bots
        for (i, name) in SYSTEM_BOT_NAMES.iter().enumerate() {
            constraints.push(ConstraintJson::Length { value: 1 });
            children.push(self.bot_row(i, name, self.states[i]));
        }

        // User bots
        for (i, (name, state)) in self.user_bots.iter().enumerate() {
            constraints.push(ConstraintJson::Length { value: 1 });
            children.push(self.bot_row(3 + i, name, *state));
        }

        // Spacer + help
        constraints.push(ConstraintJson::Fill { value: 1 });
        constraints.push(ConstraintJson::Length { value: 2 });
        children.push(TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        });
        children.push(help);

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        let len = self.total_len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1) % len;
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(len - 1);
                BotTransition::Stay
            }
            KeyCode::Char('s') => {
                if self.selected < 3 {
                    BotTransition::StartBot(Self::kind_for(self.selected))
                } else {
                    let name = self.user_bots[self.selected - 3].0.clone();
                    BotTransition::StartUserBot { name }
                }
            }
            KeyCode::Char('x') => {
                if self.selected < 3 {
                    BotTransition::StopBot(Self::kind_for(self.selected))
                } else {
                    let name = self.user_bots[self.selected - 3].0.clone();
                    BotTransition::StopUserBot { name }
                }
            }
            KeyCode::Char('r') => {
                if self.selected < 3 {
                    BotTransition::RestartBot(Self::kind_for(self.selected))
                } else {
                    let name = self.user_bots[self.selected - 3].0.clone();
                    BotTransition::RestartUserBot { name }
                }
            }
            KeyCode::Char('n') => BotTransition::GoToBotWizard,
            KeyCode::Char('2') => BotTransition::GoToChat,
            KeyCode::Char('3') => BotTransition::GoToNarratives,
            KeyCode::Char('4') => BotTransition::GoToDatabase,
            KeyCode::Char('5') => BotTransition::GoToSchedule,
            KeyCode::Char('6') => BotTransition::GoToLogViewer,
            KeyCode::Char('7') => BotTransition::GoToSettings,
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Bots"
    }

    fn on_bot_state_changed(&mut self, kind: BotKind, running: bool) {
        let idx = Self::idx_for(kind);
        self.states[idx] = if running {
            RunState::Running
        } else {
            RunState::Stopped
        };
    }

    fn on_user_bots_loaded(&mut self, names: Vec<String>) {
        self.user_bots = names.into_iter().map(|n| (n, RunState::Stopped)).collect();
        // Clamp cursor if user bots shrunk.
        let len = self.total_len();
        if len > 0 && self.selected >= len {
            self.selected = len - 1;
        }
    }

    fn on_user_bot_state_changed(&mut self, name: &str, running: bool) {
        if let Some((_, state)) = self.user_bots.iter_mut().find(|(n, _)| n == name) {
            *state = if running {
                RunState::Running
            } else {
                RunState::Stopped
            };
        }
    }
}
