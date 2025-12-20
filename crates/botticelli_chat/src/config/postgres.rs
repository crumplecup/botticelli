use derive_getters::Getters;
use derive_setters::Setters;
use serde::{Deserialize, Serialize};

use crate::EnvironmentMode;

/// PostgreSQL database configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, Setters)]
#[setters(prefix = "with_", strip_option)]
pub struct PostgresConfig {
    /// Database host.
    host: String,

    /// Database port.
    port: u16,

    /// Database user.
    user: String,

    /// Database password.
    #[serde(skip_serializing)]
    password: String,

    /// Database name.
    database: String,
}

impl PostgresConfig {
    /// Create config with environment-aware defaults.
    pub fn with_defaults(mode: EnvironmentMode) -> Self {
        Self {
            host: mode.postgres_host_default().to_string(),
            port: mode.postgres_port_default(),
            user: "botticelli".to_string(),
            password: "botticelli".to_string(),
            database: "botticelli".to_string(),
        }
    }

    /// Build database URL from configuration.
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self::with_defaults(EnvironmentMode::Local)
    }
}
