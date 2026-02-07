//! JSON error types.

/// Serde JSON error with source tracking.
#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::{Prompt, Select};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, derive_more::Display, derive_more::Error, derive_getters::Getters, elicitation::Elicit,
)]
#[display("Serde JSON error: {:?} at {}:{}", source, file, line)]
pub struct SerdeJsonError {
    /// The serde_json error source
    source: Box<serde_json::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

impl SerdeJsonError {
    /// Create a new SerdeJsonError with automatic location tracking.
    #[cfg_attr(feature = "mcp", tool)]
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

impl Clone for SerdeJsonError {
    fn clone(&self) -> Self {
        // serde_json::Error is not Clone, so we reconstruct from the message
        let msg = format!("{:?}", self.source);
        Self {
            source: Box::new(serde_json::Error::io(std::io::Error::other(msg))),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

impl PartialEq for SerdeJsonError {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
            && self.file == other.file
            && format!("{:?}", self.source) == format!("{:?}", other.source)
    }
}

impl Eq for SerdeJsonError {}

impl std::hash::Hash for SerdeJsonError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.file.hash(state);
        format!("{:?}", self.source).hash(state);
    }
}

impl PartialOrd for SerdeJsonError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SerdeJsonError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.file, self.line, format!("{:?}", self.source)).cmp(&(
            &other.file,
            other.line,
            format!("{:?}", other.source),
        ))
    }
}

impl serde::Serialize for SerdeJsonError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SerdeJsonError", 3)?;
        state.serialize_field("source", &format!("{:?}", self.source))?;
        state.serialize_field("line", &self.line)?;
        state.serialize_field("file", &self.file)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SerdeJsonError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SerdeJsonErrorHelper {
            source: String,
            line: u32,
            file: String,
        }

        let helper = SerdeJsonErrorHelper::deserialize(deserializer)?;
        Ok(Self {
            source: Box::new(serde_json::Error::io(std::io::Error::other(helper.source))),
            line: helper.line,
            file: helper.file,
        })
    }
}

impl schemars::JsonSchema for SerdeJsonError {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "SerdeJsonError".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "object",
            "required": ["source", "line", "file"],
            "properties": {
                "source": { "type": "string" },
                "line": generator.subschema_for::<u32>(),
                "file": generator.subschema_for::<String>()
            }
        })
    }
}

/// JSON error kind.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, derive_more::Display, schemars::JsonSchema, elicitation::Elicit,
)]
pub enum JsonErrorKind {
    /// Generic JSON error with message
    #[display("JSON error: {}", _0)]
    Message(String),

    /// Serde JSON error
    #[display("{}", _0)]
    SerdeJson(SerdeJsonError),
}

/// JSON serialization/deserialization error with source location.
#[derive(
    Debug,
    Clone,
    derive_more::Display,
    derive_more::Error,
    derive_getters::Getters,
    elicitation::Elicit,
)]
#[display("JSON Error: {} at {}:{}", kind, file, line)]
pub struct JsonError {
    /// The error kind
    kind: JsonErrorKind,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: String,
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
        (&self.file, self.line, &self.kind).cmp(&(&other.file, other.line, &other.kind))
    }
}

impl JsonError {
    /// Create a new JsonError with the given kind at the current location.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: JsonErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
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

// Add support for wrapping serde_json errors
impl From<serde_json::Error> for JsonError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(JsonErrorKind::SerdeJson(SerdeJsonError::new(err)))
    }
}

// Bridge serde_json::Error to BotticelliErrorKind
crate::bridge_error!(serde_json::Error => JsonError => crate::BotticelliErrorKind);
