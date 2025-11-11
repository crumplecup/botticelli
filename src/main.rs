#[cfg(feature = "database")]
use boticelli::NarrativeRepository;
use boticelli::{BoticelliConfig, BoticelliDriver, Narrative, NarrativeExecutor, Tier, TierConfig};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI rate limiting options
#[derive(Debug, Clone)]
struct RateLimitOptions {
    tier: Option<String>,
    rpm: Option<u32>,
    tpm: Option<u64>,
    rpd: Option<u32>,
    max_concurrent: Option<u32>,
    cost_input: Option<f64>,
    cost_output: Option<f64>,
    no_rate_limit: bool,
}

impl RateLimitOptions {
    /// Apply CLI overrides to a tier configuration
    fn apply_to_config(&self, mut config: TierConfig) -> TierConfig {
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

    /// Build a tier configuration from CLI overrides and config
    fn build_tier(
        &self,
        provider: &str,
    ) -> Result<Option<Box<dyn Tier>>, Box<dyn std::error::Error>> {
        // If --no-rate-limit is set, return None (no tier)
        if self.no_rate_limit && self.tier.is_none() && self.rpm.is_none() && self.tpm.is_none() {
            return Ok(None);
        }

        // Load config file
        let config = BoticelliConfig::load().ok();

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
            TierConfig {
                name: tier_name.unwrap_or_else(|| "CLI Override".to_string()),
                rpm: self.rpm,
                tpm: self.tpm,
                rpd: self.rpd,
                max_concurrent: self.max_concurrent,
                daily_quota_usd: None,
                cost_per_million_input_tokens: self.cost_input,
                cost_per_million_output_tokens: self.cost_output,
            }
        };

        // Apply CLI overrides to base config
        tier_config = self.apply_to_config(tier_config);

        Ok(Some(Box::new(tier_config) as Box<dyn Tier>))
    }
}

#[derive(Parser)]
#[command(name = "boticelli")]
#[command(about = "CLI for executing multi-act LLM narratives", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            narrative,
            backend,
            api_key,
            #[cfg(feature = "database")]
            save,
            verbose,
            tier,
            rpm,
            tpm,
            rpd,
            max_concurrent,
            cost_input,
            cost_output,
            no_rate_limit,
        } => {
            let rate_limit_opts = RateLimitOptions {
                tier,
                rpm,
                tpm,
                rpd,
                max_concurrent,
                cost_input,
                cost_output,
                no_rate_limit,
            };

            run_narrative(
                narrative,
                backend,
                api_key,
                #[cfg(feature = "database")]
                save,
                verbose,
                rate_limit_opts,
            )
            .await?;
        }

        #[cfg(feature = "database")]
        Commands::List { name, limit } => {
            list_executions(name, limit).await?;
        }

        #[cfg(feature = "database")]
        Commands::Show { id } => {
            show_execution(id).await?;
        }
    }

    Ok(())
}

async fn run_narrative(
    narrative_path: PathBuf,
    backend: String,
    _api_key: Option<String>,
    #[cfg(feature = "database")] _save: bool,
    _verbose: bool,
    rate_limit_opts: RateLimitOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the narrative
    println!("📖 Loading narrative from {:?}...", narrative_path);
    let content = std::fs::read_to_string(&narrative_path)?;
    let narrative: Narrative = content.parse()?;

    println!("✓ Loaded: {}", narrative.metadata.name);
    println!("  Description: {}", narrative.metadata.description);
    println!("  Acts: {}", narrative.toc.order.len());

    // Build tier configuration from CLI options
    let tier = rate_limit_opts.build_tier(&backend)?;

    // Display rate limiting status
    if let Some(ref t) = tier {
        println!(
            "  Rate Limiting: {} (RPM: {:?}, TPM: {:?}, RPD: {:?})",
            t.name(),
            t.rpm(),
            t.tpm(),
            t.rpd()
        );
    } else {
        println!("  Rate Limiting: Disabled");
    }
    println!();

    // Dispatch to backend-specific execution
    #[cfg(feature = "gemini")]
    if backend == "gemini" {
        // Set API key in environment if provided via command line
        if let Some(key) = _api_key {
            unsafe {
                std::env::set_var("GEMINI_API_KEY", key);
            }
        }

        let driver = boticelli::GeminiClient::new_with_tier(tier)?;
        execute_with_driver(
            driver,
            narrative,
            #[cfg(feature = "database")]
            _save,
            _verbose,
        )
        .await?;
        return Ok(());
    }

    Err(format!(
        "Unsupported backend: {} (feature may not be enabled)",
        backend
    )
    .into())
}

#[allow(dead_code)]
async fn execute_with_driver<D: BoticelliDriver>(
    driver: D,
    narrative: Narrative,
    #[cfg(feature = "database")] save: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create executor
    let executor = NarrativeExecutor::new(driver);

    // Execute the narrative
    println!("🚀 Executing narrative...\n");
    let start_time = std::time::Instant::now();

    let execution = match execute_with_progress(&executor, &narrative, verbose).await {
        Ok(exec) => exec,
        Err(e) => {
            eprintln!("❌ Execution failed: {}", e);
            return Err(e);
        }
    };

    let duration = start_time.elapsed();
    println!("\n✓ Execution completed in {:.2}s", duration.as_secs_f64());
    println!("  Total acts: {}", execution.act_executions.len());
    println!();

    // Save to database if requested
    #[cfg(feature = "database")]
    if save {
        println!("💾 Saving to database...");
        let conn = boticelli::establish_connection()?;
        let repo = boticelli::PostgresNarrativeRepository::new(conn);
        let execution_id = repo.save_execution(&execution).await?;
        println!("✓ Saved as execution ID: {}", execution_id);
    }

    // Display results summary
    println!("📊 Results:");
    for (i, act) in execution.act_executions.iter().enumerate() {
        println!("\n  Act {}: {}", i + 1, act.act_name);
        if let Some(ref model) = act.model {
            println!("    Model: {}", model);
        }
        let preview = if act.response.len() > 100 {
            format!("{}...", &act.response[..100])
        } else {
            act.response.clone()
        };
        println!("    Response: {}", preview);
    }

    Ok(())
}

#[allow(dead_code)]
async fn execute_with_progress<D: BoticelliDriver>(
    executor: &NarrativeExecutor<D>,
    narrative: &Narrative,
    verbose: bool,
) -> Result<boticelli::NarrativeExecution, Box<dyn std::error::Error>> {
    if verbose {
        println!(
            "Executing {} acts in sequence:\n",
            narrative.toc.order.len()
        );
    }

    // For now, execute all at once and show progress
    // TODO: In future, could implement streaming/incremental execution
    let execution = executor.execute(narrative).await?;

    if verbose {
        for (i, act) in execution.act_executions.iter().enumerate() {
            println!(
                "  ✓ Act {}/{}: {} ({} chars)",
                i + 1,
                execution.act_executions.len(),
                act.act_name,
                act.response.len()
            );
        }
    }

    Ok(execution)
}

#[cfg(feature = "database")]
async fn list_executions(
    name_filter: Option<String>,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use boticelli::ExecutionFilter;

    let conn = boticelli::establish_connection()?;
    let repo = boticelli::PostgresNarrativeRepository::new(conn);

    let mut filter = ExecutionFilter::new().with_limit(limit);
    if let Some(name) = name_filter {
        filter = filter.with_narrative_name(name);
    }

    let summaries = repo.list_executions(&filter).await?;

    if summaries.is_empty() {
        println!("No executions found.");
        return Ok(());
    }

    println!("📋 Stored Executions:\n");
    for summary in summaries {
        println!("  ID: {}", summary.id);
        println!("  Narrative: {}", summary.narrative_name);
        println!("  Status: {}", summary.status);
        println!("  Started: {}", summary.started_at);
        if let Some(completed) = summary.completed_at {
            println!("  Completed: {}", completed);
        }
        println!("  Acts: {}", summary.act_count);
        println!();
    }

    Ok(())
}

#[cfg(feature = "database")]
async fn show_execution(id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let conn = boticelli::establish_connection()?;
    let repo = boticelli::PostgresNarrativeRepository::new(conn);

    let execution = repo.load_execution(id).await?;

    println!("📖 Execution ID: {}", id);
    println!("Narrative: {}", execution.narrative_name);
    println!("Acts: {}\n", execution.act_executions.len());

    for (i, act) in execution.act_executions.iter().enumerate() {
        println!("Act {}: {}", i + 1, act.act_name);
        if let Some(ref model) = act.model {
            println!("  Model: {}", model);
        }
        if let Some(temp) = act.temperature {
            println!("  Temperature: {}", temp);
        }
        if let Some(max) = act.max_tokens {
            println!("  Max tokens: {}", max);
        }
        println!("  Inputs: {}", act.inputs.len());
        for (j, input) in act.inputs.iter().enumerate() {
            println!("    Input {}: {:?}", j + 1, input);
        }
        println!("  Response: {}", act.response);
        println!();
    }

    Ok(())
}
