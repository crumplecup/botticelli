//! JSON error types.

/// Serde JSON error with source tracking.
#[cfg(feature = "serde_json")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Serde JSON error: {:?} at {}:{}", source, file, line)]
pub struct SerdeJsonError {
    /// The serde_json error source
    source: Box<serde_json::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: &'static str,
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
            file: location.file(),
        }
    }
}

#[cfg(feature = "serde_json")]
impl Clone for SerdeJsonError {
    fn clone(&self) -> Self {
        // serde_json::Error is not Clone, so we reconstruct from the message
        let msg = format!("{:?}", self.source);
        Self {
            source: Box::new(serde_json::Error::io(std::io::Error::other(msg))),
            line: self.line,
            file: self.file,
        }
    }
}

#[cfg(feature = "serde_json")]
impl PartialEq for SerdeJsonError {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
            && self.file == other.file
            && format!("{:?}", self.source) == format!("{:?}", other.source)
    }
}

#[cfg(feature = "serde_json")]
impl Eq for SerdeJsonError {}

#[cfg(feature = "serde_json")]
impl std::hash::Hash for SerdeJsonError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.file.hash(state);
        format!("{:?}", self.source).hash(state);
    }
}

#[cfg(feature = "serde_json")]
impl PartialOrd for SerdeJsonError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "serde_json")]
impl Ord for SerdeJsonError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.file, self.line, format!("{:?}", self.source)).cmp(&(
            other.file,
            other.line,
            format!("{:?}", other.source),
        ))
    }
}

/// JSON error kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum JsonErrorKind {
    /// Generic JSON error with message
    #[display("JSON error: {}", _0)]
    Message(String),

    /// Serde JSON error
    #[cfg(feature = "serde_json")]
    #[display("{}", _0)]
    SerdeJson(SerdeJsonError),
}

/// JSON serialization/deserialization error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("JSON Error: {} at {}:{}", kind, file, line)]
pub struct JsonError {
    /// The error kind
    kind: JsonErrorKind,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: &'static str,
}

impl PartialEq for JsonError {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line && self.file == other.file && self.kind == other.kind
    }
}

impl Eq for JsonError {}

impl std::hash::Hash for JsonError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.file.hash(state);
        self.kind.hash(state);
    }
}

impl PartialOrd for JsonError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsonError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.file, self.line, &self.kind).cmp(&(other.file, other.line, &other.kind))
    }
}

impl JsonError {
    /// Create a new JsonError with the given kind at the current location.
    #[track_caller]
    pub fn new(kind: JsonErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

/// Support converting from String for convenience.
impl From<String> for JsonError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(JsonErrorKind::Message(message))
    }
}

/// Support converting from &str for convenience.
impl From<&str> for JsonError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(JsonErrorKind::Message(message.to_string()))
    }
}

crate::impl_error_from_kind!(JsonErrorKind => JsonError);

// Add support for wrapping serde_json errors when feature is enabled
#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for JsonError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(JsonErrorKind::SerdeJson(SerdeJsonError::new(err)))
    }
}

// Bridge serde_json::Error to BotticelliErrorKind
#[cfg(feature = "serde_json")]
crate::bridge_error!(serde_json::Error => JsonError => crate::BotticelliErrorKind);
