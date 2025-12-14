//! Narrative sampling coordinator for managing elicitation state.

use super::{ElicitationDialog, PartialNarrative};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Coordinates narrative elicitation across multiple concurrent sessions.
pub struct NarrativeSamplingCoordinator {
    sessions: HashMap<Uuid, PartialNarrative>,
    dialog: Arc<dyn ElicitationDialog>,
}

impl NarrativeSamplingCoordinator {
    /// Creates new coordinator with given dialog implementation.
    pub fn new(dialog: Arc<dyn ElicitationDialog>) -> Self {
        Self {
            sessions: HashMap::new(),
            dialog,
        }
    }

    /// Creates a new narrative session.
    pub fn create_session(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        self.sessions.insert(id, PartialNarrative::default());
        id
    }

    /// Gets a narrative session.
    pub fn get_session(&self, id: &Uuid) -> Option<&PartialNarrative> {
        self.sessions.get(id)
    }

    /// Gets a mutable narrative session.
    pub fn get_session_mut(&mut self, id: &Uuid) -> Option<&mut PartialNarrative> {
        self.sessions.get_mut(id)
    }

    /// Removes a narrative session.
    pub fn remove_session(&mut self, id: &Uuid) -> Option<PartialNarrative> {
        self.sessions.remove(id)
    }

    /// Gets reference to the dialog implementation.
    pub fn dialog(&self) -> &Arc<dyn ElicitationDialog> {
        &self.dialog
    }
}
