//! Repository types for narrative persistence.

use elicitation::{Prompt, Select};
use serde::{Deserialize, Serialize};

/// Filter criteria for querying executions.
///
/// All fields are optional to allow flexible queries. Combining multiple
/// criteria creates an AND condition.
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    elicitation::Elicit,
)]
#[setters(prefix = "with_", strip_option)]
pub struct ExecutionFilter {
    /// Filter by narrative name (exact match)
    narrative_name: Option<String>,
    /// Filter by execution status
    status: Option<ExecutionStatus>,
    /// Maximum number of results to return
    limit: Option<usize>,
    /// Number of results to skip (for pagination)
    offset: Option<usize>,
}

impl ExecutionFilter {
    /// Create an empty filter (returns all executions).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Summary of an execution (lightweight view without full act details).
///
/// Used by `list_executions` to return metadata about executions without
/// loading all the act data.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_new::new,
    elicitation::Elicit,
)]
pub struct ExecutionSummary {
    /// Unique execution ID
    id: i32,
    /// Name of the narrative that was executed
    narrative_name: String,
    /// Optional description from narrative metadata
    narrative_description: Option<String>,
    /// Execution status
    status: ExecutionStatus,
    /// Number of acts in this execution
    act_count: usize,
    /// Error message if status is Failed
    error_message: Option<String>,
}

/// Execution status enumeration.
///
/// Tracks the lifecycle state of a narrative execution.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Display,
    strum::EnumString,
    elicitation::Elicit,
)]
#[strum(serialize_all = "lowercase")]
pub enum ExecutionStatus {
    /// Execution is currently in progress
    #[display("running")]
    Running,
    /// Execution completed successfully
    #[display("completed")]
    Completed,
    /// Execution failed with an error
    #[display("failed")]
    Failed,
}
