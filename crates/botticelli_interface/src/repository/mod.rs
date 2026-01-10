//! Repository traits for content and data management.

mod act_processor;
mod bot_command;
mod content;
mod content_generation;
mod narrative;
mod narrative_provider;
mod table_query;
mod table_view;

pub use act_processor::{ActProcessor, ProcessorTrait};
pub use bot_command::BotCommandRegistry;
pub use content::ContentRepository;
pub use content_generation::ContentGenerationRepository;
pub use narrative::NarrativeRepository;
pub use narrative_provider::NarrativeProvider;
pub use table_query::TableQueryRegistry;
pub use table_view::TableView;
