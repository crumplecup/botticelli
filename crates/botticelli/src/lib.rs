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
//! use botticelli::{GeminiClient, BotticelliDriver, Input, GenerateRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = GeminiClient::new(std::env::var("GEMINI_API_KEY")?)?;
//!
//!     let request = GenerateRequest {
//!         inputs: vec![Input::Text("Hello, world!".to_string())],
//!         ..Default::default()
//!     };
//!
//!     let response = client.generate(request).await?;
//!     println!("Response: {:?}", response);
//!     Ok(())
//! }
//! ```
//!
//! # Cargo Features
//!
//! - `gemini` - Google Gemini API support
//! - `database` - PostgreSQL database integration
//! - `discord` - Discord bot integration
//! - `tui` - Terminal user interface
//! - `all` - Enable all features
//!
//! # Architecture
//!
//! Botticelli is organized as a workspace with focused crates:
//!
//! - `botticelli-core` - Core data types (Input, Output, etc.)
//! - `botticelli-interface` - BotticelliDriver trait definition
//! - `botticelli-error` - Error types
//! - `botticelli-rate-limit` - Rate limiting and retry logic
//! - `botticelli-storage` - Content-addressable storage
//! - `botticelli-models` - LLM provider implementations
//! - `botticelli-narrative` - Narrative execution engine
//! - `botticelli-database` - PostgreSQL integration
//! - `botticelli-social` - Social platform integrations
//! - `botticelli-tui` - Terminal UI
//!
//! This crate (`botticelli`) re-exports everything for convenience.

// Re-export core crates (always available)
pub use botticelli_core::*;
pub use botticelli_error::*;
pub use botticelli_interface::*;
pub use botticelli_narrative::{
    ActConfig,
    ActProcessor,
    InMemoryNarrativeRepository,
    MultiNarrative,
    Narrative,
    NarrativeExecutor,
    NarrativeMetadata,
    NarrativeProvider,
    NarrativeToc,
    ProcessorContext,
    ProcessorRegistry,
    // Note: BotCommandRegistry trait NOT re-exported to avoid ambiguity
    // Use botticelli_narrative::BotCommandRegistry for the trait
    // Use botticelli_social::BotCommandRegistry for the implementation
};
pub use botticelli_rate_limit::*;
pub use botticelli_storage::*;

// Re-export optional crates based on features
#[cfg(feature = "gemini")]
pub use botticelli_models::*;

#[cfg(feature = "database")]
pub use botticelli_database::*;

#[cfg(feature = "discord")]
pub use botticelli_social::*;

#[cfg(feature = "tui")]
pub use botticelli_tui::*;

// OpenTelemetry telemetry module
#[cfg(feature = "observability")]
pub mod telemetry;
