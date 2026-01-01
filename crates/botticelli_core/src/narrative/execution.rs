//! Narrative execution types.
//!
//! This module defines the data structures for narrative executions that are
//! shared between the executor (in botticelli-narrative) and persistence layer
//! (in botticelli-database).

use crate::{Input, TokenUsageData};
use serde::{Deserialize, Serialize};

/// Execution result for a single act in a narrative.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_builder::Builder,
)]
#[setters(prefix = "with_")]
#[builder(setter(into))]
pub struct ActExecution {
    /// Name of the act (from the narrative).
    act_name: String,

    /// The multimodal inputs that were sent to the LLM.
    inputs: Vec<Input>,

    /// The model used for this act (if overridden).
    model: Option<String>,

    /// The temperature used for this act (if overridden).
    temperature: Option<f32>,

    /// The max_tokens used for this act (if overridden).
    max_tokens: Option<u32>,

    /// The text response from the LLM.
    response: String,

    /// Position in the execution sequence (0-indexed).
    sequence_number: usize,

    /// Token usage for this act (input, output, total).
    #[serde(skip_serializing_if = "Option::is_none")]
    token_usage: Option<TokenUsageData>,

    /// Estimated cost in USD for this act.
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_cost_usd: Option<f64>,

    /// Duration in milliseconds for this act.
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
}

/// Complete execution result for a narrative.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_new::new,
)]
#[setters(prefix = "with_")]
pub struct NarrativeExecution {
    /// Name of the narrative that was executed.
    narrative_name: String,

    /// Ordered list of act executions.
    act_executions: Vec<ActExecution>,

    /// Total token usage across all acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_token_usage: Option<TokenUsageData>,

    /// Total estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_cost_usd: Option<f64>,

    /// Total duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_duration_ms: Option<u64>,
}
