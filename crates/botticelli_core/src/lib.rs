//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod input;
mod media;
mod message;
mod output;
mod request;
mod role;

pub use input::Input;
pub use media::MediaSource;
pub use message::Message;
pub use output::{Output, ToolCall};
pub use request::{GenerateRequest, GenerateResponse};
pub use role::Role;
