//! Google Gemini API client implementation.
//!
//! This module provides two clients for the Gemini API:
//! - [`GeminiClient`] - REST API client for standard requests
//! - [`GeminiLiveClient`] - WebSocket client for Live API (bidirectional streaming)
//!
//! # REST API Client
//!
//! The REST API client supports:
//! - Per-request model selection
//! - Client pooling with lazy initialization
//! - Per-model rate limiting
//! - Thread-safe concurrent access
//! - SSE streaming (via `Streaming` trait)
//!
//! # Live API Client
//!
//! The Live API client supports:
//! - WebSocket bidirectional streaming
//! - Real-time audio/video interactions
//! - Text chat
//! - Better rate limits on free tier

mod client;
mod live_client;
mod live_protocol;
mod live_rate_limit;

pub use client::{GeminiClient, TieredGemini};
pub use live_client::{GeminiLiveClient, LiveSession};
pub use live_protocol::{
    ClientContent, ClientContentMessage, FunctionCall, FunctionResponse, GenerationConfig, GoAway,
    InlineData, InlineDataPart, LiveToolCall, LiveToolCallCancellation, MediaChunk, ModelTurn,
    Part, RealtimeInput, RealtimeInputMessage, ServerContent, ServerMessage, SetupComplete,
    SetupConfig, SetupMessage, SystemInstruction, TextPart, Tool, ToolResponse,
    ToolResponseMessage, Turn, UsageMetadata,
};
pub use live_rate_limit::LiveRateLimiter;

/// Result type for Gemini operations.
pub type GeminiResult<T> = Result<T, botticelli_error::GeminiError>;
