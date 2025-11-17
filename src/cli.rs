//! Command-line interface for Boticelli.
//!
//! This module provides the CLI structure and commands for the Boticelli
//! narrative execution system. The CLI is built with clap and supports:
//!
//! - Narrative execution with various backends
//! - Rate limiting configuration
//! - Database storage and retrieval
//! - Discord bot integration
//! - Content generation management

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI rate limiting options.
///
/// Allows overriding rate limits from config files or environment variables.
/// All limits can be overridden individually, or disabled entirely with `no_rate_limit`.
#[derive(Debug, Clone)]
pub struct RateLimitOptions {
    /// API tier name to use
    pub tier: Option<String>,
    /// Requests per minute limit override
    pub rpm: Option<u32>,
    /// Tokens per minute limit override
    pub tpm: Option<u64>,
    /// Requests per day limit override
    pub rpd: Option<u32>,
    /// Maximum concurrent requests override
    pub max_concurrent: Option<u32>,
    /// Input token cost override (per million tokens)
    pub cost_input: Option<f64>,
    /// Output token cost override (per million tokens)
    pub cost_output: Option<f64>,
    /// Disable all rate limiting
    pub no_rate_limit: bool,
}

impl RateLimitOptions {
    /// Apply CLI overrides to a tier configuration.
    ///
    /// Takes a base `TierConfig` and applies any CLI-specified overrides.
    /// If `no_rate_limit` is true, removes all limit fields.
    ///
    /// # Arguments
    ///
    /// * `config` - Base tier configuration to modify
    ///
    /// # Returns
    ///
    /// Modified configuration with CLI overrides applied
    pub fn apply_to_config(&self, mut config: crate::TierConfig) -> crate::TierConfig {
        if self.no_rate_limit {
            // Remove all limits
            config.rpm = None;
            config.tpm = None;
            config.rpd = None;
            config.max_concurrent = None;
        } else {
            // Apply individual overrides
            if let Some(rpm) = self.rpm {
                config.rpm = Some(rpm);
            }
            if let Some(tpm) = self.tpm {
                config.tpm = Some(tpm);
            }
            if let Some(rpd) = self.rpd {
                config.rpd = Some(rpd);
            }
            if let Some(max_concurrent) = self.max_concurrent {
                config.max_concurrent = Some(max_concurrent);
            }
        }

        // Apply cost overrides
        if let Some(cost_input) = self.cost_input {
            config.cost_per_million_input_tokens = Some(cost_input);
        }
        if let Some(cost_output) = self.cost_output {
            config.cost_per_million_output_tokens = Some(cost_output);
        }

        config
    }

    /// Build a tier configuration from CLI overrides and config files.
    ///
    /// Resolution order:
    /// 1. CLI arguments (highest priority)
    /// 2. Environment variables (e.g., `GEMINI_TIER`)
    /// 3. Config file default tier
    /// 4. Basic config from CLI overrides only (if no config file)
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider name (e.g., "gemini", "anthropic")
    ///
    /// # Returns
    ///
    /// `Some(tier)` with configuration, or `None` if rate limiting disabled
    ///
    /// # Errors
    ///
    /// Returns error if tier name specified but not found in config
    pub fn build_tier(
        &self,
        provider: &str,
    ) -> Result<Option<Box<dyn crate::Tier>>, Box<dyn std::error::Error>> {
        // If --no-rate-limit is set and no other options, return None
        if self.no_rate_limit && self.tier.is_none() && self.rpm.is_none() && self.tpm.is_none() {
            return Ok(None);
        }

        // Load config file
        let config = crate::BoticelliConfig::load().ok();

        // Get tier name from: CLI > Env > Config default
        let tier_name = self
            .tier
            .clone()
            .or_else(|| {
                let env_var = format!("{}_TIER", provider.to_uppercase());
                std::env::var(&env_var).ok()
            })
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|c| c.providers.get(provider))
                    .map(|p| p.default_tier.clone())
            });

        // Load base tier config
        let mut tier_config = if let Some(cfg) = config {
            cfg.get_tier(provider, tier_name.as_deref())
                .ok_or_else(|| {
                    format!(
                        "Tier '{}' not found for provider '{}'",
                        tier_name.as_deref().unwrap_or("default"),
                        provider
                    )
                })?
        } else {
            // If no config file, create a basic config from CLI overrides
            crate::TierConfig {
                name: tier_name.unwrap_or_else(|| "CLI Override".to_string()),
                rpm: self.rpm,
                tpm: self.tpm,
                rpd: self.rpd,
                max_concurrent: self.max_concurrent,
                daily_quota_usd: None,
                cost_per_million_input_tokens: self.cost_input,
                cost_per_million_output_tokens: self.cost_output,
                models: std::collections::HashMap::new(),
            }
        };

        // Apply CLI overrides to base config
        tier_config = self.apply_to_config(tier_config);

        Ok(Some(Box::new(tier_config) as Box<dyn crate::Tier>))
    }
}

/// Boticelli CLI - Multi-act LLM narrative execution system.
#[derive(Parser)]
#[command(name = "boticelli")]
#[command(about = "CLI for executing multi-act LLM narratives", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Execute a narrative from a TOML file
    Run {
        /// Path to narrative TOML file
        #[arg(short, long)]
        narrative: PathBuf,

        /// LLM backend to use (gemini, anthropic, etc.)
        #[arg(short, long, default_value = "gemini")]
        backend: String,

        /// API key (or use environment variable)
        #[arg(short, long)]
        api_key: Option<String>,

        /// Save execution to database
        #[arg(short, long)]
        #[cfg(feature = "database")]
        save: bool,

        /// Show detailed progress
        #[arg(short, long)]
        verbose: bool,

        // Rate limiting options
        /// API tier to use (overrides config and env)
        #[arg(long)]
        tier: Option<String>,

        /// Override requests per minute limit
        #[arg(long)]
        rpm: Option<u32>,

        /// Override tokens per minute limit
        #[arg(long)]
        tpm: Option<u64>,

        /// Override requests per day limit
        #[arg(long)]
        rpd: Option<u32>,

        /// Override max concurrent requests
        #[arg(long)]
        max_concurrent: Option<u32>,

        /// Override input token cost (per million)
        #[arg(long)]
        cost_input: Option<f64>,

        /// Override output token cost (per million)
        #[arg(long)]
        cost_output: Option<f64>,

        /// Disable rate limiting entirely
        #[arg(long)]
        no_rate_limit: bool,

        /// Enable Discord data processing (extract JSON and insert to database)
        #[arg(long)]
        #[cfg(all(feature = "database", feature = "discord"))]
        process_discord: bool,
    },

    /// List stored narrative executions
    #[cfg(feature = "database")]
    List {
        /// Filter by narrative name
        #[arg(short, long)]
        name: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show details of a stored execution
    #[cfg(feature = "database")]
    Show {
        /// Execution ID
        id: i32,
    },

    /// Discord bot commands
    #[cfg(feature = "discord")]
    Discord {
        /// Discord subcommand
        #[command(subcommand)]
        command: DiscordCommands,
    },

    /// Content generation management commands
    #[cfg(feature = "database")]
    Content {
        /// Content subcommand
        #[command(subcommand)]
        command: ContentCommands,
    },

    /// Launch TUI for content review
    #[cfg(feature = "tui")]
    Tui {
        /// Table name to review
        table: String,
    },
}

/// Discord bot subcommands.
#[cfg(feature = "discord")]
#[derive(Subcommand)]
pub enum DiscordCommands {
    /// Start the Discord bot
    Start {
        /// Discord bot token (or use DISCORD_TOKEN environment variable)
        #[arg(short, long)]
        token: Option<String>,
    },
}

/// Content generation management subcommands.
#[cfg(feature = "database")]
#[derive(Subcommand)]
pub enum ContentCommands {
    /// List generated content from a table
    List {
        /// Content table name (e.g., "potential_posts")
        table: String,

        /// Filter by review status (pending, approved, rejected)
        #[arg(short, long)]
        status: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show details of specific generated content
    Show {
        /// Content table name
        table: String,

        /// Content ID
        id: i64,
    },

    /// Tag or rate generated content
    Tag {
        /// Content table name
        table: String,

        /// Content ID
        id: i64,

        /// Tags to add (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Rating (1-5)
        #[arg(short, long)]
        rating: Option<i32>,
    },

    /// Update review status of content
    Review {
        /// Content table name
        table: String,

        /// Content ID
        id: i64,

        /// New status (pending, approved, rejected)
        status: String,
    },

    /// Delete generated content
    Delete {
        /// Content table name
        table: String,

        /// Content ID
        id: i64,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Promote content to production table
    Promote {
        /// Content table name (source)
        table: String,

        /// Content ID
        id: i64,

        /// Target table (defaults to removing generation prefix)
        #[arg(short, long)]
        target: Option<String>,
    },
}

impl Cli {
    /// Parse CLI arguments from the environment.
    pub fn parse_args() -> Self {
        <Self as Parser>::parse()
    }
}
