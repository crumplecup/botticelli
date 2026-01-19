//! MCP-specific error types.

use std::sync::Arc;

/// Serde JSON error with source tracking.
#[cfg(feature = "serde_json")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("JSON serialization error: {:?} at {}:{}", source, file, line)]
pub struct SerdeJsonError {
    /// The serde_json error source
    source: Box<serde_json::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

/// Database error with source tracking.
#[cfg(feature = "database")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Database error: {} - {:?} at {}:{}", message, source, file, line)]
pub struct DatabaseMcpError {
    /// Error message
    message: String,
    /// The database error source
    source: Box<crate::DatabaseError>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "serde_json")]
impl SerdeJsonError {
    /// Create a new SerdeJsonError with automatic location tracking.
    #[track_caller]
    pub fn new(err: serde_json::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Box::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "database")]
impl DatabaseMcpError {
    /// Create a new DatabaseMcpError with automatic location tracking.
    #[track_caller]
    pub fn new(message: impl Into<String>, err: crate::DatabaseError) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            source: Box::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "serde_json")]
impl Clone for SerdeJsonError {
    fn clone(&self) -> Self {
        // serde_json::Error doesn't implement Clone, recreate from message
        // We use io::Error as a workaround since serde_json::Error can wrap it
        let message = self.source.to_string();
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, message);
        let json_err = serde_json::Error::io(io_err);
        Self {
            source: Box::new(json_err),
            line: self.line,
            file: self.file,
        }
    }
}

#[cfg(feature = "database")]
impl Clone for DatabaseMcpError {
    fn clone(&self) -> Self {
        // DatabaseError implements Clone, so this is straightforward
        Self {
            message: self.message.clone(),
            source: self.source.clone(),
            line: self.line,
            file: self.file,
        }
    }
}

#[cfg(feature = "serde_json")]
impl PartialEq for SerdeJsonError {
    fn eq(&self, other: &Self) -> bool {
        // Compare by string representation since serde_json::Error doesn't impl PartialEq
        self.source.to_string() == other.source.to_string()
            && self.line == other.line
            && self.file == other.file
    }
}

#[cfg(feature = "database")]
impl PartialEq for DatabaseMcpError {
    fn eq(&self, other: &Self) -> bool {
        // DatabaseError implements PartialEq
        self.message == other.message
            && self.source == other.source
            && self.line == other.line
            && self.file == other.file
    }
}

#[cfg(feature = "serde_json")]
impl Eq for SerdeJsonError {}

#[cfg(feature = "database")]
impl Eq for DatabaseMcpError {}

#[cfg(feature = "serde_json")]
impl std::hash::Hash for SerdeJsonError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.to_string().hash(state);
        self.line.hash(state);
        self.file.hash(state);
    }
}

#[cfg(feature = "database")]
impl std::hash::Hash for DatabaseMcpError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // DatabaseError implements Hash
        self.message.hash(state);
        self.source.hash(state);
        self.line.hash(state);
        self.file.hash(state);
    }
}

/// MCP error kinds.
#[derive(Debug, Clone, derive_more::Display)]
pub enum McpErrorKind {
    /// JSON serialization/deserialization error
    #[cfg(feature = "serde_json")]
    #[display("JSON error: {}", _0)]
    Json(SerdeJsonError),

    /// Database error
    #[cfg(feature = "database")]
    #[display("Database error: {}", _0)]
    Database(DatabaseMcpError),

    /// Narrative error
    #[display("Narrative error: {}", _0)]
    Narrative(crate::NarrativeError),

    /// RMCP protocol error (message + code)
    #[display("RMCP error: {} (code {})", message, code)]
    Rmcp {
        /// Error code
        code: i32,
        /// Error message
        message: String,
    },

    /// Tool not found
    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),
    /// Invalid tool arguments
    #[display("Invalid arguments for tool {}: {}", tool, reason)]
    InvalidArguments {
        /// Tool name
        tool: String,
        /// Reason for invalidity
        reason: String,
    },
    /// Invalid input
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    /// Tool execution failed
    #[display("Tool execution failed: {}", _0)]
    ExecutionFailed(String),
    /// Tool execution failed (alias)
    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),
    /// Execution error (alias)
    #[display("Execution error: {}", _0)]
    ExecutionError(String),
    /// Invalid state for operation
    #[display("Invalid state: {}", _0)]
    InvalidState(String),
    /// Session error
    #[display("Session error: {}", _0)]
    Session(String),
    /// Resource not found
    #[display("Resource not found: {}", _0)]
    ResourceNotFound(String),
    /// Backend unavailable
    #[display("Backend unavailable: {}", _0)]
    BackendUnavailable(String),
    /// Unsupported model
    #[display("Unsupported model: {}", _0)]
    UnsupportedModel(String),
    /// Mutex poisoned (internal error)
    #[display("Mutex poisoned: {}", _0)]
    MutexPoisoned(String),

    /// Parse int error with source
    #[display("Parse error: {}", message)]
    ParseInt {
        /// Error message
        message: String,
        /// Source error
        source: Arc<std::num::ParseIntError>,
    },

    /// Environment variable error with source
    #[display("Environment variable error: {}", message)]
    EnvVar {
        /// Error message
        message: String,
        /// Source error
        source: Arc<std::env::VarError>,
    },

    /// IO error with source
    #[display("IO error: {}", message)]
    Io {
        /// Error message
        message: String,
        /// Source error
        source: Arc<std::io::Error>,
    },
}

/// From implementations for automatic error conversion
#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for McpErrorKind {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(SerdeJsonError::new(err))
    }
}

#[cfg(feature = "serde_json")]
impl From<SerdeJsonError> for McpErrorKind {
    fn from(err: SerdeJsonError) -> Self {
        Self::Json(err)
    }
}

#[cfg(feature = "database")]
impl From<crate::DatabaseError> for McpErrorKind {
    fn from(err: crate::DatabaseError) -> Self {
        Self::Database(DatabaseMcpError::new("Database error", err))
    }
}

#[cfg(feature = "database")]
impl From<DatabaseMcpError> for McpErrorKind {
    fn from(err: DatabaseMcpError) -> Self {
        Self::Database(err)
    }
}

impl From<crate::NarrativeError> for McpErrorKind {
    fn from(err: crate::NarrativeError) -> Self {
        Self::Narrative(err)
    }
}

/// MCP error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("MCP Error: {} at {}:{}", kind, file, line)]
pub struct McpError {
    /// Error kind
    pub kind: McpErrorKind,
    /// Line number
    pub line: u32,
    /// File path
    pub file: String,
}

impl McpError {
    /// Create a new MCP error.
    #[track_caller]
    pub fn new(kind: McpErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }

    /// Create a tool not found error.
    #[track_caller]
    pub fn tool_not_found(tool: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ToolNotFound(tool.into()))
    }

    /// Create an invalid arguments error.
    #[track_caller]
    pub fn invalid_arguments(tool: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidArguments {
            tool: tool.into(),
            reason: reason.into(),
        })
    }

    /// Create an invalid input error.
    #[track_caller]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidInput(message.into()))
    }

    /// Create an execution failed error.
    #[track_caller]
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ExecutionFailed(message.into()))
    }

    /// Create an invalid state error.
    #[track_caller]
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidState(message.into()))
    }

    /// Create a session error.
    #[track_caller]
    pub fn session(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Session(message.into()))
    }

    /// Create a session not found error.
    #[track_caller]
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Session(format!(
            "Session not found: {}",
            session_id.into()
        )))
    }

    /// Create a resource not found error.
    #[track_caller]
    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ResourceNotFound(resource.into()))
    }

    /// Create a database error with source.
    #[cfg(feature = "database")]
    #[track_caller]
    pub fn database_error(message: impl Into<String>, source: crate::DatabaseError) -> Self {
        Self::new(McpErrorKind::Database(DatabaseMcpError::new(
            message, source,
        )))
    }

    /// Create a backend unavailable error.
    #[track_caller]
    pub fn backend_unavailable(backend: impl Into<String>) -> Self {
        Self::new(McpErrorKind::BackendUnavailable(backend.into()))
    }

    /// Create an unsupported model error.
    #[track_caller]
    pub fn unsupported_model(model: impl Into<String>) -> Self {
        Self::new(McpErrorKind::UnsupportedModel(model.into()))
    }

    /// Create a mutex poisoned error.
    #[track_caller]
    pub fn mutex_poisoned(context: impl Into<String>) -> Self {
        Self::new(McpErrorKind::MutexPoisoned(context.into()))
    }

    /// Create a parse int error with source.
    #[track_caller]
    pub fn parse_int_error(message: impl Into<String>, source: std::num::ParseIntError) -> Self {
        Self::new(McpErrorKind::ParseInt {
            message: message.into(),
            source: Arc::new(source),
        })
    }

    /// Create an environment variable error with source.
    #[track_caller]
    pub fn env_var_error(message: impl Into<String>, source: std::env::VarError) -> Self {
        Self::new(McpErrorKind::EnvVar {
            message: message.into(),
            source: Arc::new(source),
        })
    }

    /// Create an IO error with source.
    #[track_caller]
    pub fn io_error(message: impl Into<String>, source: std::io::Error) -> Self {
        Self::new(McpErrorKind::Io {
            message: message.into(),
            source: Arc::new(source),
        })
    }
}

/// From implementations for external errors to McpError
#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for McpError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(McpErrorKind::from(err))
    }
}

#[cfg(feature = "database")]
impl From<crate::DatabaseError> for McpError {
    #[track_caller]
    fn from(err: crate::DatabaseError) -> Self {
        Self::new(McpErrorKind::from(err))
    }
}

impl From<crate::NarrativeError> for McpError {
    #[track_caller]
    fn from(err: crate::NarrativeError) -> Self {
        Self::new(McpErrorKind::from(err))
    }
}

#[cfg(feature = "mcp")]
impl From<rmcp::ErrorData> for McpError {
    #[track_caller]
    fn from(error: rmcp::ErrorData) -> Self {
        // Map rmcp error code to appropriate McpErrorKind
        Self::new(McpErrorKind::Rmcp {
            code: error.code.0,
            message: error.message.to_string(),
        })
    }
}

/// Result type for MCP operations.
pub type McpResult<T> = std::result::Result<T, McpError>;
