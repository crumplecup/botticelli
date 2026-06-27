use botticelli_core::PostingJitter;
use botticelli_error::{BotticelliResult, ConfigError};
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for the bot server.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, Builder)]
pub struct BotConfig {
    /// Generation bot configuration
    generation: GenerationConfig,
    /// Curation bot configuration
    curation: CurationConfig,
    /// Posting bot configuration
    posting: PostingConfig,
}

impl BotConfig {
    /// Load bot configuration from a TOML file.
    #[tracing::instrument(skip(path))]
    pub fn from_file(path: impl AsRef<Path>) -> BotticelliResult<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            botticelli_error::BotticelliError::from(ConfigError::new(format!(
                "Failed to read config file: {}",
                e
            )))
        })?;

        toml::from_str(&content).map_err(|e| {
            botticelli_error::BotticelliError::from(ConfigError::new(format!(
                "Failed to parse config: {}",
                e
            )))
        })
    }
}

/// Configuration for the generation bot.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, Builder)]
pub struct GenerationConfig {
    /// Path to generation narrative TOML
    narrative_path: PathBuf,
    /// Name of narrative within file
    narrative_name: String,
    /// How often to run generation (hours)
    interval_hours: u64,
}

/// Configuration for the curation bot.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, Builder)]
pub struct CurationConfig {
    /// Path to curation narrative TOML
    narrative_path: PathBuf,
    /// Name of narrative within file
    narrative_name: String,
    /// How often to check for new content (hours)
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    check_interval_hours: Option<u64>,
    /// How often to check for new content (minutes, for testing)
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    check_interval_minutes: Option<u64>,
    /// Batch size for processing
    batch_size: usize,
}

/// Configuration for the posting bot.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, Builder)]
pub struct PostingConfig {
    /// Path to posting narrative TOML
    narrative_path: PathBuf,
    /// Name of narrative within file
    narrative_name: String,
    /// Base interval between posts (hours)
    base_interval_hours: u64,
    /// Jitter configuration for posting timing
    jitter: PostingJitter,
}

/// Bot scheduling configuration.
#[derive(Debug, Clone, Getters, Builder)]
pub struct BotSchedule {
    /// Generation interval
    generation_interval: std::time::Duration,
    /// Curation check interval
    curation_interval: std::time::Duration,
    /// Posting base interval
    posting_base_interval: std::time::Duration,
}

impl From<&BotConfig> for BotSchedule {
    fn from(config: &BotConfig) -> Self {
        let curation_secs = if let Some(mins) = config.curation().check_interval_minutes() {
            *mins * 60
        } else if let Some(hours) = config.curation().check_interval_hours() {
            *hours * 3600
        } else {
            12 * 3600
        };

        BotScheduleBuilder::default()
            .generation_interval(std::time::Duration::from_secs(
                *config.generation().interval_hours() * 3600,
            ))
            .curation_interval(std::time::Duration::from_secs(curation_secs))
            .posting_base_interval(std::time::Duration::from_secs(
                *config.posting().base_interval_hours() * 3600,
            ))
            .build()
            .expect("BotSchedule fields all set")
    }
}
