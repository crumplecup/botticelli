//! Narrative execution engine for Botticelli.
//!
//! This crate provides the narrative execution system that orchestrates
//! multi-step LLM interactions based on TOML-defined narratives.
//!
//! # Features
//!
//! - **TOML-based narratives**: Define complex multi-act interactions
//! - **Processor system**: Extract and process structured data from responses
//! - **Repository abstraction**: Persist executions with pluggable backends
//! - **In-memory execution**: Run narratives without persistence
//! - **Database integration**: Optional PostgreSQL persistence (with `database` feature)
//!
//! # Example
//!
//! ```rust,ignore
//! use botticelli_narrative::{Narrative, NarrativeExecutor, InMemoryNarrativeRepository};
//! use botticelli_models::GeminiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
#![recursion_limit = "2048"]
//! // Load narrative from TOML
//! let narrative = Narrative::from_file("narrative.toml")?;
//!
//! // Create executor with Gemini driver
//! let client = GeminiClient::new("api-key")?;
//! let executor = NarrativeExecutor::new(client);
//!
//! // Execute narrative
//! let execution = executor.execute(&narrative).await?;
//! println!("Completed {} acts", execution.act_executions.len());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod carousel;
mod core;
mod executor;
mod history_retention;
mod in_memory_repository;
mod multi_narrative;
mod processor;
mod provider;
mod state;
mod table_reference;
mod toml_parser;
pub mod validator;

#[cfg(feature = "database")]
mod content_generation;

#[cfg(feature = "database")]
mod extraction;

#[cfg(feature = "database")]
mod storage_actor;

pub use carousel::{CarouselConfig, CarouselResult, CarouselState};
pub use core::{Narrative, NarrativeMetadata, NarrativeSource, NarrativeToc};
pub use executor::{BotCommandRegistry, NarrativeExecutor};
pub use history_retention::{
    AUTO_SUMMARY_THRESHOLD, apply_retention_to_inputs, should_auto_summarize, summarize_input,
};
pub use in_memory_repository::InMemoryNarrativeRepository;
pub use multi_narrative::MultiNarrative;
pub use processor::{ActProcessor, ProcessorContext, ProcessorRegistry};
pub use provider::{ActConfig, NarrativeProvider};
pub use state::{NarrativeState, StateManager, StateScope};
pub use table_reference::TableReference;

#[cfg(feature = "database")]
pub use content_generation::ContentGenerationProcessor;

#[cfg(feature = "database")]
pub use extraction::{extract_json, extract_toml, parse_json, parse_toml};

#[cfg(feature = "database")]
pub use storage_actor::{StorageActor, StorageActorState, StorageMessage};
