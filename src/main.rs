#[cfg(feature = "database")]
use boticelli::NarrativeRepository;
use boticelli::{BoticelliDriver, Cli, Commands, Narrative, NarrativeExecutor, RateLimitOptions};
use clap::Parser;
use std::path::PathBuf;

#[cfg(feature = "discord")]
use boticelli::DiscordCommands;

#[cfg(feature = "database")]
use boticelli::ContentCommands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

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
            #[cfg(all(feature = "database", feature = "discord"))]
            process_discord,
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
                #[cfg(all(feature = "database", feature = "discord"))]
                process_discord,
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

        #[cfg(feature = "discord")]
        Commands::Discord { command } => match command {
            DiscordCommands::Start { token } => {
                start_discord_bot(token).await?;
            }
        },

        #[cfg(feature = "database")]
        Commands::Content { command } => match command {
            ContentCommands::List {
                table,
                status,
                limit,
            } => {
                list_content(&table, status.as_deref(), limit).await?;
            }
            ContentCommands::Show { table, id } => {
                show_content(&table, id).await?;
            }
            ContentCommands::Tag {
                table,
                id,
                tags,
                rating,
            } => {
                tag_content(&table, id, tags.as_deref(), rating).await?;
            }
            ContentCommands::Review { table, id, status } => {
                review_content(&table, id, &status).await?;
            }
            ContentCommands::Delete { table, id, yes } => {
                delete_content(&table, id, yes).await?;
            }
            ContentCommands::Promote { table, id, target } => {
                promote_content(&table, id, target.as_deref()).await?;
            }
        },

        #[cfg(feature = "tui")]
        Commands::Tui { table } => {
            run_tui_app(table).await?;
        }
    }

    Ok(())
}

#[cfg(feature = "database")]
fn create_postgres_repository()
-> Result<boticelli::PostgresNarrativeRepository, Box<dyn std::error::Error>> {
    let conn = boticelli::establish_connection()?;

    // Create filesystem storage in temp directory
    // TODO: Make this configurable via CLI args or config file
    let storage_path = std::env::temp_dir().join("boticelli_media");
    let storage = std::sync::Arc::new(boticelli::FileSystemStorage::new(storage_path)?);

    Ok(boticelli::PostgresNarrativeRepository::new(conn, storage))
}

async fn run_narrative(
    narrative_path: PathBuf,
    backend: String,
    _api_key: Option<String>,
    #[cfg(feature = "database")] _save: bool,
    _verbose: bool,
    rate_limit_opts: RateLimitOptions,
    #[cfg(all(feature = "database", feature = "discord"))] process_discord: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the narrative
    println!("📖 Loading narrative from {:?}...", narrative_path);
    
    #[cfg(feature = "database")]
    let narrative = {
        // Check if narrative has a template field by parsing metadata first
        let content = std::fs::read_to_string(&narrative_path)?;
        let has_template = content.contains("template =");
        
        if has_template {
            // Load with database connection for prompt assembly
            let mut conn = boticelli::establish_connection()?;
            Narrative::from_file_with_db(&narrative_path, &mut conn)?
        } else {
            // Load normally without database
            Narrative::from_file(&narrative_path)?
        }
    };
    
    #[cfg(not(feature = "database"))]
    let narrative = Narrative::from_file(&narrative_path)?;

    println!("✓ Loaded: {}", narrative.metadata.name);
    println!("  Description: {}", narrative.metadata.description);
    println!("  Acts: {}", narrative.toc.order.len());
    
    if let Some(ref template) = narrative.metadata.template {
        println!("  Template: {} (schema-based content generation)", template);
    }

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
            #[cfg(all(feature = "database", feature = "discord"))]
            process_discord,
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
    #[cfg(all(feature = "database", feature = "discord"))] process_discord: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build processor registry based on enabled features and narrative configuration
    #[cfg(feature = "database")]
    let executor = {
        let has_template = narrative.metadata.template.is_some();
        
        #[cfg(feature = "discord")]
        let needs_discord = process_discord;
        #[cfg(not(feature = "discord"))]
        let needs_discord = false;
        
        if has_template || needs_discord {
            let mut registry = boticelli::ProcessorRegistry::new();
            
            // Register content generation processor if template present
            if has_template {
                println!("🔧 Enabling content generation processing...");
                let conn = boticelli::establish_connection()?;
                let content_processor = boticelli::ContentGenerationProcessor::new(
                    std::sync::Arc::new(std::sync::Mutex::new(conn))
                );
                registry.register(Box::new(content_processor));
                println!("✓ Registered content generation processor");
            }
            
            // Register Discord processors if requested
            #[cfg(feature = "discord")]
            if needs_discord {
                println!("🔧 Enabling Discord data processing...");
                let conn = boticelli::establish_connection()?;
                let repo = std::sync::Arc::new(boticelli::DiscordRepository::new(conn));
                
                registry.register(Box::new(boticelli::DiscordGuildProcessor::new(repo.clone())));
                registry.register(Box::new(boticelli::DiscordUserProcessor::new(repo.clone())));
                registry.register(Box::new(boticelli::DiscordChannelProcessor::new(repo.clone())));
                registry.register(Box::new(boticelli::DiscordRoleProcessor::new(repo.clone())));
                registry.register(Box::new(boticelli::DiscordGuildMemberProcessor::new(repo.clone())));
                registry.register(Box::new(boticelli::DiscordMemberRoleProcessor::new(repo.clone())));
                
                println!("✓ Registered 6 Discord processors");
            }
            
            println!();
            boticelli::NarrativeExecutor::with_processors(driver, registry)
        } else {
            boticelli::NarrativeExecutor::new(driver)
        }
    };
    
    // Create executor (when database feature not enabled)
    #[cfg(not(feature = "database"))]
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
        let repo = create_postgres_repository()?;
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

    let repo = create_postgres_repository()?;

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
    let repo = create_postgres_repository()?;

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

#[cfg(feature = "discord")]
async fn start_discord_bot(token: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Get token from command line or environment
    let token = token
        .or_else(|| std::env::var("DISCORD_TOKEN").ok())
        .ok_or(
            "Discord token not provided. Use --token or set DISCORD_TOKEN environment variable",
        )?;

    println!("🤖 Starting Boticelli Discord bot...");

    // Establish database connection
    let conn = boticelli::establish_connection()?;

    // Create and start the bot
    let mut bot = boticelli::BoticelliBot::new(token, conn).await?;

    println!("✓ Bot initialized successfully");
    println!("🚀 Connecting to Discord...");
    println!("   (Press Ctrl+C to stop)");
    println!();

    // Start the bot (this blocks until shutdown)
    bot.start().await?;

    Ok(())
}

// Content management functions
#[cfg(feature = "database")]
async fn list_content(
    table: &str,
    status: Option<&str>,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = boticelli::establish_connection()?;

    let results = boticelli::list_content(&mut conn, table, status, limit)?;

    if results.is_empty() {
        println!("No content found in table '{}'", table);
        if let Some(s) = status {
            println!("  (filtered by status: {})", s);
        }
        return Ok(());
    }

    println!("📋 Content from '{}':\n", table);
    for item in &results {
        let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let status = item
            .get("review_status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let generated_at = item
            .get("generated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        println!("  ID: {}", id);
        println!("  Status: {}", status);
        println!("  Generated: {}", generated_at);

        // Show content fields (skip metadata)
        let metadata_fields = [
            "id",
            "generated_at",
            "source_narrative",
            "source_act",
            "generation_model",
            "review_status",
            "tags",
            "rating",
        ];

        for (key, value) in item.as_object().unwrap() {
            if !metadata_fields.contains(&key.as_str()) {
                println!("  {}: {}", key, value);
            }
        }
        println!();
    }

    println!("Total: {} items", results.len());
    Ok(())
}

#[cfg(feature = "database")]
async fn show_content(table: &str, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = boticelli::establish_connection()?;

    let item = boticelli::get_content_by_id(&mut conn, table, id)?;

    println!("📄 Content from '{}' (ID: {})\n", table, id);

    // Pretty print JSON
    println!("{}", serde_json::to_string_pretty(&item)?);

    Ok(())
}

#[cfg(feature = "database")]
async fn tag_content(
    table: &str,
    id: i64,
    tags: Option<&str>,
    rating: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = boticelli::establish_connection()?;

    let tag_list = tags.map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    });

    boticelli::update_content_metadata(&mut conn, table, id, tag_list.as_deref(), rating)?;

    println!("✓ Updated content {} in '{}'", id, table);
    if let Some(t) = tags {
        println!("  Tags: {}", t);
    }
    if let Some(r) = rating {
        println!("  Rating: {}/5", r);
    }

    Ok(())
}

#[cfg(feature = "database")]
async fn review_content(
    table: &str,
    id: i64,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = boticelli::establish_connection()?;

    boticelli::update_review_status(&mut conn, table, id, status)?;

    println!("✓ Updated review status for content {} in '{}'", id, table);
    println!("  Status: {}", status);

    Ok(())
}

#[cfg(feature = "database")]
async fn delete_content(table: &str, id: i64, yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !yes {
        print!("Delete content {} from '{}'? [y/N] ", id, table);
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let mut conn = boticelli::establish_connection()?;
    boticelli::delete_content(&mut conn, table, id)?;

    println!("✓ Deleted content {} from '{}'", id, table);
    Ok(())
}

#[cfg(feature = "database")]
async fn promote_content(
    table: &str,
    id: i64,
    target: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = boticelli::establish_connection()?;

    // Determine target table
    // If not specified, try to derive from source table name
    // e.g., "potential_posts" -> "discord_channels" (based on template)
    let target_table = if let Some(t) = target {
        t.to_string()
    } else {
        // Try to infer from table comment which has the template name
        // For now, require explicit target
        return Err("Target table must be specified with --target flag. \
             Example: --target discord_channels"
            .into());
    };

    println!(
        "🚀 Promoting content {} from '{}' to '{}'...",
        id, table, target_table
    );

    let new_id = boticelli::promote_content(&mut conn, table, &target_table, id)?;

    println!("✓ Content promoted successfully!");
    println!("  Source: {} (ID: {})", table, id);
    println!("  Target: {} (ID: {})", target_table, new_id);

    Ok(())
}

#[cfg(feature = "tui")]
async fn run_tui_app(table: String) -> Result<(), Box<dyn std::error::Error>> {
    let conn = boticelli::establish_connection()?;
    boticelli::run_tui(table, conn)?;
    Ok(())
}
