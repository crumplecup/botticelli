//! Narrative file browser — left pane lists `.toml` files, right pane previews the
//! selected file's act structure.

use std::path::PathBuf;

use botticelli_narrative::TomlNarrativeFile;
use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

/// Summary of a single narrative file shown in the browser list.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct NarrativeEntry {
    /// Absolute path to the `.toml` file.
    path: PathBuf,
    /// Display name (file stem).
    name: String,
    /// Number of acts defined in the file (0 for parse failures).
    act_count: usize,
    /// Whether the file parsed without errors.
    valid: bool,
}

impl NarrativeEntry {
    fn load(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        match std::fs::read_to_string(&path).map(|s| toml::from_str::<TomlNarrativeFile>(&s)) {
            Ok(Ok(file)) => {
                let act_count = file.acts.len();
                Self {
                    path,
                    name,
                    act_count,
                    valid: true,
                }
            }
            _ => Self {
                path,
                name,
                act_count: 0,
                valid: false,
            },
        }
    }
}

/// Two-pane narrative browser: list on the left, act-count preview on the right.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct NarrativeBrowserScreen {
    /// All narrative entries found in the configured directory.
    entries: Vec<NarrativeEntry>,
    /// Index of the currently highlighted entry.
    selected: usize,
}

impl NarrativeBrowserScreen {
    /// Scan `dir` for `.toml` files and populate the entry list.
    ///
    /// Pass `None` to show the "no directory configured" placeholder.
    #[instrument]
    pub fn new(dir: Option<PathBuf>) -> Self {
        let entries = match dir {
            None => Vec::new(),
            Some(d) => Self::scan(&d),
        };
        Self {
            entries,
            selected: 0,
        }
    }

    fn scan(dir: &PathBuf) -> Vec<NarrativeEntry> {
        let Ok(read) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut entries: Vec<NarrativeEntry> = read
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "toml").unwrap_or(false))
            .map(|e| NarrativeEntry::load(e.path()))
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    fn selected_entry(&self) -> Option<&NarrativeEntry> {
        self.entries.get(self.selected)
    }

    fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    fn move_up(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.selected.saturating_sub(1).min(self.entries.len() - 1);
        }
    }

    fn list_pane(&self) -> TuiNode {
        let header = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: Some(BlockJson {
                    title: Some("Narratives".to_string()),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        };

        let mut constraints = vec![ConstraintJson::Length { value: 3 }];
        let mut children = vec![header];

        if self.entries.is_empty() {
            constraints.push(ConstraintJson::Fill { value: 1 });
            children.push(TuiNode::Widget {
                widget: Box::new(WidgetJson::Paragraph {
                    text: ParagraphText::Plain("  (no narratives found)".to_string()),
                    style: None,
                    wrap: true,
                    scroll: None,
                    alignment: None,
                    block: None,
                }),
            });
        } else {
            for (i, entry) in self.entries.iter().enumerate() {
                let cursor = if i == self.selected { "▶ " } else { "  " };
                let valid_mark = if entry.valid { "" } else { " ✗" };
                constraints.push(ConstraintJson::Length { value: 1 });
                children.push(TuiNode::Widget {
                    widget: Box::new(WidgetJson::Paragraph {
                        text: ParagraphText::Plain(format!(
                            "{}{}{}",
                            cursor, entry.name, valid_mark
                        )),
                        style: None,
                        wrap: true,
                        scroll: None,
                        alignment: None,
                        block: None,
                    }),
                });
            }
            constraints.push(ConstraintJson::Fill { value: 1 });
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
        }

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn preview_pane(&self) -> TuiNode {
        let content = match self.selected_entry() {
            None => "  No file selected.".to_string(),
            Some(entry) => {
                if !entry.valid {
                    format!("  ✗  {} — parse error", entry.name)
                } else {
                    format!("  {}  ({} acts)", entry.name, entry.act_count)
                }
            }
        };

        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(content),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: Some(BlockJson {
                    title: Some("Preview".to_string()),
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

impl BotScreen for NarrativeBrowserScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let help = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(
                    "  j/k=navigate  Enter=edit  n=new  Esc=bots".to_string(),
                ),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        };

        let split = TuiNode::Layout {
            direction: DirectionJson::Horizontal,
            constraints: vec![
                ConstraintJson::Percentage { value: 30 },
                ConstraintJson::Percentage { value: 70 },
            ],
            children: vec![self.list_pane(), self.preview_pane()],
            margin: None,
        };

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![split, help],
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => BotTransition::GoToBots,
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down();
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up();
                BotTransition::Stay
            }
            KeyCode::Enter => {
                let path = self.selected_entry().map(|e| e.path.clone());
                BotTransition::GoToNarrativeEditor { path }
            }
            KeyCode::Char('n') => BotTransition::GoToNarrativeEditor { path: None },
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Narratives"
    }
}
