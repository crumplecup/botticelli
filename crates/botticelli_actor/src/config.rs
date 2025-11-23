//! Actor configuration types and loading.

use crate::{ActorError, ActorErrorKind, ActorResult};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use typed_builder::TypedBuilder;

/// Cache strategy for actor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheStrategy {
    /// No caching - always re-query knowledge.
    None,
    /// In-memory cache with TTL (faster, volatile).
    Memory,
    /// Disk-based cache with TTL (persistent, slower).
    Disk,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self::Memory
    }
}

/// Actor state cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, TypedBuilder)]
pub struct ActorCacheConfig {
    /// Cache strategy.
    #[builder(default = CacheStrategy::Memory)]
    #[serde(default = "default_cache_strategy")]
    strategy: CacheStrategy,

    /// TTL in seconds for cached entries.
    #[builder(default = 300)]
    #[serde(default = "default_cache_ttl")]
    ttl_seconds: u64,

    /// Maximum number of cache entries.
    #[builder(default = 1000)]
    #[serde(default = "default_cache_max_entries")]
    max_entries: usize,

    /// Path for disk cache (only used with Disk strategy).
    #[builder(default)]
    #[serde(default)]
    disk_path: Option<PathBuf>,
}

fn default_cache_strategy() -> CacheStrategy {
    CacheStrategy::Memory
}

fn default_cache_ttl() -> u64 {
    300
}

fn default_cache_max_entries() -> usize {
    1000
}

impl Default for ActorCacheConfig {
    fn default() -> Self {
        Self {
            strategy: default_cache_strategy(),
            ttl_seconds: default_cache_ttl(),
            max_entries: default_cache_max_entries(),
            disk_path: None,
        }
    }
}

/// Execution configuration for error handling.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, TypedBuilder)]
pub struct ExecutionConfig {
    /// Stop execution on unrecoverable errors.
    #[builder(default = true)]
    #[serde(default = "default_stop_on_unrecoverable")]
    stop_on_unrecoverable: bool,

    /// Maximum retry attempts for recoverable errors.
    #[builder(default = 3)]
    #[serde(default = "default_max_retries")]
    max_retries: u32,

    /// Continue execution on recoverable errors (collect vs fail fast).
    #[builder(default = true)]
    #[serde(default = "default_continue_on_error")]
    continue_on_error: bool,
}

fn default_stop_on_unrecoverable() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_continue_on_error() -> bool {
    true
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            stop_on_unrecoverable: default_stop_on_unrecoverable(),
            max_retries: default_max_retries(),
            continue_on_error: default_continue_on_error(),
        }
    }
}

/// Actor settings with sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, TypedBuilder)]
pub struct ActorSettings {
    /// Maximum posts per day.
    #[builder(default = 10)]
    #[serde(default = "default_max_posts")]
    max_posts_per_day: u32,

    /// Minimum interval between posts in minutes.
    #[builder(default = 60)]
    #[serde(default = "default_min_interval")]
    min_interval_minutes: u32,

    /// Number of retry attempts for failed operations.
    #[builder(default = 3)]
    #[serde(default = "default_retry_attempts")]
    retry_attempts: u32,

    /// Timezone for scheduling (IANA timezone name).
    #[builder(default = "UTC".to_string())]
    #[serde(default = "default_timezone")]
    timezone: String,
}

fn default_max_posts() -> u32 {
    10
}

fn default_min_interval() -> u32 {
    60
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl Default for ActorSettings {
    fn default() -> Self {
        Self {
            max_posts_per_day: default_max_posts(),
            min_interval_minutes: default_min_interval(),
            retry_attempts: default_retry_attempts(),
            timezone: default_timezone(),
        }
    }
}

/// Per-skill configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, TypedBuilder)]
pub struct SkillConfig {
    /// Whether this skill is enabled.
    #[builder(default = true)]
    #[serde(default = "default_skill_enabled")]
    enabled: bool,

    /// Skill-specific settings.
    #[builder(default)]
    #[serde(default)]
    #[serde(flatten)]
    settings: HashMap<String, serde_json::Value>,
}

fn default_skill_enabled() -> bool {
    true
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            enabled: default_skill_enabled(),
            settings: HashMap::new(),
        }
    }
}

/// Main actor configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, TypedBuilder)]
pub struct ActorConfig {
    /// Actor name.
    name: String,

    /// Actor description.
    description: String,

    /// Knowledge table names this actor consumes.
    knowledge: Vec<String>,

    /// Skill names this actor uses.
    skills: Vec<String>,

    /// General actor settings.
    #[builder(default)]
    #[serde(default)]
    config: ActorSettings,

    /// Cache configuration.
    #[builder(default)]
    #[serde(default)]
    cache: ActorCacheConfig,

    /// Execution configuration.
    #[builder(default)]
    #[serde(default)]
    execution: ExecutionConfig,

    /// Per-skill configuration.
    #[builder(default)]
    #[serde(default)]
    skill_configs: HashMap<String, SkillConfig>,
}

impl ActorConfig {
    /// Load actor configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to TOML file
    ///
    /// # Returns
    ///
    /// Parsed configuration on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - TOML parsing fails
    /// - Required fields missing
    #[tracing::instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn from_file<P: AsRef<Path>>(path: P) -> ActorResult<Self> {
        let path = path.as_ref();
        tracing::debug!("Loading actor config from file");

        let contents = fs::read_to_string(path).map_err(|e| {
            ActorError::new(ActorErrorKind::FileIo {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        })?;

        let config: ConfigFile = toml::from_str(&contents)?;

        // Validate required fields
        if config.actor.name.is_empty() {
            return Err(ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Actor name cannot be empty".to_string(),
            )));
        }

        if config.actor.knowledge.is_empty() {
            tracing::warn!("Actor has no knowledge tables configured");
        }

        if config.actor.skills.is_empty() {
            tracing::warn!("Actor has no skills configured");
        }

        tracing::info!(
            name = %config.actor.name,
            knowledge_tables = config.actor.knowledge.len(),
            skills = config.actor.skills.len(),
            "Loaded actor configuration"
        );

        Ok(ActorConfig {
            name: config.actor.name,
            description: config.actor.description,
            knowledge: config.actor.knowledge,
            skills: config.actor.skills,
            config: config.actor.config.unwrap_or_default(),
            cache: config.actor.cache.unwrap_or_default(),
            execution: config.actor.execution.unwrap_or_default(),
            skill_configs: config.skills.unwrap_or_default(),
        })
    }

    /// Validate configuration.
    ///
    /// Checks for common configuration issues.
    ///
    /// # Returns
    ///
    /// List of validation warnings (empty if valid).
    #[tracing::instrument(skip(self))]
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.knowledge.is_empty() {
            warnings.push("No knowledge tables configured".to_string());
        }

        if self.skills.is_empty() {
            warnings.push("No skills configured".to_string());
        }

        if *self.config.max_posts_per_day() == 0 {
            warnings.push("max_posts_per_day is 0, actor will not post".to_string());
        }

        if *self.config.min_interval_minutes() == 0 {
            warnings.push("min_interval_minutes is 0, rate limiting disabled".to_string());
        }

        if let Some(disk_path) = self.cache.disk_path() {
            if self.cache.strategy() != &CacheStrategy::Disk {
                warnings.push(format!(
                    "disk_path set ({}) but cache strategy is {:?}",
                    disk_path.display(),
                    self.cache.strategy()
                ));
            }
        }

        for skill in &self.skills {
            if let Some(skill_config) = self.skill_configs.get(skill) {
                if !skill_config.enabled() {
                    warnings.push(format!("Skill '{}' is configured but disabled", skill));
                }
            }
        }

        tracing::debug!(warnings = warnings.len(), "Configuration validated");
        warnings
    }
}

/// Internal TOML file structure.
#[derive(Debug, Deserialize)]
struct ConfigFile {
    actor: ActorSection,
    #[serde(default)]
    skills: Option<HashMap<String, SkillConfig>>,
}

/// Actor section in TOML file.
#[derive(Debug, Deserialize)]
struct ActorSection {
    name: String,
    description: String,
    knowledge: Vec<String>,
    skills: Vec<String>,
    #[serde(default)]
    config: Option<ActorSettings>,
    #[serde(default)]
    cache: Option<ActorCacheConfig>,
    #[serde(default)]
    execution: Option<ExecutionConfig>,
}
