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

        /// Save execution results to database
        #[arg(long)]
        save: bool,

        /// Process Discord infrastructure (guilds, channels, etc.)
        #[arg(long)]
        process_discord: bool,
    },

    /// Launch the terminal user interface for a table
    Tui {
        /// Name of the table to view
        table: String,
    },

    /// Content management commands
    #[command(subcommand)]
    Content(ContentCommands),
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
