//! Repository traits for content and data management.

mod content;
mod content_generation;
mod narrative;
mod table_query;
mod table_view;

pub use content::ContentRepository;
pub use content_generation::ContentGenerationRepository;
pub use narrative::NarrativeRepository;
pub use table_query::TableQueryRegistry;
pub use table_view::TableView;
