//! Content formatting skill for preparing content for platform-specific posting.

use crate::{
    ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillOutputBuilder, SkillResult,
};
use async_trait::async_trait;
use serde_json::json;

/// Skill for formatting content for posting.
pub struct ContentFormatterSkill {
    name: String,
}

impl ContentFormatterSkill {
    /// Create a new content formatter skill.
    pub fn new() -> Self {
        Self {
            name: "content_formatter".to_string(),
        }
    }
}

impl Default for ContentFormatterSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for ContentFormatterSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Format content for platform-specific posting requirements"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing content formatter skill");

        let max_text_length = context
            .config()
            .get("max_text_length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2000);

        let include_source = context
            .config()
            .get("include_source")
            .map(|s| s.parse::<bool>().unwrap_or(true))
            .unwrap_or(true);

        let add_hashtags = context
            .config()
            .get("add_hashtags")
            .map(|s| s.parse::<bool>().unwrap_or(true))
            .unwrap_or(true);

        tracing::info!(
            max_text_length,
            include_source,
            add_hashtags,
            "Content formatter configuration loaded"
        );

        let content_rows = context
            .knowledge()
            .get("content")
            .cloned()
            .unwrap_or_default();

        let formatted_count = content_rows.len();

        tracing::info!(formatted = formatted_count, "Content formatting completed");

        Ok(SkillOutputBuilder::default()
            .skill_name(self.name.clone())
            .data(json!({
                "max_text_length": max_text_length,
                "include_source": include_source,
                "add_hashtags": add_hashtags,
                "formatted_count": formatted_count,
            }))
            .build()
            .map_err(|e| ActorError::new(ActorErrorKind::Narrative(e)))?)
    }
}
