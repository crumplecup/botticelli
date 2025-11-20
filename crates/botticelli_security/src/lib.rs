//! Security framework for safe agentic bot operations.
//!
//! This crate provides a multi-layer security framework for executing bot commands
//! from AI-generated narratives. It protects against AI hallucinations, prompt injection,
//! privilege escalation, and abuse while maintaining comprehensive audit trails.
//!
//! # Architecture
//!
//! The security framework consists of 5 layers:
//!
//! 1. **Permission Layer** - Granular per-narrative command permissions
//! 2. **Validation Layer** - Input validation and resource checks
//! 3. **Content Layer** - Content filtering and pattern detection
//! 4. **Rate Limit Layer** - Token bucket rate limiting
//! 5. **Approval Layer** - Human-in-the-loop for dangerous operations
//!
//! All operations are logged to an audit trail for accountability.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod permission;
mod validation;
mod content;
mod rate_limit;
mod approval;
mod executor;

pub use error::{SecurityError, SecurityErrorKind, SecurityResult};
pub use permission::{
    CommandPermission, ResourcePermission, PermissionConfig, PermissionChecker,
};
pub use validation::{CommandValidator, ValidationError, DiscordValidator};
pub use content::{ContentFilter, ContentFilterConfig, ContentViolation};
pub use rate_limit::{RateLimiter, RateLimit, RateLimitExceeded};
pub use approval::{ApprovalWorkflow, PendingAction, ApprovalDecision};
pub use executor::SecureExecutor;
