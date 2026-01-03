//! Google Gemini API client implementation.
//!
//! This module provides a client for the Google Gemini API with support for:
//! - Per-request model selection (different requests can use different models)
//! - Client pooling with lazy initialization (one client per model)
//! - Per-model rate limiting (each model has independent rate limits)
//! - Thread-safe concurrent access

mod tiered;
mod core;
mod driver;
mod tool_calling;
mod streaming;
mod metadata;
mod vision;
mod token_counting;

pub use tiered::TieredGemini;
pub use core::GeminiClient;
