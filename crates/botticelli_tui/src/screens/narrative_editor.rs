//! Narrative editor — displays parsed act structure with live validation feedback.
//!
//! `Ctrl+S` serializes the current `TomlNarrativeFile` and emits `SaveNarrative`.
//! Editing act prompts inline is a planned future enhancement.

use std::path::PathBuf;

use botticelli_narrative::TomlNarrativeFile;
use botticelli_narrative::validator::validate_narrative_toml;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

/// Load state of the file shown in the editor.
#[derive(Debug, Clone)]
pub enum EditorContent {
    /// Blank editor — user is creating a new file.
    New,
    /// File loaded successfully.
    Loaded {
        /// Parsed narrative.
        file: Box<TomlNarrativeFile>,
        /// Raw TOML string (for Ctrl+S round-trip).
        raw: String,
    },
    /// File could not be read or parsed; message describes the error.
    Error(String),
}

/// Narrative editor screen.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct NarrativeEditorScreen {
    /// Destination path (`None` for new files until first save).
    path: Option<PathBuf>,
    /// Parsed content or error state.
    content: EditorContent,
    /// Validation status of the current TOML.
    validation_summary: String,
    /// Scroll offset into the act list.
    scroll: usize,
}

impl NarrativeEditorScreen {
    /// Open an existing narrative file for viewing.
    #[instrument]
    pub fn open(path: PathBuf) -> Self {
        let (content, validation_summary) = match std::fs::read_to_string(&path) {
            Err(e) => (
                EditorContent::Error(e.to_string()),
                format!("Error reading file: {e}"),
            ),
            Ok(raw) => match toml::from_str::<TomlNarrativeFile>(&raw) {
                Err(e) => (
                    EditorContent::Error(e.to_string()),
                    format!("Parse error: {e}"),
                ),
                Ok(file) => {
                    let result = validate_narrative_toml(&raw);
                    let summary = if result.is_valid() {
                        format!("✓ Valid  ({} acts)", file.acts.len())
                    } else {
                        format!(
                            "✗ {} error(s)  {}",
                            result.errors.len(),
                            result
                                .errors
                                .first()
                                .map(|e| e.message.as_str())
                                .unwrap_or_default()
                        )
                    };
                    (
                        EditorContent::Loaded {
                            file: Box::new(file),
                            raw,
                        },
                        summary,
                    )
                }
            },
        };
        Self {
            path: Some(path),
            content,
            validation_summary,
            scroll: 0,
        }
    }

    /// Open a blank editor for a new narrative.
    pub fn new_file() -> Self {
        Self {
            path: None,
            content: EditorContent::New,
            validation_summary: "(new file — not yet saved)".to_string(),
            scroll: 0,
        }
    }

    fn act_rows(&self) -> Vec<String> {
        match &self.content {
            EditorContent::New => vec!["  (blank narrative — press Ctrl+S to save)".to_string()],
            EditorContent::Error(msg) => vec![format!("  ✗ {msg}")],
            EditorContent::Loaded { file, .. } => {
                if file.acts.is_empty() {
                    vec!["  (no acts defined)".to_string()]
                } else {
                    let mut rows: Vec<String> = file
                        .acts
                        .keys()
                        .map(|name| format!("  act: {name}"))
                        .collect();
                    rows.sort();
                    rows
                }
            }
        }
    }

    fn title_line(&self) -> String {
        match &self.path {
            None => "New narrative".to_string(),
            Some(p) => p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        }
    }

    /// Serialize back to TOML and emit a `SaveNarrative` transition.
    fn do_save(&self) -> BotTransition {
        let Some(path) = self.path.clone() else {
            // No path yet — would need a filename prompt (future work).
            return BotTransition::Stay;
        };
        let toml = match &self.content {
            EditorContent::Loaded { raw, .. } => raw.clone(),
            EditorContent::New => String::new(),
            EditorContent::Error(_) => return BotTransition::Stay,
        };
        BotTransition::SaveNarrative { path, toml }
    }
}

impl BotScreen for NarrativeEditorScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let title = self.title_line();
        let header = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: false,
                scroll: None,
                alignment: None,
                block: Some(BlockJson {
                    title: Some(title),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        };

        let rows = self.act_rows();
        let visible = rows.iter().skip(self.scroll);

        let mut constraints = vec![ConstraintJson::Length { value: 3 }];
        let mut children = vec![header];

        for row in visible {
            constraints.push(ConstraintJson::Length { value: 1 });
            children.push(TuiNode::Widget {
                widget: Box::new(WidgetJson::Paragraph {
                    text: ParagraphText::Plain(row.clone()),
                    style: None,
                    wrap: false,
                    scroll: None,
                    alignment: None,
                    block: None,
                }),
            });
        }

        // Filler to push status + help to the bottom.
        constraints.push(ConstraintJson::Fill { value: 1 });
        children.push(TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: false,
                scroll: None,
                alignment: None,
                block: None,
            }),
        });

        // Validation status.
        constraints.push(ConstraintJson::Length { value: 1 });
        children.push(TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(format!("  {}", self.validation_summary)),
                style: None,
                wrap: false,
                scroll: None,
                alignment: None,
                block: None,
            }),
        });

        // Help row.
        constraints.push(ConstraintJson::Length { value: 1 });
        children.push(TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain("  j/k=scroll  Ctrl+S=save  Esc=browser".to_string()),
                style: None,
                wrap: false,
                scroll: None,
                alignment: None,
                block: None,
            }),
        });

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            KeyCode::Esc => BotTransition::GoToNarratives,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => self.do_save(),
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                BotTransition::Stay
            }
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Narrative Editor"
    }
}
