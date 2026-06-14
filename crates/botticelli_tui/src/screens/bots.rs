//! Bot operator console — shows status of all three bots and issues control
//! commands.

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

const BOT_NAMES: [&str; 3] = ["generation", "curation", "posting"];

/// Bot operator console screen.
///
/// Shows the run state of all three bots. Keys `j`/`k` navigate; `s`/`x`/`r`
/// send [`BotTransition::StartBot`] / [`BotTransition::StopBot`] /
/// [`BotTransition::RestartBot`] to the controller.
#[derive(Debug, Clone)]
pub struct BotStatusScreen {
    /// Cursor position (0 = generation, 1 = curation, 2 = posting).
    selected: usize,
    /// Per-bot run state, mirrored from controller after each transition.
    states: [RunState; 3],
}

impl BotStatusScreen {
    /// Create a new Bots screen with all bots shown as stopped.
    pub fn new() -> Self {
        Self {
            selected: 0,
            states: [RunState::Stopped; 3],
        }
    }

    /// Current run state for a bot by index.
    pub fn state(&self, idx: usize) -> RunState {
        self.states[idx]
    }

    /// Currently selected bot index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    #[cfg(feature = "cli")]
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

    fn bot_row(&self, idx: usize) -> TuiNode {
        let cursor = if idx == self.selected { "▶" } else { " " };
        let name = BOT_NAMES[idx];
        let badge = self.states[idx].badge();
        let text = format!("  {}  {:<12}  {}", cursor, name, badge);
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
                text: ParagraphText::Plain("  Bot             Status".to_string()),
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
                    "  s=start  x=stop  r=restart  j/k=select  1-7=screens  q=quit".to_string(),
                ),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        };

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Length { value: 3 },
                ConstraintJson::Length { value: 1 },
                ConstraintJson::Length { value: 1 },
                ConstraintJson::Length { value: 1 },
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 2 },
            ],
            children: vec![
                header,
                self.bot_row(0),
                self.bot_row(1),
                self.bot_row(2),
                TuiNode::Widget {
                    widget: Box::new(WidgetJson::Paragraph {
                        text: ParagraphText::Plain(String::new()),
                        style: None,
                        wrap: true,
                        scroll: None,
                        alignment: None,
                        block: None,
                    }),
                },
                help,
            ],
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1) % 3;
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(2);
                BotTransition::Stay
            }
            #[cfg(feature = "cli")]
            KeyCode::Char('s') => BotTransition::StartBot(Self::kind_for(self.selected)),
            #[cfg(feature = "cli")]
            KeyCode::Char('x') => BotTransition::StopBot(Self::kind_for(self.selected)),
            #[cfg(feature = "cli")]
            KeyCode::Char('r') => BotTransition::RestartBot(Self::kind_for(self.selected)),
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
}
