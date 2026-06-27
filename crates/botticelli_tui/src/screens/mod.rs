//! Screen implementations for the botticelli operator console.

pub mod bots;
pub mod chat;
pub mod database;
pub mod log_viewer;
pub mod narrative_browser;
pub mod narrative_editor;
pub mod narrative_wizard;
pub mod placeholder;
pub mod schedule;
pub mod settings;

pub use bots::{BotStatusScreen, RunState};
pub use chat::{ChatMessage, ChatRole, ChatScreen, ModelStatus};
pub use database::DatabaseBrowserScreen;
pub use log_viewer::{LogFilter, LogViewerScreen};
pub use narrative_browser::{NarrativeBrowserScreen, NarrativeEntry};
pub use narrative_editor::{EditorContent, NarrativeEditorScreen};
pub use narrative_wizard::NarrativeWizardScreen;
pub use placeholder::PlaceholderScreen;
pub use schedule::ScheduleScreen;
pub use settings::SettingsScreen;
