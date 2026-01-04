//! Generic OpenAI-compatible API client.
//!
//! This module provides a reusable client for any API that follows the OpenAI chat completions format.
//! Used by HuggingFace, Groq, and potentially future providers like OpenAI, Perplexity, etc.

mod client;
mod conversions;
mod dto;
mod provider_impl;
mod tool_calling_impl;

pub use client::OpenAICompatibleClient;
pub use dto::{ChatFunctionDef, ChatMessage, ChatRequest, ChatResponse, ChatTool};

