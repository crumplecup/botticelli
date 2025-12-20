//! Botticelli - Unified LLM API Interface
//!
//! Botticelli provides a unified, trait-based interface for interacting with multiple
//! Large Language Model (LLM) APIs. It supports multimodal inputs/outputs, narrative
//! execution workflows, and content management.
//!
//! # Features
//!
//! - **Unified Interface**: Single `BotticelliDriver` trait for all LLM providers
//! - **Multimodal Support**: Text, images, audio, video, and documents
//! - **Narrative System**: Multi-step LLM workflows with TOML-based narratives
//! - **Rate Limiting**: Automatic rate limiting and retry with exponential backoff
//! - **Database Integration**: PostgreSQL persistence for narratives and content
//! - **Social Platforms**: Discord bot integration
//! - **Terminal UI**: Interactive content review and management
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use botticelli_models::GeminiClient;
//! use botticelli_interface::BotticelliDriver;
//! use botticelli_core::{GenerateRequest, Input, Message, Role};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = GeminiClient::new()?;
//!
//!     let message = Message::new(Role::User, vec![Input::Text("Hello!".to_string())]);
//!     let request = GenerateRequest::new(vec![message]);
//!
//!     let response = client.generate(&request).await?;
//!     println!("Response: {:?}", response);
//!     Ok(())
//! }
//! ```
//!
//! # Cargo Features
//!
//! - `gemini` - Google Gemini API support
//! - `anthropic` - Anthropic Claude API support
//! - `huggingface` - HuggingFace Inference API support
//! - `groq` - Groq LPU Inference API support
//! - `ollama` - Ollama local LLM support
//! - `database` - PostgreSQL database integration
//! - `discord` - Discord bot integration
//! - `tui` - Terminal user interface
//! - `all` - Enable all features
//!
//! # Architecture
//!
//! Botticelli is organized as a workspace with focused crates.
//! Import directly from the crate you need:
//!
//! ```rust,ignore
//! use botticelli_core::{GenerateRequest, Message};
//! use botticelli_interface::{BotticelliDriver, ToolCalling};
//! use botticelli_models::AnthropicClient;
//! ```
//!
//! Available crates:
//! - `botticelli_core` - Core data types
//! - `botticelli_interface` - Trait definitions
//! - `botticelli_error` - Error types
//! - `botticelli_models` - LLM provider implementations
//! - `botticelli_narrative` - Narrative execution engine
//! - `botticelli_database` - PostgreSQL integration
//! - `botticelli_social` - Social platform integrations
//! - `botticelli_tui` - Terminal UI

// This is a facade crate - Cargo.toml declares all dependencies.
// Users should import directly from the crate they need.
// No re-exports needed here.
