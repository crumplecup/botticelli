//! Trait for narrative component elicitors.

use crate::elicitation::{ElicitationDialog, PartialNarrative};
use crate::ChatResult;
use async_trait::async_trait;

/// Trait for eliciting specific narrative components.
///
/// Each elicitor is responsible for gathering information about one
/// aspect of a narrative (metadata, acts, inputs, etc.).
#[async_trait]
pub trait NarrativeElicitor: Send + Sync {
    /// Human-readable name of this elicitor.
    fn name(&self) -> &str;

    /// Description of what this elicitor does.
    fn description(&self) -> &str;

    /// Check if this elicitor can run given current state.
    ///
    /// Returns true if prerequisites are met.
    fn can_run(&self, partial: &PartialNarrative) -> bool;

    /// Perform elicitation, updating the partial narrative.
    ///
    /// # Arguments
    ///
    /// * `dialog` - UI abstraction for user interaction
    /// * `partial` - Narrative being constructed (mutable)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - User cancels the operation
    /// - Invalid input that cannot be recovered
    /// - IO errors during interaction
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> ChatResult<()>;

    /// Check if this aspect is complete.
    ///
    /// Returns true if no more information is needed.
    fn is_complete(&self, partial: &PartialNarrative) -> bool;

    /// Suggest what to do next (optional guidance).
    ///
    /// Returns None if no specific suggestion.
    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        let _ = partial;
        None
    }
}
