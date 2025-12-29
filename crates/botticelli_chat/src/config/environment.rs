use serde::{Deserialize, Serialize};

/// Environment mode for deployment configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentMode {
    /// Local development mode - services on localhost.
    #[default]
    Local,
    /// Container deployment mode - services in container network.
    Container,
    /// Test mode - optimized for integration testing.
    Test,
}

impl EnvironmentMode {
    /// Get default postgres host for this environment.
    pub fn postgres_host_default(&self) -> &str {
        match self {
            Self::Local => "localhost",
            Self::Container => "postgres",
            Self::Test => "localhost",
        }
    }

    /// Get default MCP server host for this environment.
    pub fn mcp_server_host_default(&self) -> &str {
        match self {
            Self::Local => "localhost",
            Self::Container => "mcp-server",
            Self::Test => "localhost",
        }
    }

    /// Get default postgres port for this environment.
    pub fn postgres_port_default(&self) -> u16 {
        match self {
            Self::Local => 5432,
            Self::Container => 5432,
            Self::Test => 5432,
        }
    }

    /// Get default MCP server port for this environment.
    pub fn mcp_server_port_default(&self) -> u16 {
        match self {
            Self::Local => 3000,
            Self::Container => 3000,
            Self::Test => 3001,
        }
    }
}

/// Environment configuration.
#[derive(
    Debug, Clone, Serialize, Deserialize, derive_getters::Getters, derive_setters::Setters,
)]
#[setters(prefix = "with_", strip_option)]
pub struct EnvironmentConfig {
    /// Deployment mode.
    #[serde(default)]
    mode: EnvironmentMode,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            mode: EnvironmentMode::Local,
        }
    }
}
