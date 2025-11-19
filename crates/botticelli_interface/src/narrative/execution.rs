//! Narrative execution types.
//!
//! This module defines the data structures for narrative executions that are
//! shared between the executor (in botticelli-narrative) and persistence layer
//! (in botticelli-database).

use botticelli_core::Input;
use serde::{Deserialize, Serialize};

/// Execution result for a single act in a narrative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActExecution {
    /// Name of the act (from the narrative).
    pub act_name: String,

    /// The multimodal inputs that were sent to the LLM.
    pub inputs: Vec<Input>,

    /// The model used for this act (if overridden).
    pub model: Option<String>,

    /// The temperature used for this act (if overridden).
    pub temperature: Option<f32>,

    /// The max_tokens used for this act (if overridden).
    pub max_tokens: Option<u32>,

    /// The text response from the LLM.
    pub response: String,

    /// Position in the execution sequence (0-indexed).
    pub sequence_number: usize,
}

/// Complete execution result for a narrative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeExecution {
    /// Name of the narrative that was executed.
    pub narrative_name: String,

    /// Ordered list of act executions.
    pub act_executions: Vec<ActExecution>,
}
