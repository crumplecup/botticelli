//! Content management command handlers.

use botticelli::BotticelliResult;
use super::commands::{ContentCommands, OutputFormat};

/// Handle content management commands.
pub async fn handle_content_command(cmd: ContentCommands) -> BotticelliResult<()> {
    match cmd {
        ContentCommands::List {
            table,
            status,
            limit,
            format,
        } => list_content(&table, status.as_deref(), limit, format).await,

        ContentCommands::Show { table, id } => show_content(&table, id).await,

        ContentCommands::Last { format } => last_generation(format).await,

        ContentCommands::Generations { status, limit } => {
            list_generations(status.as_deref(), limit).await
        }
    }
}

/// List content from a table.
#[cfg(feature = "database")]
async fn list_content(
    table: &str,
    status: Option<&str>,
    limit: i64,
    format: OutputFormat,
) -> BotticelliResult<()> {
    use botticelli::establish_connection;
    use botticelli::list_content as db_list_content;

    let mut conn = establish_connection()?;
    let content = db_list_content(&mut conn, table, status, limit as usize)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&content)
                .map_err(|e| botticelli::JsonError::new(e.to_string()))?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            println!("Content from table '{}':", table);
            println!("{:-<80}", "");
            for item in &content {
                let json = serde_json::to_string_pretty(item)
                    .map_err(|e| botticelli::JsonError::new(e.to_string()))?;
                println!("{}", json);
                println!("{:-<80}", "");
            }
            println!("Total: {} items", content.len());
        }
        OutputFormat::TableNameOnly => {
            println!("{}", table);
        }
    }

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn list_content(
    _table: &str,
    _status: Option<&str>,
    _limit: i64,
    _format: OutputFormat,
) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}

/// Show a specific content item.
#[cfg(feature = "database")]
async fn show_content(table: &str, id: i64) -> BotticelliResult<()> {
    use botticelli::establish_connection;
    use botticelli_database::content_management::get_content_by_id;

    let mut conn = establish_connection()?;
    let content = get_content_by_id(&mut conn, table, id)?;

    let json = serde_json::to_string_pretty(&content)
        .map_err(|e| botticelli::JsonError::new(e.to_string()))?;
    println!("{}", json);

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn show_content(_table: &str, _id: i64) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}

/// Get the last successful generation.
#[cfg(feature = "database")]
async fn last_generation(format: OutputFormat) -> BotticelliResult<()> {
    use botticelli::{establish_connection, ContentGenerationRepository, PostgresContentGenerationRepository};

    let mut conn = establish_connection()?;
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    match repo.get_last_successful()? {
        Some(generation) => match format {
            OutputFormat::TableNameOnly => {
                println!("{}", generation.table_name);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&generation)
                    .map_err(|e| botticelli::JsonError::new(e.to_string()))?;
                println!("{}", json);
            }
            OutputFormat::Human => {
                println!("Last generated table: {}", generation.table_name);
                println!("  Narrative: {}", generation.narrative_name);
                println!("  File: {}", generation.narrative_file);
                println!("  Generated: {}", generation.generated_at);
                if let Some(rows) = generation.row_count {
                    println!("  Rows: {}", rows);
                }
                if let Some(ms) = generation.generation_duration_ms {
                    println!("  Duration: {}ms", ms);
                }
            }
        },
        None => {
            eprintln!("No successful generations found");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn last_generation(_format: OutputFormat) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}

/// List all content generations.
#[cfg(feature = "database")]
async fn list_generations(status: Option<&str>, limit: i64) -> BotticelliResult<()> {
    use botticelli::{establish_connection, ContentGenerationRepository, PostgresContentGenerationRepository};

    let mut conn = establish_connection()?;
    let mut repo = PostgresContentGenerationRepository::new(&mut conn);

    let generations = repo.list_generations(status.map(String::from), limit)?;

    println!(
        "{:<20} {:<15} {:<10} {:<20}",
        "Table", "Status", "Rows", "Generated"
    );
    println!("{:-<70}", "");

    for generation in generations {
        println!(
            "{:<20} {:<15} {:<10} {:<20}",
            generation.table_name,
            generation.status,
            generation.row_count
                .map(|r| r.to_string())
                .unwrap_or_else(|| "-".to_string()),
            generation.generated_at.format("%Y-%m-%d %H:%M")
        );
    }

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn list_generations(_status: Option<&str>, _limit: i64) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}
