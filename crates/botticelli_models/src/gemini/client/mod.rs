//! Google Gemini API client implementation.
//!
//! This module provides a client for the Google Gemini API with support for:
//! - Per-request model selection (different requests can use different models)
//! - Client pooling with lazy initialization (one client per model)
//! - Per-model rate limiting (each model has independent rate limits)
//! - Thread-safe concurrent access

mod core;
mod driver;
mod metadata;
mod streaming;
mod tiered;
mod token_counting;
mod tool_calling;
mod vision;

pub use core::GeminiClient;
pub use tiered::TieredGemini;
