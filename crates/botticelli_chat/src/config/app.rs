use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{
    ChatConfig, EnvironmentConfig, EnvironmentMode, McpClientConfig, McpServerConfig,
    ObservabilityConfig, PostgresConfig,
};

/// Complete application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ChatAppConfig {
    /// Environment configuration.
    #[serde(default)]
    environment: EnvironmentConfig,

    /// PostgreSQL configuration.
    #[serde(default)]
    postgres: PostgresConfig,

    /// MCP server configuration.
    #[serde(default)]
    mcp_server: McpServerConfig,

    /// MCP client configuration.
    #[serde(default)]
    mcp_client: McpClientConfig,

    /// Chat configuration.
    #[serde(default)]
    chat: ChatConfig,

    /// Observability configuration.
    #[serde(default)]
    observability: ObservabilityConfig,
}

impl ChatAppConfig {
    /// Load configuration with precedence: CLI > ENV > File > Defaults.
    ///
    /// # Precedence Order
    /// 1. Values passed in `overrides`
    /// 2. Environment variables (BOTTICELLI_*)
    /// 3. Config file at `config_path`
    /// 4. Defaults
    #[tracing::instrument]
    pub fn load(config_path: Option<&Path>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Start with defaults
        let defaults = Self::default();
        builder = builder
            .set_default("environment.mode", "local")?
            .set_default("postgres.host", defaults.postgres.host().clone())?
            .set_default("postgres.port", *defaults.postgres.port() as i64)?
            .set_default("postgres.user", defaults.postgres.user().clone())?
            .set_default("postgres.password", defaults.postgres.password().clone())?
            .set_default("postgres.database", defaults.postgres.database().clone())?
            .set_default("mcp_server.host", defaults.mcp_server.host().clone())?
            .set_default("mcp_server.port", *defaults.mcp_server.port() as i64)?
            .set_default(
                "mcp_client.timeout_seconds",
                *defaults.mcp_client.timeout_seconds() as i64,
            )?
            .set_default(
                "mcp_client.retry_attempts",
                *defaults.mcp_client.retry_attempts() as i64,
            )?
            .set_default(
                "observability.rust_log",
                defaults.observability.rust_log().clone(),
            )?
            .set_default(
                "observability.otel_exporter",
                defaults.observability.otel_exporter().clone(),
            )?
            .set_default(
                "observability.otel_endpoint",
                defaults.observability.otel_endpoint().clone(),
            )?;

        // Load from file if provided
        if let Some(path) = config_path {
            tracing::debug!(path = ?path, "Loading config file");
            builder = builder.add_source(File::from(path).required(false));
        } else {
            // Try default locations
            builder = builder
                .add_source(File::with_name("chat").required(false))
                .add_source(File::with_name("config/chat").required(false));
        }

        // Load from environment variables (BOTTICELLI_*)
        builder = builder.add_source(
            Environment::with_prefix("BOTTICELLI")
                .separator("__")
                .try_parsing(true),
        );

        let built_config = builder.build()?;

        // Check which values were explicitly set before consuming config
        let postgres_host_set = built_config.get_string("postgres.host").is_ok();
        let mcp_host_set = built_config.get_string("mcp_server.host").is_ok();

        // Deserialize into our config struct (consumes built_config)
        let mut app_config: ChatAppConfig = built_config.try_deserialize()?;

        // Apply environment-aware defaults if not explicitly set
        let mode = *app_config.environment.mode();
        if !postgres_host_set {
            app_config.postgres = app_config
                .postgres
                .clone()
                .with_host(mode.postgres_host_default().to_string());
        }
        if !mcp_host_set {
            app_config.mcp_server = app_config
                .mcp_server
                .clone()
                .with_host(mode.mcp_server_host_default().to_string());
        }

        tracing::info!(
            mode = ?mode,
            postgres_host = %app_config.postgres.host(),
            mcp_host = %app_config.mcp_server.host(),
            "Loaded configuration"
        );

        Ok(app_config)
    }

    /// Create builder for custom configuration.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

impl Default for ChatAppConfig {
    fn default() -> Self {
        let mode = EnvironmentMode::Local;
        Self {
            environment: EnvironmentConfig::default(),
            postgres: PostgresConfig::with_defaults(mode),
            mcp_server: McpServerConfig::with_defaults(mode),
            mcp_client: McpClientConfig::default(),
            chat: ChatConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Builder for custom configuration with overrides.
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    config_path: Option<std::path::PathBuf>,
    mode: Option<EnvironmentMode>,
    postgres_host: Option<String>,
    postgres_port: Option<u16>,
    postgres_user: Option<String>,
    postgres_password: Option<String>,
    postgres_database: Option<String>,
    mcp_host: Option<String>,
    mcp_port: Option<u16>,
}

impl ConfigBuilder {
    /// Set config file path.
    pub fn config_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set environment mode.
    pub fn mode(mut self, mode: EnvironmentMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set postgres host.
    pub fn postgres_host(mut self, host: impl Into<String>) -> Self {
        self.postgres_host = Some(host.into());
        self
    }

    /// Set postgres port.
    pub fn postgres_port(mut self, port: u16) -> Self {
        self.postgres_port = Some(port);
        self
    }

    /// Set postgres user.
    pub fn postgres_user(mut self, user: impl Into<String>) -> Self {
        self.postgres_user = Some(user.into());
        self
    }

    /// Set postgres password.
    pub fn postgres_password(mut self, password: impl Into<String>) -> Self {
        self.postgres_password = Some(password.into());
        self
    }

    /// Set postgres database name.
    pub fn postgres_database(mut self, database: impl Into<String>) -> Self {
        self.postgres_database = Some(database.into());
        self
    }

    /// Set MCP server host.
    pub fn mcp_host(mut self, host: impl Into<String>) -> Self {
        self.mcp_host = Some(host.into());
        self
    }

    /// Set MCP server port.
    pub fn mcp_port(mut self, port: u16) -> Self {
        self.mcp_port = Some(port);
        self
    }

    /// Build configuration.
    pub fn build(self) -> Result<ChatAppConfig, ConfigError> {
        let mut config = ChatAppConfig::load(self.config_path.as_deref())?;

        // Apply overrides
        if let Some(mode) = self.mode {
            config.environment = config.environment.clone().with_mode(mode);

            // Update dependent defaults if not overridden
            if self.postgres_host.is_none() {
                config.postgres = config
                    .postgres
                    .clone()
                    .with_host(mode.postgres_host_default().to_string());
            }
            if self.mcp_host.is_none() {
                config.mcp_server = config
                    .mcp_server
                    .clone()
                    .with_host(mode.mcp_server_host_default().to_string());
            }
        }

        if let Some(host) = self.postgres_host {
            config.postgres = config.postgres.clone().with_host(host);
        }
        if let Some(port) = self.postgres_port {
            config.postgres = config.postgres.clone().with_port(port);
        }
        if let Some(user) = self.postgres_user {
            config.postgres = config.postgres.clone().with_user(user);
        }
        if let Some(password) = self.postgres_password {
            config.postgres = config.postgres.clone().with_password(password);
        }
        if let Some(database) = self.postgres_database {
            config.postgres = config.postgres.clone().with_database(database);
        }
        if let Some(host) = self.mcp_host {
            config.mcp_server = config.mcp_server.clone().with_host(host);
        }
        if let Some(port) = self.mcp_port {
            config.mcp_server = config.mcp_server.clone().with_port(port);
        }

        Ok(config)
    }
}
