//! Database browser screen — two-mode drill-down into BotStorage tables.

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

/// Logical BotStorage tables shown in the browser.
const TABLES: &[(&str, &str)] = &[
    ("narrative_executions", "Narrative execution runs"),
    ("actor_states", "Scheduled actor task states"),
    ("actor_executions", "Actor execution history"),
    ("content", "Generated content rows"),
    ("model_responses", "LLM request/response log"),
];

const PAGE_SIZE: usize = 20;

/// Current display mode for the database browser.
enum DbMode {
    /// Showing the list of available tables.
    TableList,
    /// Showing paginated rows for a selected table.
    ContentView {
        table_name: String,
        rows: Vec<String>,
        offset: usize,
    },
    /// Storage is not configured in context.
    NoStorage,
}

/// Two-mode database browser screen.
///
/// **Mode 1 – Table list:** lists BotStorage logical tables. `Enter` triggers
/// a `LoadDatabaseTable` transition; the controller fetches rows and calls
/// [`on_table_loaded`](DatabaseBrowserScreen::on_table_loaded).
///
/// **Mode 2 – Content view:** shows paginated one-line row summaries.
/// `Esc` returns to the table list.
pub struct DatabaseBrowserScreen {
    mode: DbMode,
    selected: usize,
}

impl DatabaseBrowserScreen {
    /// Create a browser screen. Pass `has_storage = true` when storage is
    /// available in context; `false` shows a "not configured" message.
    pub fn new(has_storage: bool) -> Self {
        Self {
            mode: if has_storage {
                DbMode::TableList
            } else {
                DbMode::NoStorage
            },
            selected: 0,
        }
    }

    /// Currently selected table index (in TableList mode).
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// True when the screen is showing the content of a table.
    pub fn is_content_view(&self) -> bool {
        matches!(self.mode, DbMode::ContentView { .. })
    }

    fn make_paragraph(text: impl Into<String>, title: Option<String>) -> TuiNode {
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

    fn render_table_list(&self) -> TuiNode {
        let header = Self::make_paragraph("", Some("Database".to_string()));

        let mut children = vec![header];
        for (i, (name, desc)) in TABLES.iter().enumerate() {
            let cursor = if i == self.selected { "▶" } else { " " };
            children.push(Self::make_paragraph(
                format!("  {}  {:<26}  {}", cursor, name, desc),
                None,
            ));
        }

        // fill + help
        children.push(Self::make_paragraph("", None));
        children.push(Self::make_paragraph(
            "  Enter=open  j/k=navigate  1=bots  2=chat  3=narratives  q=quit",
            None,
        ));

        let mut constraints = vec![ConstraintJson::Length { value: 3 }];
        for _ in 0..TABLES.len() {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        constraints.push(ConstraintJson::Fill { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn render_content_view(table_name: &str, rows: &[String], offset: usize) -> TuiNode {
        let title = format!("Database > {}", table_name);
        let header = Self::make_paragraph("", Some(title));

        let page = &rows[offset.min(rows.len())..rows.len().min(offset + PAGE_SIZE)];
        let mut children = vec![header];
        for row in page {
            children.push(Self::make_paragraph(format!("  {}", row), None));
        }

        let total = rows.len();
        let from = if total == 0 { 0 } else { offset + 1 };
        let to = (offset + PAGE_SIZE).min(total);
        let status = if total == 0 {
            "(empty)".to_string()
        } else {
            format!("rows {}-{} of {}  PgDn/PgUp=page", from, to, total)
        };

        children.push(Self::make_paragraph("", None));
        children.push(Self::make_paragraph(
            format!("  {}  Esc=back  q=quit", status),
            None,
        ));

        let mut constraints = vec![ConstraintJson::Length { value: 3 }];
        for _ in 0..page.len() {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        constraints.push(ConstraintJson::Fill { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn render_no_storage() -> TuiNode {
        let header = Self::make_paragraph("", Some("Database".to_string()));
        let msg = Self::make_paragraph(
            "  Storage not configured. Set REDB_PATH to enable the database browser.",
            None,
        );
        let help = Self::make_paragraph("  1=bots  2=chat  3=narratives  q=quit", None);

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Length { value: 3 },
                ConstraintJson::Length { value: 2 },
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![header, msg, Self::make_paragraph("", None), help],
            margin: None,
        }
    }
}

impl BotScreen for DatabaseBrowserScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        match &self.mode {
            DbMode::TableList => self.render_table_list(),
            DbMode::ContentView {
                table_name,
                rows,
                offset,
            } => Self::render_content_view(table_name, rows, *offset),
            DbMode::NoStorage => Self::render_no_storage(),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match &mut self.mode {
            DbMode::TableList => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.selected = (self.selected + 1) % TABLES.len();
                    BotTransition::Stay
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = self.selected.checked_sub(1).unwrap_or(TABLES.len() - 1);
                    BotTransition::Stay
                }
                KeyCode::Enter => BotTransition::LoadDatabaseTable {
                    table: TABLES[self.selected].0.to_string(),
                },
                KeyCode::Char('1') => BotTransition::GoToBots,
                KeyCode::Char('2') => BotTransition::GoToChat,
                KeyCode::Char('3') => BotTransition::GoToNarratives,
                KeyCode::Char('5') => BotTransition::GoToSchedule,
                KeyCode::Char('6') => BotTransition::GoToLogViewer,
                KeyCode::Char('7') => BotTransition::GoToSettings,
                KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
                _ => BotTransition::Stay,
            },
            DbMode::ContentView { offset, rows, .. } => match key.code {
                KeyCode::Esc => {
                    self.mode = DbMode::TableList;
                    BotTransition::Stay
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = rows.len().saturating_sub(1);
                    *offset = (*offset + 1).min(max);
                    BotTransition::Stay
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *offset = offset.saturating_sub(1);
                    BotTransition::Stay
                }
                KeyCode::PageDown => {
                    let max = rows.len().saturating_sub(1);
                    *offset = (*offset + PAGE_SIZE).min(max);
                    BotTransition::Stay
                }
                KeyCode::PageUp => {
                    *offset = offset.saturating_sub(PAGE_SIZE);
                    BotTransition::Stay
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
                _ => BotTransition::Stay,
            },
            DbMode::NoStorage => match key.code {
                KeyCode::Char('1') => BotTransition::GoToBots,
                KeyCode::Char('2') => BotTransition::GoToChat,
                KeyCode::Char('3') => BotTransition::GoToNarratives,
                KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
                _ => BotTransition::Stay,
            },
        }
    }

    fn screen_name(&self) -> &'static str {
        "Database"
    }

    fn on_table_loaded(&mut self, table: &str, rows: Vec<String>) {
        self.mode = DbMode::ContentView {
            table_name: table.to_string(),
            rows,
            offset: 0,
        };
    }
}
