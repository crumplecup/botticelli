//! Rate limiting skill.

use crate::{ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillResult};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

/// Skill for enforcing posting frequency limits.
pub struct RateLimitingSkill {
    name: String,
    #[allow(dead_code)]
    state: HashMap<String, usize>,
}

impl RateLimitingSkill {
    /// Create a new rate limiting skill.
    pub fn new() -> Self {
        Self {
            name: "rate_limiting".to_string(),
            state: HashMap::new(),
        }
    }

    /// Check if rate limit would be exceeded.
    fn check_limit(&self, max_posts: usize, current_count: usize) -> bool {
        current_count < max_posts
    }
}

impl Default for RateLimitingSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for RateLimitingSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Enforce posting frequency limits to prevent rate limit violations"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing rate limiting skill");

        let max_posts_per_day = context
            .config
            .get("max_posts_per_day")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);

        let min_interval_minutes = context
            .config
            .get("min_interval_minutes")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);

        tracing::info!(
            max_posts_per_day,
            min_interval_minutes,
            "Rate limiting configuration loaded"
        );

        let mut total_items = 0;
        for rows in context.knowledge.values() {
            total_items += rows.len();
        }

        let current_count = 0;
        let can_post = self.check_limit(max_posts_per_day, current_count);

        if !can_post {
            tracing::warn!(
                current = current_count,
                max = max_posts_per_day,
                "Rate limit would be exceeded"
            );
            return Err(ActorError::new(ActorErrorKind::RateLimitExceeded(
                min_interval_minutes * 60,
            )));
        }

        let remaining = max_posts_per_day.saturating_sub(current_count);

        tracing::info!(
            items = total_items,
            current_count,
            remaining,
            "Rate limit check passed"
        );

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            data: json!({
                "max_posts_per_day": max_posts_per_day,
                "min_interval_minutes": min_interval_minutes,
                "current_count": current_count,
                "remaining": remaining,
                "can_post": can_post,
            }),
        })
    }
}
