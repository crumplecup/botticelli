//! Terminal User Interface for Botticelli.
//!
//! Provides an interactive TUI for chat, narrative management, and settings.
//! Built with ratatui for terminal rendering and tokio for async operations.
//!
//! ## Architecture
//!
//! The TUI follows a clean separation of concerns:
//!
//! - **State** ([`AppState`]): Central application state with view modes
//! - **Views** ([`View`]): Composable UI components (chat, narrative browser, editor)
//! - **Events** ([`Event`], [`EventHandler`]): Input handling and async event loop
//! - **Commands** ([`Command`]): User actions that modify state
//! - **Rendering**: Decoupled from state via ratatui's immediate mode
//!
//! ## View Modes
//!
//! - **Chat**: Interactive conversation with LLM (streaming responses)
//! - **Narrative Browser**: Browse and manage narrative templates
//! - **Narrative Editor**: Edit narrative structure and sections
//! - **Settings**: Configure model, temperature, and system prompt
//!
//! ## Key Features
//!
//! - Async streaming LLM responses with visual feedback
//! - Real-time syntax highlighting for code blocks
//! - Keyboard-driven navigation (Vim-style and arrow keys)
//! - Responsive layout with status bar and help text
//! - Integration with MCP tool calling and narrative generation

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod app;
mod commands;
mod elicitation;
mod elicitation_dialog;
mod error;
mod events;
mod state;
mod tui;
mod view;

pub use app::TuiApp;
pub use commands::Command;
pub use elicitation_dialog::TuiElicitationDialog;
pub use error::{TuiError, TuiErrorKind, TuiResult};
pub use events::{Event, EventHandler, McpError, McpMessage, McpUpdate};
pub use state::{AppState, ChatMessage, ConversationId, NarrativeId, TuiLlmBackend, ViewMode};
pub use tui::Tui;
pub use view::{ChatView, NarrativeBrowserView, NarrativeEditorView, View};
