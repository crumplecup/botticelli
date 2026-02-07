//! Narrative error types.

/// Specific error conditions for narrative operations.
#[cfg(feature = "mcp")]
use crate::tool;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum NarrativeErrorKind {
    /// I/O error (file read/write)
    #[display("I/O error: {}", _0)]
    Io(crate::IoError),
    /// JSON serialization/deserialization error
    #[display("JSON error: {}", _0)]
    Json(crate::JsonError),
    /// TOML deserialization error
    #[display("TOML error: {}", _0)]
    Toml(crate::TomlError),
    /// Failed to read narrative file
    #[display("Failed to read narrative file: {}", _0)]
    FileRead(String),
    /// Failed to parse TOML content
    #[display("Failed to parse TOML: {}", _0)]
    TomlParse(String),
    /// Table of contents is empty
    #[display("Table of contents (toc.order) cannot be empty")]
    EmptyToc,
    /// Act referenced in table of contents does not exist in acts map
    #[display("Act '{}' referenced in toc.order does not exist in acts map", _0)]
    MissingAct(String),
    /// Act prompt is empty or contains only whitespace
    #[display("Act '{}' has an empty prompt", _0)]
    EmptyPrompt(String),
    /// Template field required but not set
    #[display("Template field is required for prompt assembly")]
    MissingTemplate,
    /// Failed to assemble prompt with schema injection
    #[display("Failed to assemble prompt for act '{}': {}", act, message)]
    PromptAssembly {
        /// Act name
        act: String,
        /// Error message
        message: String,
    },
    /// Bot command registry not configured
    #[display("Bot command not configured: {}", _0)]
    BotCommandNotConfigured(String),
    /// Bot command execution failed
    #[display("Bot command failed: {}", _0)]
    BotCommandFailed(String),
    /// Table query registry not configured
    #[display("Table query not configured: {}", _0)]
    TableQueryNotConfigured(String),
    /// Table query execution failed
    #[display("Table query failed: {}", _0)]
    TableQueryFailed(String),
    /// Serialization error
    #[display("Serialization error: {}", _0)]
    SerializationError(String),
    /// Carousel budget exhausted
    #[display(
        "Carousel budget exhausted after {completed_iterations} of {max_iterations} iterations"
    )]
    CarouselBudgetExhausted {
        /// Completed iterations
        completed_iterations: u32,
        /// Maximum iterations requested
        max_iterations: u32,
    },
    /// Configuration error
    #[display("Configuration error: {}", _0)]
    ConfigurationError(String),
    /// Narrative not found in file
    #[display("Narrative '{}' not found. Available narratives: {}", name, available)]
    NarrativeNotFound {
        /// Requested narrative name
        name: String,
        /// Available narrative names
        available: String,
    },
    /// Ambiguous narrative selection - multiple narratives exist but no name provided
    #[display("Multiple narratives found, must specify one: {}", available)]
    AmbiguousNarrative {
        /// Available narrative names
        available: String,
    },
    /// No narrative found in file
    #[display("No narrative definition found in TOML file")]
    NoNarrativeFound,
    /// Missing required field in TOML configuration
    #[display("Missing required field '{}' in {} input", field, input_type)]
    MissingRequiredField {
        /// Field name
        field: String,
        /// Input type (text, bot_command, table, etc.)
        input_type: String,
    },
    /// Invalid value for field
    #[display("Invalid value '{}' for field '{}': {}", value, field, reason)]
    InvalidFieldValue {
        /// The invalid value
        value: String,
        /// Field name  
        field: String,
        /// Reason why it's invalid
        reason: String,
    },
    /// Invalid reference format
    #[display("Invalid reference format '{}': {}", reference, reason)]
    InvalidReferenceFormat {
        /// The reference string
        reference: String,
        /// Explanation of what's wrong
        reason: String,
    },
    /// Wrong entry type found
    #[display(
        "Reference '{}' resolves to {} but expected {}",
        reference,
        found,
        expected
    )]
    WrongReferenceType {
        /// The reference string
        reference: String,
        /// What was found (e.g., "inline definition")
        found: String,
        /// What was expected (e.g., "file reference")
        expected: String,
    },
    /// Resource not found (bot, table, media definition)
    #[display("{} '{}' not found in shared resources", resource_type, name)]
    ResourceNotFound {
        /// Type of resource (bot, table, media)
        resource_type: String,
        /// Name that was requested
        name: String,
    },
    /// Template resolution error
    #[display("Template error: {}", _0)]
    TemplateError(String),
    /// Nested narrative load failed
    #[display("Nested narrative load failed: {}", _0)]
    NestedNarrativeLoadFailed(String),
    /// Nested narrative execution failed
    #[display("Nested narrative execution failed: {}", _0)]
    NestedNarrativeExecutionFailed(String),
    /// State management error
    #[display("State error: {}", _0)]
    StateError(String),
    /// Feature not implemented
    #[display("Not implemented: {}", _0)]
    NotImplemented(String),
    /// Invalid TOML syntax pattern detected during validation
    #[display("Invalid TOML syntax: {}", _0)]
    InvalidSyntax(String),
    /// Missing required TOML section during validation
    #[display("Missing required section: {}", _0)]
    MissingSection(String),
    /// Undefined resource reference in act
    #[display("Undefined reference '{}' in act '{}'", reference, act)]
    UndefinedReference {
        /// The reference string (e.g., "bots.my_bot")
        reference: String,
        /// Act name where reference was found
        act: String,
    },
    /// Circular dependency detected in narrative references
    #[display("Circular dependency: {}", _0)]
    CircularDependency(String),
}

/// Error type for narrative operations.
///
/// # Examples
///
/// ```
/// use botticelli_error::{NarrativeError, NarrativeErrorKind};
///
/// let err = NarrativeError::new(NarrativeErrorKind::EmptyToc);
/// assert!(format!("{}", err).contains("empty"));
/// ```
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Narrative Error: {} at line {} in {}", kind, line, file)]
pub struct NarrativeError {
    /// The specific error condition
    kind: NarrativeErrorKind,
    /// Line number where the error occurred
    line: u32,
    /// Source file where the error occurred
    file: String,
}

impl NarrativeError {
    /// Create a new NarrativeError with automatic location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: NarrativeErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

impl From<std::io::Error> for NarrativeError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Self::new(NarrativeErrorKind::Io(crate::IoError::from(err)))
    }
}

impl From<serde_json::Error> for NarrativeError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(NarrativeErrorKind::Json(crate::JsonError::from(err)))
    }
}

impl From<toml::de::Error> for NarrativeError {
    #[track_caller]
    fn from(err: toml::de::Error) -> Self {
        Self::new(NarrativeErrorKind::Toml(crate::TomlError::from(err)))
    }
}

crate::impl_error_from_kind!(NarrativeErrorKind => NarrativeError);

/// Result type for narrative operations.
pub type NarrativeResult<T> = Result<T, NarrativeError>;
