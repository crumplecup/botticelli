//! Narrative tool implementations.

use crate::{
    CreateNarrativeParams, CreateNarrativeResult, ModifyNarrativeParams, ModifyNarrativeResult,
    SaveNarrativeParams, SaveNarrativeResult,
};
use rmcp::tool;
use std::fs;
use std::path::Path;
use tracing::instrument;

/// Create a narrative from a natural language description.
///
/// Generates a TOML narrative file from a description using an LLM backend.
#[tool]
#[instrument(skip(_params), fields(name = %_params.name))]
pub async fn create_narrative(
    _params: CreateNarrativeParams,
) -> Result<CreateNarrativeResult, rmcp::ErrorData> {
    // TODO: Implement narrative creation using LLM backend
    // This requires:
    // 1. LLM backend integration (trait-based)
    // 2. Narrative generation prompt
    // 3. TOML parsing and validation
    // 4. Comment insertion

    tracing::warn!("create_narrative not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "create_narrative requires LLM backend integration",
        None,
    ))
}

/// Modify an existing narrative based on natural language instructions.
///
/// Takes existing TOML and modification instructions, uses LLM to apply changes.
#[tool]
#[instrument(skip(_params), fields(has_save_path = _params.save_to.is_some()))]
pub async fn modify_narrative(
    _params: ModifyNarrativeParams,
) -> Result<ModifyNarrativeResult, rmcp::ErrorData> {
    // TODO: Implement narrative modification using LLM backend
    // This requires:
    // 1. TOML parsing
    // 2. LLM backend for interpreting modifications
    // 3. TOML regeneration
    // 4. Validation
    // 5. Optional file save

    tracing::warn!("modify_narrative not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "modify_narrative requires LLM backend integration",
        None,
    ))
}

/// Save a narrative TOML to a file.
///
/// Persists narrative content to the filesystem with optional overwrite protection.
#[tool]
#[instrument(skip(params), fields(path = %params.file_path, overwrite = params.overwrite))]
pub async fn save_narrative(
    params: SaveNarrativeParams,
) -> Result<SaveNarrativeResult, rmcp::ErrorData> {
    tracing::debug!("Saving narrative to file");

    let path = Path::new(&params.file_path);

    // Check if file exists and overwrite is not allowed
    if path.exists() && !params.overwrite {
        tracing::warn!("File already exists and overwrite not allowed");
        return Err(rmcp::ErrorData::new(
            rmcp::model::ErrorCode::INVALID_PARAMS,
            format!(
                "File '{}' already exists. Set overwrite=true to replace it.",
                params.file_path
            ),
            None,
        ));
    }

    let overwritten = path.exists();

    // Write the file
    fs::write(path, &params.narrative_toml).map_err(|e| {
        tracing::error!(error = ?e, "Failed to write file");
        rmcp::ErrorData::new(
            rmcp::model::ErrorCode::INTERNAL_ERROR,
            format!("Failed to write file: {}", e),
            None,
        )
    })?;

    // Get the absolute path
    let abs_path = path.canonicalize().map_err(|e| {
        tracing::error!(error = ?e, "Failed to canonicalize path");
        rmcp::ErrorData::new(
            rmcp::model::ErrorCode::INTERNAL_ERROR,
            format!("Failed to get absolute path: {}", e),
            None,
        )
    })?;

    let size_bytes = params.narrative_toml.len();

    tracing::info!(
        path = %abs_path.display(),
        size = size_bytes,
        overwritten = overwritten,
        "Successfully saved narrative"
    );

    Ok(SaveNarrativeResult::new(
        abs_path.to_string_lossy().to_string(),
        size_bytes,
        overwritten,
    ))
}
