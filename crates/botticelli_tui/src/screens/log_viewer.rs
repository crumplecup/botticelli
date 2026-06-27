//! Log viewer screen — scrollable tail of the server log file with level filter.

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

const PAGE_SIZE: usize = 20;

/// Which log levels to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    /// Show all lines.
    All,
    /// Show lines containing " INFO" or higher.
    Info,
    /// Show lines containing " WARN" or " ERROR".
    Warn,
    /// Show lines containing " ERROR" only.
    Error,
}

impl LogFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Info => "INFO+",
            Self::Warn => "WARN+",
            Self::Error => "ERROR",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Info,
            Self::Info => Self::Warn,
            Self::Warn => Self::Error,
            Self::Error => Self::All,
        }
    }

    fn matches(self, line: &str) -> bool {
        match self {
            Self::All => true,
            Self::Info => {
                line.contains(" INFO") || line.contains(" WARN") || line.contains(" ERROR")
            }
            Self::Warn => line.contains(" WARN") || line.contains(" ERROR"),
            Self::Error => line.contains(" ERROR"),
        }
    }
}

/// Scrollable tail of a log file with level filtering.
///
/// Lines are loaded by the controller via `LoadLogLines` / `GoToLogViewer`
/// and delivered via [`on_log_lines_loaded`](LogViewerScreen::on_log_lines_loaded).
/// The screen is pure — it never reads the file itself.
pub struct LogViewerScreen {
    all_lines: Vec<String>,
    offset: usize,
    filter: LogFilter,
    tail: bool,
}

impl LogViewerScreen {
    /// Create with the initial log lines (may be empty).
    pub fn new(lines: Vec<String>) -> Self {
        let len = lines.len();
        Self {
            all_lines: lines,
            offset: len.saturating_sub(PAGE_SIZE),
            filter: LogFilter::All,
            tail: true,
        }
    }

    /// Current scroll offset (index into filtered lines).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Active log level filter.
    pub fn filter(&self) -> LogFilter {
        self.filter
    }

    fn filtered_lines(&self) -> Vec<&str> {
        self.all_lines
            .iter()
            .filter(|l| self.filter.matches(l))
            .map(String::as_str)
            .collect()
    }

    fn paragraph(text: impl Into<String>, title: Option<String>) -> TuiNode {
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(text.into()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: title.map(|t| BlockJson {
                    title: Some(t),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        }
    }
}

impl BotScreen for LogViewerScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let lines = self.filtered_lines();
        let total = lines.len();
        let offset = self.offset.min(total.saturating_sub(1));
        let page = &lines[offset..total.min(offset + PAGE_SIZE)];
        let content: String = page.join("\n");

        let title = format!(
            "Log Viewer [filter: {}] [{}/{}]",
            self.filter.label(),
            offset + 1,
            total.max(1)
        );
        let log_widget = Self::paragraph(content, Some(title));

        let tail_indicator = if self.tail { "TAIL" } else { "SCROLL" };
        let help = Self::paragraph(
            format!(
                "  j/k=scroll  G=tail  f=filter({})  r=refresh  {}  1=bots  2=chat  q=quit",
                self.filter.label(),
                tail_indicator,
            ),
            None,
        );

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![log_widget, help],
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.filtered_lines().len().saturating_sub(1);
                self.offset = (self.offset + 1).min(max);
                self.tail = self.offset >= max;
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.offset = self.offset.saturating_sub(1);
                self.tail = false;
                BotTransition::Stay
            }
            KeyCode::PageDown => {
                let max = self.filtered_lines().len().saturating_sub(1);
                self.offset = (self.offset + PAGE_SIZE).min(max);
                self.tail = self.offset >= max;
                BotTransition::Stay
            }
            KeyCode::PageUp => {
                self.offset = self.offset.saturating_sub(PAGE_SIZE);
                self.tail = false;
                BotTransition::Stay
            }
            KeyCode::Char('G') => {
                let total = self.filtered_lines().len();
                self.offset = total.saturating_sub(PAGE_SIZE);
                self.tail = true;
                BotTransition::Stay
            }
            KeyCode::Char('f') => {
                self.filter = self.filter.next();
                let total = self.filtered_lines().len();
                if self.tail {
                    self.offset = total.saturating_sub(PAGE_SIZE);
                }
                BotTransition::Stay
            }
            KeyCode::Char('r') => BotTransition::LoadLogLines,
            KeyCode::Char('1') => BotTransition::GoToBots,
            KeyCode::Char('2') => BotTransition::GoToChat,
            KeyCode::Char('3') => BotTransition::GoToNarratives,
            KeyCode::Char('4') => BotTransition::GoToDatabase,
            KeyCode::Char('5') => BotTransition::GoToSchedule,
            KeyCode::Char('7') => BotTransition::GoToSettings,
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Log Viewer"
    }

    fn on_log_lines_loaded(&mut self, lines: Vec<String>) {
        let was_tail = self.tail;
        self.all_lines = lines;
        if was_tail {
            let total = self.filtered_lines().len();
            self.offset = total.saturating_sub(PAGE_SIZE);
        }
    }
}
