#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Chat interface for user/botticelli interaction.
//!
//! Provides trait-based abstraction for building chat interfaces across
//! different platforms (TUI, web, mobile). Enables users to direct
//! botticelli on tasks like narrative generation, bot assignment, and
//! social media scheduling.

mod command;
mod error;
mod input;
mod interface;
mod message;
mod parser;
mod response;

pub use command::{BotCommand, Command, NarrativeCommand, SocialCommand};
pub use error::{ChatError, ChatErrorKind, ChatResult};
pub use input::UserInput;
pub use interface::ChatInterface;
pub use message::Message;
pub use parser::parse_intent;
pub use response::Response;
