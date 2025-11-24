//! Server configuration for actor-server binary.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level server configuration loaded from TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorServerConfig {
    /// Server-level settings
    #[serde(default)]
    pub server: ServerSettings,
    /// Actor configurations
    #[serde(default)]
    pub actors: Vec<ActorInstanceConfig>,
}

impl ActorServerConfig {
    /// Load server configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or TOML is invalid.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }
}

/// Server-level settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Interval in seconds between checking scheduled tasks
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    /// Maximum consecutive failures before pausing a task
    #[serde(default = "default_max_failures")]
    pub max_consecutive_failures: u32,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval(),
            max_consecutive_failures: default_max_failures(),
        }
    }
}

fn default_check_interval() -> u64 {
    60
}

fn default_max_failures() -> u32 {
    5
}

/// Configuration for a single actor instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorInstanceConfig {
    /// Actor name (unique identifier)
    pub name: String,
    /// Path to the actor's configuration TOML file
    pub config_file: String,
    /// Discord channel ID for posting
    #[serde(default)]
    pub channel_id: Option<String>,
    /// Task scheduling configuration
    #[serde(default)]
    pub schedule: ScheduleConfig,
    /// Whether this actor is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Task scheduling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduleConfig {
    /// Fixed interval in seconds
    Interval {
        /// Interval duration in seconds
        seconds: u64,
    },
    /// Cron expression (for future Phase 4 implementation)
    #[allow(dead_code)]
    Cron {
        /// Cron expression string
        expression: String,
    },
    /// One-time execution at specific time
    #[allow(dead_code)]
    Once {
        /// ISO 8601 timestamp
        at: String,
    },
    /// Execute immediately on startup
    Immediate,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self::Interval { seconds: 3600 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_config() {
        let toml = r#"
[server]
check_interval_seconds = 30
max_consecutive_failures = 3

[[actors]]
name = "test_actor"
config_file = "actors/test.toml"
channel_id = "123456"
enabled = true

[actors.schedule]
type = "Interval"
seconds = 1800
"#;

        let config: ActorServerConfig = toml::from_str(toml).expect("Valid TOML");
        assert_eq!(config.server.check_interval_seconds, 30);
        assert_eq!(config.server.max_consecutive_failures, 3);
        assert_eq!(config.actors.len(), 1);
        assert_eq!(config.actors[0].name, "test_actor");
        assert_eq!(config.actors[0].config_file, "actors/test.toml");
        assert_eq!(config.actors[0].channel_id, Some("123456".to_string()));

        match &config.actors[0].schedule {
            ScheduleConfig::Interval { seconds } => assert_eq!(*seconds, 1800),
            _ => panic!("Expected Interval schedule"),
        }
    }

    #[test]
    fn test_default_values() {
        let toml = r#"
[[actors]]
name = "minimal"
config_file = "test.toml"
"#;

        let config: ActorServerConfig = toml::from_str(toml).expect("Valid TOML");
        assert_eq!(config.server.check_interval_seconds, 60);
        assert_eq!(config.server.max_consecutive_failures, 5);
        assert!(config.actors[0].enabled);
    }

    #[test]
    fn test_immediate_schedule() {
        let toml = r#"
[[actors]]
name = "immediate"
config_file = "test.toml"

[actors.schedule]
type = "Immediate"
"#;

        let config: ActorServerConfig = toml::from_str(toml).expect("Valid TOML");
        matches!(config.actors[0].schedule, ScheduleConfig::Immediate);
    }
}
