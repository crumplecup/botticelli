//! Error type elicitation tools.
//!
//! Provides elicitation tools for all error types from botticelli_error crate.
//! This enables agents to construct error scenarios for testing and mocking.

use crate::rmcp_server::BotticelliServer;
use botticelli_error::{
    BuilderErrorKind,
    ChatErrorKind,
    EnvErrorKind,
    HttpErrorKind,
    JsonErrorKind,
    ObservabilityErrorKind,
    RateLimitErrorKind,
    SamplingErrorKind,
    StorageErrorKind,
    ValidationError,
    ValidationErrorKind,
    ValidationLocation,
    ValidationResult,
    ValidationWarning,
    ValidationWarningKind,
};
use elicitation::Elicit;
use elicitation_macros::elicit_tools;
use rmcp::tool;
use rmcp::tool_router;

// ============================================================================
// Elicit Tools - Separate impl block for error type elicitation
// ============================================================================

#[elicit_tools(
    BuilderErrorKind,
    ChatErrorKind,
    EnvErrorKind,
    HttpErrorKind,
    JsonErrorKind,
    ObservabilityErrorKind,
    RateLimitErrorKind,
    SamplingErrorKind,
    StorageErrorKind,
    ValidationError,
    ValidationErrorKind,
    ValidationLocation,
    ValidationResult,
    ValidationWarning,
    ValidationWarningKind
)]
#[tool_router(router = errors_elicit_tool_router, vis = "pub(crate)")]
impl BotticelliServer {}
