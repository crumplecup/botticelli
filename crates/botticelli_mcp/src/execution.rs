//! Execution tool types for LLM generation and narrative execution.
//!
//! These tools require LLM backend features to be enabled.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Parameters for simple text generation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct GenerateParams {
    /// The prompt to send to the LLM.
    prompt: String,

    /// Model to use (e.g., 'gemini-2.0-flash-exp', 'gpt-4o', 'claude-3-5-sonnet-20241022').
    #[serde(default = "default_model")]
    model: String,

    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,

    /// Sampling temperature (0.0-2.0).
    #[serde(default = "default_temperature")]
    temperature: f32,

    /// Optional system prompt to set context.
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
}

impl GenerateParams {
    /// Create new generate parameters.
    #[instrument]
    pub fn new(
        prompt: String,
        model: String,
        max_tokens: u32,
        temperature: f32,
        system_prompt: Option<String>,
    ) -> Self {
        Self {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
        }
    }
}

#[tracing::instrument]
fn default_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

#[tracing::instrument]
fn default_max_tokens() -> u32 {
    1024
}

#[tracing::instrument]
fn default_temperature() -> f32 {
    1.0
}

/// Result from text generation.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct GenerateResult {
    /// The generated text response.
    text: String,

    /// Model that was used for generation.
    model: String,

    /// Number of tokens used in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens_used: Option<u32>,
}

impl GenerateResult {
    /// Create a new generation result.
    #[instrument]
    pub fn new(text: String, model: String, tokens_used: Option<u32>) -> Self {
        Self {
            text,
            model,
            tokens_used,
        }
    }
}

/// Parameters for executing a single narrative act.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ExecuteActParams {
    /// The act prompt or instruction.
    prompt: String,

    /// Model to use for execution.
    #[serde(default = "default_model")]
    model: String,

    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,

    /// Sampling temperature (0.0-2.0).
    #[serde(default = "default_temperature")]
    temperature: f32,

    /// Optional system context.
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,

    /// Optional context from previous acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

impl ExecuteActParams {
    /// Create new execute act parameters.
    #[instrument]
    pub fn new(
        prompt: String,
        model: String,
        max_tokens: u32,
        temperature: f32,
        system_prompt: Option<String>,
        context: Option<String>,
    ) -> Self {
        Self {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
            context,
        }
    }
}

/// Result from executing a single act.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ExecuteActResult {
    /// The generated response from the act.
    response: String,

    /// Model that was used.
    model: String,

    /// Number of tokens used.
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens_used: Option<u32>,

    /// Whether execution was successful.
    success: bool,
}

impl ExecuteActResult {
    /// Create a new act execution result.
    #[instrument]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ExecuteNarrativeParams {
    /// Path to the narrative TOML file.
    narrative_path: String,

    /// Initial prompt to start the narrative.
    prompt: String,

    /// Model to use (overrides narrative settings if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,

    /// Maximum tokens per act.
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

impl ExecuteNarrativeParams {
    /// Create new execute narrative parameters.
    #[instrument]
    pub fn new(
        narrative_path: String,
        prompt: String,
        model: Option<String>,
        max_tokens: u32,
    ) -> Self {
        Self {
            narrative_path,
            prompt,
            model,
            max_tokens,
        }
    }
}

/// Result from executing a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ExecuteNarrativeResult {
    /// The final output from the narrative.
    final_output: String,

    /// Number of acts executed.
    acts_executed: usize,

    /// Model(s) used during execution.
    models_used: Vec<String>,

    /// Total tokens used across all acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_tokens: Option<u32>,

    /// Whether the narrative completed successfully.
    success: bool,

    /// Optional error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ExecuteNarrativeResult {
    /// Create a new narrative execution result.
    #[instrument]
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
