//! Terminal User Interface for Botticelli.
//!
//! # Architecture (elicit_ui IR pipeline)
//!
//! Screens implement [`BotScreen`]: produce a [`elicit_ratatui::TuiNode`] and
//! handle key events returning a [`BotTransition`]. The [`BotController`]
//! drives the state machine — it renders each frame via `verified_draw`
//! (which carries `Established<BotUiConsistent>`) and applies transitions.
//! Screens own their own state; no global `AppState`.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod context;
pub mod contracts;
pub mod controller;
pub mod screen;
pub mod screens;

pub use context::BotScreenContext;
pub use contracts::{BotUiConsistent, LayoutError, verified_draw};
pub use controller::BotController;
pub use error::{TuiError, TuiErrorKind, TuiResult};
pub use screen::{BotKind, BotScreen, BotTransition};

mod error;
