// Narrative discovery and metadata extraction

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use botticelli_narrative::{Narrative, NarrativeMetadata};
use tracing::{debug, instrument, warn};

use super::{NarrativeEntry, NarrativeMetadata as EntryMetadata};

/// Discovers narratives in a directory tree
#[instrument(skip_all, fields(path = %root.as_ref().display()))]
pub async fn discover_narratives<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<NarrativeEntry>, DiscoveryError> {
    let root = root.as_ref();

    if !root.exists() {
        debug!("Narratives directory does not exist, returning empty list");
        return Ok(Vec::new());
    }

    let mut narratives = Vec::new();

    // Use walkdir to traverse directory tree
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories and non-TOML files
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        match load_narrative_entry(path, root).await {
            Ok(entry) => {
                debug!(path = %path.display(), name = %entry.name, "Discovered narrative");
                narratives.push(entry);
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to load narrative");
            }
        }
    }

    debug!(count = narratives.len(), "Discovered narratives");
    Ok(narratives)
}

/// Loads a single narrative entry
async fn load_narrative_entry(path: &Path, root: &Path) -> Result<NarrativeEntry, DiscoveryError> {
    // Load the narrative to extract metadata
    let narrative = Narrative::from_file(path)
        .map_err(|e| DiscoveryError::LoadFailed(path.to_path_buf(), e.to_string()))?;

    // Extract metadata
    let metadata = extract_metadata(narrative.metadata());

    // Get file metadata
    let file_metadata = std::fs::metadata(path)
        .map_err(|e| DiscoveryError::MetadataFailed(path.to_path_buf(), e.to_string()))?;

    let last_modified = file_metadata
        .modified()
        .unwrap_or_else(|_| SystemTime::now());

    // Determine category from path relative to root
    let category = infer_category(path, root);

    // Get file name without extension
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(NarrativeEntry {
        path: path.to_path_buf(),
        name,
        metadata,
        category,
        last_modified,
    })
}

/// Extracts our simplified metadata from the full NarrativeMetadata
fn extract_metadata(narrative_meta: &NarrativeMetadata) -> EntryMetadata {
    EntryMetadata {
        name: narrative_meta.name().clone(),
        model: narrative_meta.model().clone(),
        act_count: 0, // We'll need to count acts from toc if needed
        description: narrative_meta.description().clone(),
    }
}

/// Infers category from file path relative to root
fn infer_category(path: &Path, root: &Path) -> String {
    // Get parent directory relative to root
    if let Ok(relative) = path.strip_prefix(root) {
        if let Some(parent) = relative.parent() {
            if parent == Path::new("") {
                return "Uncategorized".to_string();
            }

            // Use first directory component as category
            if let Some(first_component) = parent.components().next() {
                return first_component
                    .as_os_str()
                    .to_str()
                    .unwrap_or("Uncategorized")
                    .to_string();
            }
        }
    }

    "Uncategorized".to_string()
}

/// Errors that can occur during narrative discovery
#[derive(Debug, Clone, derive_more::Display)]
pub enum DiscoveryError {
    #[display("Failed to load narrative from {}: {}", _0.display(), _1)]
    LoadFailed(PathBuf, String),

    #[display("Failed to read file metadata for {}: {}", _0.display(), _1)]
    MetadataFailed(PathBuf, String),

    #[display("I/O error: {}", _0)]
    IoError(String),
}

impl std::error::Error for DiscoveryError {}

impl From<std::io::Error> for DiscoveryError {
    fn from(e: std::io::Error) -> Self {
        DiscoveryError::IoError(e.to_string())
    }
}
