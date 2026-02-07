//! Role types for conversation participants.

use elicitation::{Prompt, Select};
use serde::{Deserialize, Serialize};

/// Roles are the same across modalities (text, image, etc.)
///
/// # Examples
///
/// ```
/// use botticelli_core::Role;
///
/// let user_role = Role::User;
/// let assistant_role = Role::Assistant;
/// assert_ne!(user_role, assistant_role);
///
/// // Display implementation
/// assert_eq!(format!("{}", Role::System), "System");
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    derive_more::Display,
    elicitation::Elicit,
)]
pub enum Role {
    /// System messages provide context and instructions
    System,
    /// User messages are from the human
    User,
    /// Assistant messages are from the AI
    Assistant,
}
