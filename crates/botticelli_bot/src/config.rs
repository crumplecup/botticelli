use botticelli_error::{BotticelliResult, ConfigError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for the bot server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Generation bot configuration
    pub generation: GenerationConfig,
    /// Curation bot configuration
    pub curation: CurationConfig,
    /// Posting bot configuration
    pub posting: PostingConfig,
}

impl BotConfig {
    /// Load bot configuration from a TOML file.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Path to generation narrative TOML
    pub narrative_path: PathBuf,
    /// Name of narrative within file
    pub narrative_name: String,
    /// How often to run generation (hours)
    pub interval_hours: u64,
}

/// Configuration for the curation bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationConfig {
    /// Path to curation narrative TOML
    pub narrative_path: PathBuf,
    /// Name of narrative within file
    pub narrative_name: String,
    /// How often to check for new content (hours)
    pub check_interval_hours: u64,
    /// Batch size for processing
    pub batch_size: usize,
}

/// Configuration for the posting bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingConfig {
    /// Path to posting narrative TOML
    pub narrative_path: PathBuf,
    /// Name of narrative within file
    pub narrative_name: String,
    /// Base interval between posts (hours)
    pub base_interval_hours: u64,
    /// Maximum jitter to add (±minutes)
    pub jitter_minutes: u64,
}

/// Bot scheduling configuration.
#[derive(Debug, Clone)]
pub struct BotSchedule {
    /// Generation interval
    pub generation_interval: std::time::Duration,
    /// Curation check interval
    pub curation_interval: std::time::Duration,
    /// Posting base interval
    pub posting_base_interval: std::time::Duration,
    /// Posting jitter range
    pub posting_jitter: std::time::Duration,
}

impl From<&BotConfig> for BotSchedule {
    fn from(config: &BotConfig) -> Self {
        Self {
            generation_interval: std::time::Duration::from_secs(
                config.generation.interval_hours * 3600,
            ),
            curation_interval: std::time::Duration::from_secs(
                config.curation.check_interval_hours * 3600,
            ),
            posting_base_interval: std::time::Duration::from_secs(
                config.posting.base_interval_hours * 3600,
            ),
            posting_jitter: std::time::Duration::from_secs(config.posting.jitter_minutes * 60),
        }
    }
}
