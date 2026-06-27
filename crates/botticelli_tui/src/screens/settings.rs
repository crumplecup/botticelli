//! Settings screen — view and adjust runtime configuration.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

const LOG_LEVELS: &[&str] = &["info", "debug", "warn", "error"];

const SETTING_COUNT: usize = 4;

/// Indices into the settings list.
const IDX_LOG_LEVEL: usize = 0;
const IDX_NARRATIVES_DIR: usize = 1;
const IDX_LOG_FILE: usize = 2;
const IDX_STORAGE_PATH: usize = 3;

/// Settings screen — shows current configuration with one editable field.
///
/// **Log Level** (selected with `j`/`k`, cycled with `Enter`): cycles through
/// `info` / `debug` / `warn` / `error`. Press `s` to persist the chosen level
/// to `botticelli-settings.env` via the `SaveSettings` transition.
///
/// All other fields are read-only and reflect values passed in at construction.
pub struct SettingsScreen {
    selected: usize,
    log_level_idx: usize,
    narratives_dir: String,
    log_file: String,
    storage_path: String,
    /// Feedback message shown after a save.
    save_feedback: Option<&'static str>,
}

impl SettingsScreen {
    /// Create the screen pre-filled with current configuration values.
    pub fn new(
        narratives_dir: Option<String>,
        log_file: Option<String>,
        storage_path: Option<String>,
    ) -> Self {
        let current_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        let log_level_idx = LOG_LEVELS
            .iter()
            .position(|l| current_log.contains(*l))
            .unwrap_or(0);
        Self {
            selected: 0,
            log_level_idx,
            narratives_dir: narratives_dir.unwrap_or_else(|| "(not set)".to_string()),
            log_file: log_file.unwrap_or_else(|| "(not set)".to_string()),
            storage_path: storage_path.unwrap_or_else(|| "(not set)".to_string()),
            save_feedback: None,
        }
    }

    /// Currently selected setting row.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Current log level selection (index into `info/debug/warn/error`).
    pub fn log_level_idx(&self) -> usize {
        self.log_level_idx
    }

    /// Current log level label.
    pub fn log_level(&self) -> &'static str {
        LOG_LEVELS[self.log_level_idx]
    }

    fn label_for(idx: usize) -> &'static str {
        match idx {
            IDX_LOG_LEVEL => "Log Level",
            IDX_NARRATIVES_DIR => "Narratives Dir",
            IDX_LOG_FILE => "Log File",
            IDX_STORAGE_PATH => "Storage Path",
            _ => "Unknown",
        }
    }

    fn value_for(&self, idx: usize) -> String {
        match idx {
            IDX_LOG_LEVEL => {
                format!(
                    "{} (Enter to cycle: {})",
                    LOG_LEVELS[self.log_level_idx],
                    LOG_LEVELS.join(" / ")
                )
            }
            IDX_NARRATIVES_DIR => self.narratives_dir.clone(),
            IDX_LOG_FILE => self.log_file.clone(),
            IDX_STORAGE_PATH => self.storage_path.clone(),
            _ => String::new(),
        }
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

impl BotScreen for SettingsScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let header = Self::paragraph("", Some("Settings".to_string()));

        let mut children = vec![header];
        let mut constraints = vec![ConstraintJson::Length { value: 3 }];

        for i in 0..SETTING_COUNT {
            let cursor = if i == self.selected { "▶" } else { " " };
            let label = Self::label_for(i);
            let value = self.value_for(i);
            children.push(Self::paragraph(
                format!("{}  {:<18}  {}", cursor, label, value),
                None,
            ));
            constraints.push(ConstraintJson::Length { value: 1 });
        }

        // feedback row
        children.push(Self::paragraph("", None));
        constraints.push(ConstraintJson::Length { value: 1 });

        let feedback = self.save_feedback.unwrap_or("");
        children.push(Self::paragraph(format!("  {}", feedback), None));
        constraints.push(ConstraintJson::Length { value: 1 });

        // fill
        children.push(Self::paragraph("", None));
        constraints.push(ConstraintJson::Fill { value: 1 });

        let help = Self::paragraph(
            "  j/k=navigate  Enter=edit log level  s=save  1=bots  2=chat  q=quit",
            None,
        );
        children.push(help);
        constraints.push(ConstraintJson::Length { value: 1 });

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        // Clear feedback on any new key
        self.save_feedback = None;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1) % SETTING_COUNT;
                BotTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(SETTING_COUNT - 1);
                BotTransition::Stay
            }
            KeyCode::Enter => {
                if self.selected == IDX_LOG_LEVEL {
                    self.log_level_idx = (self.log_level_idx + 1) % LOG_LEVELS.len();
                }
                BotTransition::Stay
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let rust_log = LOG_LEVELS[self.log_level_idx].to_string();
                self.save_feedback = Some("Saved to botticelli-settings.env (restart to apply)");
                BotTransition::SaveSettings { rust_log }
            }
            KeyCode::Char('s') => {
                let rust_log = LOG_LEVELS[self.log_level_idx].to_string();
                self.save_feedback = Some("Saved to botticelli-settings.env (restart to apply)");
                BotTransition::SaveSettings { rust_log }
            }
            KeyCode::Char('1') => BotTransition::GoToBots,
            KeyCode::Char('2') => BotTransition::GoToChat,
            KeyCode::Char('3') => BotTransition::GoToNarratives,
            KeyCode::Char('4') => BotTransition::GoToDatabase,
            KeyCode::Char('5') => BotTransition::GoToSchedule,
            KeyCode::Char('6') => BotTransition::GoToLogViewer,
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Settings"
    }
}
