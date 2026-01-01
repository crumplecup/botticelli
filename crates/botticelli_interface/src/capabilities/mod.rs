//! Optional capability traits for LLM backends.

mod audio;
mod batch;
mod documents;
mod embeddings;
mod json_mode;
mod streaming;
mod tokens;
mod tools;
mod video;
mod vision;

pub use audio::Audio;
pub use batch::BatchGeneration;
pub use documents::DocumentProcessing;
pub use embeddings::Embeddings;
pub use json_mode::JsonMode;
pub use streaming::Streaming;
pub use tokens::TokenCounting;
pub use tools::ToolCalling;
pub use video::Video;
pub use vision::Vision;
