//! Error types for narrative operations.

/// Specific error conditions for narrative operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum NarrativeErrorKind {
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
        /// Name of the act that failed
        act: String,
        /// Error message from prompt assembly
        message: String,
    },
}

/// Error type for narrative operations.
#[derive(Debug, Clone, derive_new::new, derive_more::Display)]
#[display("Narrative Error: {} at line {} in {}", kind, line, file)]
pub struct NarrativeError {
    /// The specific error condition
    pub kind: NarrativeErrorKind,
    /// Line number where the error occurred
    pub line: u32,
    /// Source file where the error occurred
    pub file: &'static str,
}

impl std::error::Error for NarrativeError {}
