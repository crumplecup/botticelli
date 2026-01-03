//! Execution tool types for LLM generation and narrative execution.
//!
//! These tools require LLM backend features to be enabled.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for simple text generation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateParams {
    /// The prompt to send to the LLM.
    pub prompt: String,

    /// Model to use (e.g., 'gemini-2.0-flash-exp', 'gpt-4o', 'claude-3-5-sonnet-20241022').
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Sampling temperature (0.0-2.0).
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Optional system prompt to set context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

fn default_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_temperature() -> f32 {
    1.0
}

/// Result from text generation.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GenerateResult {
    /// The generated text response.
    pub text: String,

    /// Model that was used for generation.
    pub model: String,

    /// Number of tokens used in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,
}

impl GenerateResult {
    /// Create a new generation result.
    pub fn new(text: String, model: String, tokens_used: Option<u32>) -> Self {
        Self {
            text,
            model,
            tokens_used,
        }
    }
}

/// Parameters for executing a single narrative act.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteActParams {
    /// The act prompt or instruction.
    pub prompt: String,

    /// Model to use for execution.
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Sampling temperature (0.0-2.0).
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Optional system context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Optional context from previous acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Result from executing a single act.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExecuteActResult {
    /// The generated response from the act.
    pub response: String,

    /// Model that was used.
    pub model: String,

    /// Number of tokens used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,

    /// Whether execution was successful.
    pub success: bool,
}

impl ExecuteActResult {
    /// Create a new act execution result.
    pub fn new(response: String, model: String, tokens_used: Option<u32>, success: bool) -> Self {
        Self {
            response,
            model,
            tokens_used,
            success,
        }
    }
}

/// Parameters for executing a complete narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteNarrativeParams {
    /// Path to the narrative TOML file.
    pub narrative_path: String,

    /// Initial prompt to start the narrative.
    pub prompt: String,

    /// Model to use (overrides narrative settings if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Maximum tokens per act.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

/// Result from executing a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExecuteNarrativeResult {
    /// The final output from the narrative.
    pub final_output: String,

    /// Number of acts executed.
    pub acts_executed: usize,

    /// Model(s) used during execution.
    pub models_used: Vec<String>,

    /// Total tokens used across all acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,

    /// Whether the narrative completed successfully.
    pub success: bool,

    /// Optional error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ExecuteNarrativeResult {
    /// Create a new narrative execution result.
    pub fn new(
        final_output: String,
        acts_executed: usize,
        models_used: Vec<String>,
        total_tokens: Option<u32>,
        success: bool,
        error: Option<String>,
    ) -> Self {
        Self {
            final_output,
            acts_executed,
            models_used,
            total_tokens,
            success,
            error,
        }
    }
}
