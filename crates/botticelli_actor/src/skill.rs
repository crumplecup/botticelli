//! Skill system for reusable actor capabilities.

use crate::{ActorError, ActorErrorKind, SocialMediaPlatform};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Result type for skill operations.
pub type SkillResult<T> = Result<T, ActorError>;

/// Output from skill execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillOutput {
    /// Skill name that produced output.
    pub skill_name: String,
    /// Result data (JSON for flexibility).
    pub data: JsonValue,
}

/// Context provided to skills during execution.
pub struct SkillContext {
    /// Knowledge table data (table_name -> rows).
    pub knowledge: HashMap<String, Vec<JsonValue>>,
    /// Skill-specific configuration.
    pub config: HashMap<String, String>,
    /// Platform interface.
    pub platform: Arc<dyn SocialMediaPlatform>,
}

/// Information about a skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInfo {
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
}

/// Trait for skill implementations.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Get skill name.
    fn name(&self) -> &str;

    /// Get skill description.
    fn description(&self) -> &str;

    /// Execute skill with provided context.
    ///
    /// # Arguments
    ///
    /// * `context` - Execution context with knowledge and configuration
    ///
    /// # Returns
    ///
    /// Skill output on success.
    ///
    /// # Errors
    ///
    /// Returns error if skill execution fails.
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput>;
}

/// Registry for managing skills.
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl SkillRegistry {
    /// Create a new empty skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill.
    ///
    /// # Arguments
    ///
    /// * `skill` - Skill to register
    #[tracing::instrument(skip(self, skill), fields(skill_name = skill.name()))]
    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let name = skill.name().to_string();
        tracing::debug!("Registering skill");
        self.skills.insert(name, skill);
    }

    /// Get a skill by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    ///
    /// # Returns
    ///
    /// Skill if found, None otherwise.
    #[tracing::instrument(skip(self), fields(skill_name = name))]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(name).cloned()
    }

    /// Execute a skill by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    /// * `context` - Execution context
    ///
    /// # Returns
    ///
    /// Skill output on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Skill not found
    /// - Skill execution fails
    #[tracing::instrument(skip(self, context), fields(skill_name = name))]
    pub async fn execute(&self, name: &str, context: &SkillContext) -> SkillResult<SkillOutput> {
        let skill = self
            .get(name)
            .ok_or_else(|| ActorError::new(ActorErrorKind::SkillNotFound(name.to_string())))?;

        tracing::debug!("Executing skill");
        skill.execute(context).await
    }

    /// List all registered skills.
    ///
    /// Returns information about all skills in the registry.
    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Vec<SkillInfo> {
        self.skills
            .values()
            .map(|skill| SkillInfo {
                name: skill.name().to_string(),
                description: skill.description().to_string(),
            })
            .collect()
    }

    /// Get number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
