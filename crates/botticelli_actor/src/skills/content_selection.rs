//! Content selection skill for querying and ranking content from database.

use crate::{
    ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillOutputBuilder, SkillResult,
};
use async_trait::async_trait;
use serde_json::json;

/// Skill for selecting and ranking content from the database.
pub struct ContentSelectionSkill {
    name: String,
}

impl ContentSelectionSkill {
    /// Create a new content selection skill.
    pub fn new() -> Self {
        Self {
            name: "content_selection".to_string(),
        }
    }
}

impl Default for ContentSelectionSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for ContentSelectionSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Select and rank content from database based on priority and freshness"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing content selection skill");

        let max_candidates = context
            .config()
            .get("max_candidates")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);

        let priority_weight = context
            .config()
            .get("priority_weight")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.7);

        let freshness_weight = context
            .config()
            .get("freshness_weight")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.3);

        tracing::info!(
            max_candidates,
            priority_weight,
            freshness_weight,
            "Content selection configuration loaded"
        );

        let content_rows = context
            .knowledge()
            .get("content")
            .cloned()
            .unwrap_or_default();

        tracing::debug!(count = content_rows.len(), "Processing content items");

        let candidates = content_rows
            .into_iter()
            .take(max_candidates)
            .collect::<Vec<_>>();

        tracing::info!(selected = candidates.len(), "Content selection completed");

        Ok(SkillOutputBuilder::default()
            .skill_name(self.name.clone())
            .data(json!({
                "candidates": candidates,
                "max_candidates": max_candidates,
                "priority_weight": priority_weight,
                "freshness_weight": freshness_weight,
            }))
            .build()
            .map_err(|e| ActorError::new(ActorErrorKind::Narrative(e)))?)
    }
}
