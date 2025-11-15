//! One-time migration tool to move existing binary data to new storage backend.
//!
//! This tool migrates media data from the old storage format (binary columns in act_inputs)
//! to the new external storage system with metadata in media_references table.
//!
//! Usage:
//!   cargo run --bin migrate_media --features database
//!
//! Environment variables:
//!   DATABASE_URL - PostgreSQL connection string (required)
//!   MEDIA_STORAGE_PATH - Path for filesystem storage (default: /tmp/boticelli_media)

use boticelli::{
    establish_connection, FileSystemStorage, MediaMetadata, MediaType, NarrativeRepository,
    PostgresNarrativeRepository,
};
use diesel::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("🔄 Starting media migration");

    // Setup storage
    let storage_path = std::env::var("MEDIA_STORAGE_PATH")
        .unwrap_or_else(|_| std::env::temp_dir().join("boticelli_media").display().to_string());

    tracing::info!("📁 Storage path: {}", storage_path);

    let storage = Arc::new(FileSystemStorage::new(&storage_path)?);
    let conn = establish_connection()?;
    let repo = PostgresNarrativeRepository::new(conn, storage);

    // Query for inputs with binary data
    tracing::info!("🔍 Searching for inputs with binary data...");

    let inputs_with_binary = find_inputs_with_binary(&repo).await?;

    if inputs_with_binary.is_empty() {
        tracing::info!("✓ No binary data found to migrate");
        return Ok(());
    }

    tracing::info!("📦 Found {} inputs to migrate", inputs_with_binary.len());

    let mut migrated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for (id, data, mime_type, media_type) in inputs_with_binary {
        match migrate_input(id, &data, mime_type.as_deref(), &media_type, &repo).await {
            Ok(true) => {
                migrated += 1;
                tracing::debug!("✓ Migrated input {}", id);
            }
            Ok(false) => {
                skipped += 1;
                tracing::debug!("⊘ Skipped input {} (already migrated)", id);
            }
            Err(e) => {
                failed += 1;
                tracing::error!("✗ Failed to migrate input {}: {}", id, e);
            }
        }

        if (migrated + skipped + failed) % 100 == 0 {
            tracing::info!(
                "Progress: {} migrated, {} skipped, {} failed",
                migrated,
                skipped,
                failed
            );
        }
    }

    tracing::info!("✓ Migration complete!");
    tracing::info!("  Migrated: {}", migrated);
    tracing::info!("  Skipped:  {}", skipped);
    tracing::info!("  Failed:   {}", failed);

    if failed > 0 {
        tracing::warn!("⚠ Some migrations failed. Check logs for details.");
        std::process::exit(1);
    }

    Ok(())
}

/// Find all act_inputs with binary data that need migration.
async fn find_inputs_with_binary(
    _repo: &PostgresNarrativeRepository,
) -> Result<Vec<(i32, Vec<u8>, Option<String>, String)>, Box<dyn std::error::Error>> {
    use boticelli::database::schema::act_inputs;

    let mut conn = establish_connection()?;

    // Query for inputs with source_binary or source_base64 that don't have media_ref_id
    let results: Vec<(i32, Option<Vec<u8>>, Option<String>, Option<String>, String)> =
        act_inputs::table
            .select((
                act_inputs::id,
                act_inputs::source_binary,
                act_inputs::source_base64,
                act_inputs::mime_type,
                act_inputs::input_type,
            ))
            .filter(act_inputs::media_ref_id.is_null())
            .filter(
                act_inputs::source_binary
                    .is_not_null()
                    .or(act_inputs::source_base64.is_not_null()),
            )
            .load(&mut conn)?;

    let mut inputs = Vec::new();

    for (id, binary, base64, mime_type, input_type) in results {
        // Get the actual data
        let data = if let Some(bin) = binary {
            bin
        } else if let Some(b64) = base64 {
            // Decode base64
            use base64::{engine::general_purpose::STANDARD, Engine};
            match STANDARD.decode(&b64) {
                Ok(decoded) => decoded,
                Err(e) => {
                    tracing::warn!("Failed to decode base64 for input {}: {}", id, e);
                    continue;
                }
            }
        } else {
            continue;
        };

        inputs.push((id, data, mime_type, input_type));
    }

    Ok(inputs)
}

/// Migrate a single input to the new storage system.
async fn migrate_input(
    input_id: i32,
    data: &[u8],
    mime_type: Option<&str>,
    input_type_str: &str,
    repo: &PostgresNarrativeRepository,
) -> Result<bool, Box<dyn std::error::Error>> {
    use boticelli::database::schema::act_inputs;

    // Determine media type from input_type
    let media_type = match input_type_str.to_lowercase().as_str() {
        "image" => MediaType::Image,
        "audio" => MediaType::Audio,
        "video" => MediaType::Video,
        _ => {
            tracing::warn!(
                "Unknown input type '{}' for input {}, skipping",
                input_type_str,
                input_id
            );
            return Ok(false);
        }
    };

    // Create metadata
    let metadata = MediaMetadata {
        media_type,
        mime_type: mime_type.unwrap_or("application/octet-stream").to_string(),
        filename: None,
        width: None,
        height: None,
        duration_seconds: None,
    };

    // Store media
    let media_ref = repo.store_media(data, &metadata).await?;

    // Update act_inputs to reference the new media
    let mut conn = establish_connection()?;

    diesel::update(act_inputs::table.find(input_id))
        .set(act_inputs::media_ref_id.eq(media_ref.id))
        .execute(&mut conn)?;

    Ok(true)
}
