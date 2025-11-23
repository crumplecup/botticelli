//! Duplicate checking skill to prevent reposting same content.

use crate::{Skill, SkillContext, SkillOutput, SkillResult};
use async_trait::async_trait;
use serde_json::json;

/// Skill for checking if content has been posted recently.
pub struct DuplicateCheckSkill {
    name: String,
}

impl DuplicateCheckSkill {
    /// Create a new duplicate check skill.
    pub fn new() -> Self {
        Self {
            name: "duplicate_check".to_string(),
        }
    }
}

impl Default for DuplicateCheckSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for DuplicateCheckSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Check if content has been posted recently to prevent duplicates"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing duplicate check skill");

        let lookback_days = context
            .config
            .get("lookback_days")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(30);

        let similarity_threshold = context
            .config
            .get("similarity_threshold")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.9);

        tracing::info!(
            lookback_days,
            similarity_threshold,
            "Duplicate check configuration loaded"
        );

        let post_history = context
            .knowledge
            .get("post_history")
            .cloned()
            .unwrap_or_default();

        tracing::debug!(
            history_count = post_history.len(),
            "Processing post history"
        );

        let content_ids: Vec<i32> = post_history
            .iter()
            .filter_map(|row| row.get("content_id").and_then(|v| v.as_i64()).map(|i| i as i32))
            .collect();

        tracing::info!(
            posted_content_ids = content_ids.len(),
            "Duplicate check completed"
        );

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            data: json!({
                "posted_content_ids": content_ids,
                "lookback_days": lookback_days,
                "similarity_threshold": similarity_threshold,
                "history_count": post_history.len(),
            }),
        })
    }
}
