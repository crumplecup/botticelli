//! Content management command handlers.

use super::commands::{ContentCommands, OutputFormat};
use botticelli_error::BotticelliResult;

/// Handle content management commands.
pub async fn handle_content_command(cmd: ContentCommands) -> BotticelliResult<()> {
    match cmd {
        ContentCommands::List {
            table,
            status,
            limit,
            format,
        } => list_content(&table, status.as_deref(), limit, format).await,

        ContentCommands::Show { table, id } => show_content(&table, &id).await,

        ContentCommands::Last { format } => last_generation(format).await,

        ContentCommands::Generations { status, limit } => {
            list_generations(status.as_deref(), limit).await
        }
    }
}

// ── Storage helper ────────────────────────────────────────────────────────────

#[cfg(feature = "database")]
fn open_storage() -> BotticelliResult<std::sync::Arc<dyn botticelli_interface::BotStorage>> {
    use botticelli_error::{BackendError, BotticelliError};
    let db_path = dirs::data_dir()
        .map(|d| d.join("botticelli").join("botticelli.redb"))
        .ok_or_else(|| BackendError::new("Cannot determine data directory"))?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;
    }
    Ok(std::sync::Arc::new(
        botticelli_database::RedbStorage::open(&db_path)
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?,
    ))
}

// ── list_content ──────────────────────────────────────────────────────────────

#[cfg(feature = "database")]
async fn list_content(
    table: &str,
    _status: Option<&str>,
    limit: i64,
    format: OutputFormat,
) -> BotticelliResult<()> {
    let storage = open_storage()?;
    let content = storage
        .list_content(table, limit as usize)
        .await
        .map_err(|e| botticelli_error::BackendError::new(format!("{e}")))?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&content)
                .map_err(|e| botticelli_error::JsonError::new(e.to_string()))?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            println!("Content from table '{}':", table);
            println!("{:-<80}", "");
            for item in &content {
                let json = serde_json::to_string_pretty(item.content_json())
                    .map_err(|e| botticelli_error::JsonError::new(e.to_string()))?;
                println!("id: {}  created: {}", item.id(), item.created_at());
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

// ── show_content ──────────────────────────────────────────────────────────────

#[cfg(feature = "database")]
async fn show_content(table: &str, id: &str) -> BotticelliResult<()> {
    let storage = open_storage()?;
    match storage
        .get_content(id)
        .await
        .map_err(|e| botticelli_error::BackendError::new(format!("{e}")))?
    {
        Some(item) if item.table_name() == table => {
            let json = serde_json::to_string_pretty(item.content_json())
                .map_err(|e| botticelli_error::JsonError::new(e.to_string()))?;
            println!("{}", json);
        }
        Some(_) => {
            eprintln!("Content {} does not belong to table '{}'", id, table);
            std::process::exit(1);
        }
        None => {
            eprintln!("Content {} not found", id);
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn show_content(_table: &str, _id: &str) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}

// ── last_generation ───────────────────────────────────────────────────────────

#[cfg(feature = "database")]
async fn last_generation(format: OutputFormat) -> BotticelliResult<()> {
    let storage = open_storage()?;
    let generations = storage
        .list_content_generations(50)
        .await
        .map_err(|e| botticelli_error::BackendError::new(format!("{e}")))?;

    match generations.into_iter().find(|g| g.status() == "success") {
        Some(generation) => match format {
            OutputFormat::TableNameOnly => {
                println!("{}", generation.table_name());
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&generation)
                    .map_err(|e| botticelli_error::JsonError::new(e.to_string()))?;
                println!("{}", json);
            }
            OutputFormat::Human => {
                println!("Last generated table: {}", generation.table_name());
                println!("  Narrative: {}", generation.narrative_name());
                println!("  File: {}", generation.narrative_file());
                println!("  Generated: {}", generation.generated_at());
                if let Some(rows) = generation.row_count() {
                    println!("  Rows: {}", rows);
                }
                if let Some(ms) = generation.generation_duration_ms() {
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

// ── list_generations ──────────────────────────────────────────────────────────

#[cfg(feature = "database")]
async fn list_generations(status: Option<&str>, limit: i64) -> BotticelliResult<()> {
    let storage = open_storage()?;
    let mut generations = storage
        .list_content_generations(limit as usize)
        .await
        .map_err(|e| botticelli_error::BackendError::new(format!("{e}")))?;

    if let Some(s) = status {
        generations.retain(|g| g.status() == s);
    }

    println!(
        "{:<20} {:<15} {:<10} {:<20}",
        "Table", "Status", "Rows", "Generated"
    );
    println!("{:-<70}", "");

    for generation in generations {
        println!(
            "{:<20} {:<15} {:<10} {:<20}",
            generation.table_name(),
            generation.status(),
            generation
                .row_count()
                .map(|r| r.to_string())
                .unwrap_or_else(|| "-".to_string()),
            generation.generated_at().format("%Y-%m-%d %H:%M")
        );
    }

    Ok(())
}

#[cfg(not(feature = "database"))]
async fn list_generations(_status: Option<&str>, _limit: i64) -> BotticelliResult<()> {
    eprintln!("Error: Database feature not enabled. Rebuild with --features database");
    std::process::exit(1);
}
