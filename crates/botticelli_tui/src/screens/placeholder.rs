//! Temporary placeholder screen used while per-screen implementations are built.
//!
//! Renders a centered "coming soon" message. Removed once all real screens are
//! implemented.

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{BlockJson, BordersJson, ParagraphText, TuiNode, WidgetJson};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

/// A placeholder that renders a "coming soon" message for unimplemented screens.
#[derive(Debug, Clone)]
pub struct PlaceholderScreen {
    name: &'static str,
}

impl PlaceholderScreen {
    /// Create a placeholder for the named screen.
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl BotScreen for PlaceholderScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let msg = format!(
            "[ {} — coming soon ]\n\n1 Bots  2 Chat  3 Narratives  4 DB  5 Schedule  6 Logs  7 Settings  q Quit",
            self.name
        );
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(msg),
                style: None,
                wrap: true,
                scroll: None,
                alignment: Some("Center".to_string()),
                block: Some(BlockJson {
                    title: Some(self.name.to_string()),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            KeyCode::Char('1') => BotTransition::GoToBots,
            KeyCode::Char('2') => BotTransition::GoToChat,
            KeyCode::Char('3') => BotTransition::GoToNarratives,
            KeyCode::Char('4') => BotTransition::GoToDatabase,
            KeyCode::Char('5') => BotTransition::GoToSchedule,
            KeyCode::Char('6') => BotTransition::GoToLogViewer,
            KeyCode::Char('7') => BotTransition::GoToSettings,
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        self.name
    }
}
