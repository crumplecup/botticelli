//! Schedule screen — actor task state list with per-task execution history.

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

const PAGE_SIZE: usize = 18;

/// Left/right split view of scheduled actor tasks and their execution history.
///
/// **Left pane (45%)** — list of `ActorServerStateRecord` rows, navigable with
/// `j`/`k`. `Enter` triggers `LoadTaskExecutions` for the right pane. `r` triggers
/// `LoadScheduleData` to refresh.
///
/// **Right pane (55%)** — most-recent executions for the selected task, delivered
/// via [`on_task_executions_loaded`](ScheduleScreen::on_task_executions_loaded).
pub struct ScheduleScreen {
    task_rows: Vec<String>,
    task_ids: Vec<String>,
    selected: usize,
    task_offset: usize,
    exec_rows: Vec<String>,
    has_storage: bool,
}

impl ScheduleScreen {
    /// Create with pre-loaded task rows and corresponding task IDs.
    ///
    /// Pass empty vecs when storage is unavailable (`has_storage = false`).
    pub fn new(task_rows: Vec<String>, task_ids: Vec<String>, has_storage: bool) -> Self {
        Self {
            task_rows,
            task_ids,
            selected: 0,
            task_offset: 0,
            exec_rows: vec![],
            has_storage,
        }
    }

    /// Currently selected task index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Cached execution rows for the selected task.
    pub fn exec_rows(&self) -> &[String] {
        &self.exec_rows
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

    fn render_left_pane(&self) -> TuiNode {
        let mut text = String::new();
        let end = self.task_rows.len().min(self.task_offset + PAGE_SIZE);
        for (i, row) in self.task_rows[self.task_offset.min(self.task_rows.len())..end]
            .iter()
            .enumerate()
        {
            let abs = self.task_offset + i;
            let cursor = if abs == self.selected { "▶" } else { " " };
            text.push_str(&format!("{}  {}\n", cursor, row));
        }
        if self.task_rows.is_empty() {
            text.push_str(if self.has_storage {
                "  (no tasks — press r to refresh)"
            } else {
                "  Storage not configured."
            });
        }
        Self::paragraph(text, Some("Tasks".to_string()))
    }

    fn render_right_pane(&self) -> TuiNode {
        let text = if self.exec_rows.is_empty() {
            if self.task_rows.is_empty() {
                "  Select a task to view executions.".to_string()
            } else {
                "  Press Enter on a task to load executions.".to_string()
            }
        } else {
            self.exec_rows
                .iter()
                .map(|r| format!("  {}\n", r))
                .collect()
        };
        Self::paragraph(text, Some("Executions".to_string()))
    }
}

impl BotScreen for ScheduleScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let body = TuiNode::Layout {
            direction: DirectionJson::Horizontal,
            constraints: vec![
                ConstraintJson::Percentage { value: 45 },
                ConstraintJson::Percentage { value: 55 },
            ],
            children: vec![self.render_left_pane(), self.render_right_pane()],
            margin: None,
        };

        let help = Self::paragraph(
            "  Enter=executions  j/k=navigate  r=refresh  1=bots  2=chat  4=db  q=quit",
            None,
        );

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![body, help],
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.task_rows.is_empty() {
                    self.selected = (self.selected + 1).min(self.task_rows.len() - 1);
                    if self.selected >= self.task_offset + PAGE_SIZE {
                        self.task_offset += 1;
                    }
                }
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                if self.selected < self.task_offset {
                    self.task_offset = self.selected;
                }
                BotTransition::Stay
            }
            KeyCode::Enter => {
                if let Some(task_id) = self.task_ids.get(self.selected) {
                    BotTransition::LoadTaskExecutions {
                        task_id: task_id.clone(),
                    }
                } else {
                    BotTransition::Stay
                }
            }
            KeyCode::Char('r') => BotTransition::LoadScheduleData,
            KeyCode::Char('1') => BotTransition::GoToBots,
            KeyCode::Char('2') => BotTransition::GoToChat,
            KeyCode::Char('3') => BotTransition::GoToNarratives,
            KeyCode::Char('4') => BotTransition::GoToDatabase,
            KeyCode::Char('6') => BotTransition::GoToLogViewer,
            KeyCode::Char('7') => BotTransition::GoToSettings,
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Schedule"
    }

    fn on_schedule_loaded(&mut self, task_rows: Vec<String>, task_ids: Vec<String>) {
        self.task_rows = task_rows;
        self.task_ids = task_ids;
        self.selected = 0;
        self.task_offset = 0;
        self.exec_rows = vec![];
    }

    fn on_task_executions_loaded(&mut self, exec_rows: Vec<String>) {
        self.exec_rows = exec_rows;
    }
}
