//! Anthropic API request and response types.

use botticelli_core::ToolDefinition;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// Anthropic tool definition format.
///
/// Represents a tool in Anthropic's API format, converted from MCP ToolDefinition.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct AnthropicTool {
    /// Tool name
    name: String,
    /// Tool description
    description: String,
    /// JSON Schema for input parameters
    input_schema: serde_json::Value,
}

impl AnthropicTool {
    /// Convert MCP ToolDefinition to Anthropic format.
    ///
    /// # Arguments
    ///
    /// * `tool` - The MCP tool definition to convert
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_core::ToolDefinition;
    /// use botticelli_models::anthropic::AnthropicTool;
    /// use serde_json::json;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mcp_tool = ToolDefinition::builder()
    ///     .name("echo".to_string())
    ///     .description("Echoes input".to_string())
    ///     .input_schema(json!({"type": "object", "properties": {"message": {"type": "string"}}}))
    ///     .build()?;
    ///
    /// let anthropic_tool = AnthropicTool::from_mcp(&mcp_tool);
    /// assert_eq!(anthropic_tool.name(), "echo");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_mcp(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema().clone(),
        }
    }
}

/// Anthropic API request.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[builder(setter(into), pattern = "owned")]
pub struct AnthropicRequest {
    /// Model identifier
    model: String,
    /// List of messages
    messages: Vec<AnthropicMessage>,
    /// Maximum tokens to generate
    max_tokens: u32,
    /// Optional system prompt
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    /// Optional temperature
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Available tools for the LLM to call
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

impl AnthropicRequest {
    /// Creates a builder for `AnthropicRequest`.
    pub fn builder() -> AnthropicRequestBuilder {
        AnthropicRequestBuilder::default()
    }
}

/// Anthropic message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[builder(setter(into), pattern = "owned")]
pub struct AnthropicMessage {
    /// Role of the message sender
    role: String,
    /// Content blocks
    content: Vec<AnthropicContentBlock>,
}

impl AnthropicMessage {
    /// Creates a builder for `AnthropicMessage`.
    pub fn builder() -> AnthropicMessageBuilder {
        AnthropicMessageBuilder::default()
    }
}

/// Content block in an Anthropic message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    /// Text content
    Text {
        /// Text content
        text: String,
    },
    /// Image content
    Image {
        /// Image source
        source: AnthropicImageSource,
    },
}

/// Image source for Anthropic API.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[builder(setter(into), pattern = "owned")]
pub struct AnthropicImageSource {
    /// Source type (always "base64")
    #[builder(default = "\"base64\".to_string()")]
    r#type: String,
    /// Media type
    media_type: String,
    /// Base64-encoded image data
    data: String,
}

impl AnthropicImageSource {
    /// Creates a builder for `AnthropicImageSource`.
    pub fn builder() -> AnthropicImageSourceBuilder {
        AnthropicImageSourceBuilder::default()
    }
}

/// Anthropic API response.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[builder(setter(into), pattern = "owned")]
pub struct AnthropicResponse {
    /// Response ID
    id: String,
    /// Response type
    #[serde(rename = "type")]
    response_type: String,
    /// Role (should be "assistant")
    role: String,
    /// Content blocks
    content: Vec<AnthropicContent>,
    /// Model used
    model: String,
    /// Stop reason
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_reason: Option<String>,
    /// Usage information
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<AnthropicUsage>,
}

impl AnthropicResponse {
    /// Creates a builder for `AnthropicResponse`.
    pub fn builder() -> AnthropicResponseBuilder {
        AnthropicResponseBuilder::default()
    }
}

/// Content in an Anthropic response.
///
/// Can be either text or a tool use request from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContent {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Tool use request from the model
    ToolUse {
        /// Unique identifier for this tool use
        id: String,
        /// Name of the tool to call
        name: String,
        /// Arguments to pass to the tool (JSON object)
        input: serde_json::Value,
    },
}

/// Usage information from Anthropic API.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[builder(setter(into), pattern = "owned")]
pub struct AnthropicUsage {
    /// Input tokens
    input_tokens: u32,
    /// Output tokens
    output_tokens: u32,
}

impl AnthropicUsage {
    /// Creates a builder for `AnthropicUsage`.
    pub fn builder() -> AnthropicUsageBuilder {
        AnthropicUsageBuilder::default()
    }
}
