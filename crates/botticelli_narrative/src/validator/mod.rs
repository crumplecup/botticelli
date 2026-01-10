//! Narrative TOML validation with actionable error messages.
//!
//! This module provides comprehensive validation for narrative TOML files,
//! catching common syntax errors and providing specific fix suggestions.

mod analysis;
mod core;
mod extraction;
mod models;
mod resources;
mod structure;
mod syntax;

// Re-export validation types from botticelli_error
pub use botticelli_error::{
    ValidationError, ValidationErrorKind, ValidationLocation, ValidationResult, ValidationWarning,
    ValidationWarningKind,
};

// Re-export helper types
pub use analysis::Analyzer;
pub use extraction::DataExtractor;
pub use models::ModelValidator;
pub use resources::{ResourceRegistry, ResourceValidator};
pub use structure::StructureValidator;
pub use syntax::SyntaxValidator;

// Re-export core types and functions
pub use core::{
    validate_narrative_file, validate_narrative_file_with_config, validate_narrative_toml,
    validate_narrative_toml_with_config, ValidationConfig,
};
