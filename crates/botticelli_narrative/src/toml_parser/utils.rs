//! Utility functions for TOML parsing.

use botticelli_core::HistoryRetention;
use botticelli_error::{IoError, NarrativeError, NarrativeErrorKind, NarrativeResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, instrument, warn};

/// Recursively search for a file starting from a base directory.
///
/// Searches upward through parent directories until the file is found or root is reached.
/// Then searches downward recursively from the highest found directory.
///
/// # Arguments
///
/// * `filename` - The filename to search for (without path, e.g., "BOTTICELLI_CONTEXT.md")
/// * `start_dir` - Optional starting directory (defaults to current directory)
/// * `context_path` - Optional context base path from configuration
///
/// # Returns
///
/// The full path to the file if found, or an error if not found.
#[instrument(skip_all, fields(%filename))]
pub(super) fn find_file_recursive(
    filename: &str,
    start_dir: Option<&Path>,
    context_path: Option<&str>,
) -> NarrativeResult<PathBuf> {
    // Determine search start directory
    let base_dir = if let Some(ctx_path) = context_path {
        PathBuf::from(ctx_path)
    } else if let Some(start) = start_dir {
        start.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::Io(IoError::from(e)))
        })?
    };

    debug!(search_base = %base_dir.display(), "Starting file search");

    // First try exact match in base directory
    let direct_path = base_dir.join(filename);
    if direct_path.exists() {
        debug!(found = %direct_path.display(), "Found file directly");
        return Ok(direct_path);
    }

    // Search upward to find project root or file
    let mut current = base_dir.as_path();
    let mut search_roots = vec![current.to_path_buf()];

    while let Some(parent) = current.parent() {
        let candidate = parent.join(filename);
        if candidate.exists() {
            debug!(found = %candidate.display(), "Found file in parent directory");
            return Ok(candidate);
        }

        // Check for workspace markers (stop at project root)
        if parent.join("Cargo.toml").exists() || parent.join(".git").exists() {
            search_roots.push(parent.to_path_buf());
            break;
        }

        search_roots.push(parent.to_path_buf());
        current = parent;
    }

    // Search downward recursively from collected roots
    for root in search_roots.iter().rev() {
        if let Ok(found) = search_directory_recursive(root, filename) {
            debug!(found = %found.display(), "Found file via recursive search");
            return Ok(found);
        }
    }

    error!("File not found after exhaustive search");
    Err(NarrativeErrorKind::FileRead(format!(
        "File '{}' not found in {} or any parent/child directories",
        filename,
        base_dir.display()
    ))
    .into())
}

/// Recursively search a directory tree for a file.
#[instrument(skip_all, fields(%filename, dir = %dir.display()))]
fn search_directory_recursive(dir: &Path, filename: &str) -> Result<PathBuf, ()> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Check if this is the file we're looking for
            if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(filename) {
                debug!(found = %path.display(), "Found file in directory");
                return Ok(path);
            }

            // Recurse into subdirectories (skip hidden dirs and common ignore patterns)
            if path.is_dir()
                && let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
                && !dir_name.starts_with('.')
                && dir_name != "target"
                && dir_name != "node_modules"
                && let Ok(found) = search_directory_recursive(&path, filename)
            {
                return Ok(found);
            }
        }
    }

    Err(())
}

/// Expand environment variables in string values within a HashMap.
///
/// Supports `${VAR_NAME}` and `$VAR_NAME` syntax.
#[instrument(skip_all, fields(arg_count = args.len()))]
pub(super) fn expand_env_vars(args: &HashMap<String, Value>) -> HashMap<String, Value> {
    debug!("Expanding environment variables in arguments");
    args.iter()
        .map(|(k, v)| {
            let expanded_value = match v {
                Value::String(s) => match shellexpand::env(s) {
                    Ok(expanded) => {
                        if expanded.as_ref() != s {
                            debug!(key = %k, original = %s, expanded = %expanded, "Expanded environment variable");
                        }
                        Value::String(expanded.into_owned())
                    }
                    Err(_) => v.clone(), // Keep original if expansion fails
                },
                _ => v.clone(),
            };
            (k.clone(), expanded_value)
        })
        .collect()
}

/// Parse history retention string to HistoryRetention enum.
///
/// Accepts: "full", "summary", "drop"
/// Returns HistoryRetention::Full if None or invalid value.
#[instrument(skip_all, fields(value = ?value))]
pub(super) fn parse_history_retention(value: Option<&String>) -> HistoryRetention {
    let result = match value.map(|s| s.as_str()) {
        Some("full") => HistoryRetention::Full,
        Some("summary") => HistoryRetention::Summary,
        Some("drop") => HistoryRetention::Drop,
        Some(invalid) => {
            warn!(
                value = invalid,
                "Invalid history_retention value, defaulting to 'full'"
            );
            HistoryRetention::Full
        }
        None => HistoryRetention::Full,
    };
    debug!(result = ?result, "Parsed history retention");
    result
}

/// Check if a string is a resource reference (bots.name, tables.name, media.name, narratives.name, narrative:name).
#[instrument(skip_all, fields(%s))]
pub(super) fn is_reference(s: &str) -> bool {
    let result = s.starts_with("bots.")
        || s.starts_with("tables.")
        || s.starts_with("media.")
        || s.starts_with("narratives.")
        || s.starts_with("narrative:");
    debug!(is_ref = result, "Checked if string is reference");
    result
}

/// Infer MIME type from file extension.
#[instrument(skip_all, fields(%path))]
pub(super) fn infer_mime_type(path: &str) -> Option<String> {
    let extension = std::path::Path::new(path)
        .extension()?
        .to_str()?
        .to_lowercase();

    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mp3",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        _ => {
            debug!(extension = %extension, "Unknown file extension, cannot infer MIME");
            return None;
        }
    };

    debug!(mime = %mime, "Inferred MIME type");
    Some(mime.to_string())
}

/// Infer media type category from extension.
#[instrument(skip_all, fields(%path))]
pub(super) fn infer_media_type_from_extension(path: &str) -> NarrativeResult<&'static str> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            error!("Cannot determine file extension");
            NarrativeErrorKind::InvalidFieldValue {
                value: path.to_string(),
                field: "file".to_string(),
                reason: "cannot determine file extension".to_string(),
            }
        })?
        .to_lowercase();

    let media_type = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
        "mp3" | "wav" | "ogg" => "audio",
        "mp4" | "avi" | "mov" | "webm" => "video",
        "pdf" | "txt" | "md" | "json" => "document",
        _ => {
            error!(extension = %extension, "Unsupported file extension");
            return Err(NarrativeErrorKind::InvalidFieldValue {
                value: extension,
                field: "file extension".to_string(),
                reason: "unsupported file type".to_string(),
            }
            .into());
        }
    };

    debug!(media_type, "Inferred media type category");
    Ok(media_type)
}
