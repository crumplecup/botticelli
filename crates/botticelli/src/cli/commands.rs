//! CLI command definitions.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Botticelli - Unified LLM API interface with narrative execution and content management
#[derive(Parser, Debug)]
#[command(name = "botticelli")]
#[command(about = "Unified LLM API interface with narrative execution and content management", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Command to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a narrative from a TOML file
    Run {
        /// Path to the narrative TOML file
        #[arg(long)]
        narrative: PathBuf,

        /// Specific narrative name (for multi-narrative files)
        #[arg(long)]
        narrative_name: Option<String>,

        /// Save execution results to database
        #[arg(long)]
        save: bool,

        /// Process Discord infrastructure (guilds, channels, etc.)
        #[arg(long)]
        process_discord: bool,

        /// Directory for persistent state storage
        #[cfg(all(feature = "gemini", feature = "database"))]
        #[arg(long)]
        state_dir: Option<PathBuf>,

        /// Budget multiplier for requests per minute (0.0 < x ≤ 1.0)
        #[arg(long)]
        rpm_multiplier: Option<f64>,

        /// Budget multiplier for tokens per minute (0.0 < x ≤ 1.0)
        #[arg(long)]
        tpm_multiplier: Option<f64>,

        /// Budget multiplier for requests per day (0.0 < x ≤ 1.0)
        #[arg(long)]
        rpd_multiplier: Option<f64>,
    },

    /// Launch the terminal user interface for a table
    Tui {
        /// Name of the table to view
        table: String,
    },

    /// Content management commands
    #[command(subcommand)]
    Content(ContentCommands),

    /// Run the bot server with generation, curation, and posting bots
    Server {
        /// Override config file path
        #[arg(long)]
        config: Option<PathBuf>,

        /// Enable only specific bots (comma-separated: generation,curation,posting)
        #[arg(long)]
        only: Option<String>,
    },
}

/// Content management subcommands
#[derive(Subcommand, Debug)]
pub enum ContentCommands {
    /// List content from a generation table
    List {
        /// Name of the table to list
        table: String,

        /// Status filter
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of rows to display
        #[arg(long, default_value = "20")]
        limit: i64,

        /// Output format
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },

    /// Show a specific content item
    Show {
        /// Name of the table
        table: String,

        /// ID of the content item
        id: i64,
    },

    /// Get the most recently generated table
    Last {
        /// Output format
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },

    /// List all content generations with tracking metadata
    Generations {
        /// Status filter
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of generations to display
        #[arg(long, default_value = "20")]
        limit: i64,
    },
}

/// Output format options
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Human-readable format
    Human,
    /// JSON format
    Json,
    /// Table name only (for scripting)
    TableNameOnly,
}
