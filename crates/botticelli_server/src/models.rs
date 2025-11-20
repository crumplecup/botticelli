//! Model catalog and automatic downloading

use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument};

/// Supported model configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelSpec {
    /// Mistral 7B Instruct v0.3 - Q4_K_M quantization (4GB)
    Mistral7BInstructV03Q4,
    /// Mistral 7B Instruct v0.3 - Q5_K_M quantization (5GB)
    Mistral7BInstructV03Q5,
    /// Mistral 7B Instruct v0.3 - Q8_0 quantization (7GB)
    Mistral7BInstructV03Q8,
}

impl ModelSpec {
    /// Get the Hugging Face repository ID
    pub fn repo_id(&self) -> &'static str {
        match self {
            ModelSpec::Mistral7BInstructV03Q4
            | ModelSpec::Mistral7BInstructV03Q5
            | ModelSpec::Mistral7BInstructV03Q8 => "MaziyarPanahi/Mistral-7B-Instruct-v0.3-GGUF",
        }
    }

    /// Get the GGUF filename within the repo
    pub fn filename(&self) -> &'static str {
        match self {
            ModelSpec::Mistral7BInstructV03Q4 => "Mistral-7B-Instruct-v0.3.Q4_K_M.gguf",
            ModelSpec::Mistral7BInstructV03Q5 => "Mistral-7B-Instruct-v0.3.Q5_K_M.gguf",
            ModelSpec::Mistral7BInstructV03Q8 => "Mistral-7B-Instruct-v0.3.Q8_0.gguf",
        }
    }

    /// Get the tokenizer model ID (may differ from model repo)
    pub fn tokenizer_id(&self) -> &'static str {
        match self {
            ModelSpec::Mistral7BInstructV03Q4
            | ModelSpec::Mistral7BInstructV03Q5
            | ModelSpec::Mistral7BInstructV03Q8 => "mistralai/Mistral-7B-Instruct-v0.3",
        }
    }

    /// Get approximate download size in GB
    pub fn size_gb(&self) -> u32 {
        match self {
            ModelSpec::Mistral7BInstructV03Q4 => 4,
            ModelSpec::Mistral7BInstructV03Q5 => 5,
            ModelSpec::Mistral7BInstructV03Q8 => 7,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ModelSpec::Mistral7BInstructV03Q4 => {
                "Mistral 7B Instruct v0.3 (Q4_K_M, ~4GB, good quality/speed balance)"
            }
            ModelSpec::Mistral7BInstructV03Q5 => {
                "Mistral 7B Instruct v0.3 (Q5_K_M, ~5GB, better quality)"
            }
            ModelSpec::Mistral7BInstructV03Q8 => {
                "Mistral 7B Instruct v0.3 (Q8_0, ~7GB, highest quality)"
            }
        }
    }

    /// Parse from a string identifier
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mistral-7b-q4" | "mistral-7b-instruct-v0.3-q4" => {
                Some(ModelSpec::Mistral7BInstructV03Q4)
            }
            "mistral-7b-q5" | "mistral-7b-instruct-v0.3-q5" => {
                Some(ModelSpec::Mistral7BInstructV03Q5)
            }
            "mistral-7b-q8" | "mistral-7b-instruct-v0.3-q8" => {
                Some(ModelSpec::Mistral7BInstructV03Q8)
            }
            _ => None,
        }
    }

    /// List all available models
    pub fn all() -> &'static [ModelSpec] {
        &[
            ModelSpec::Mistral7BInstructV03Q4,
            ModelSpec::Mistral7BInstructV03Q5,
            ModelSpec::Mistral7BInstructV03Q8,
        ]
    }
}

/// Model downloader and manager
pub struct ModelManager {
    download_dir: PathBuf,
}

impl ModelManager {
    /// Create a new model manager with the specified download directory
    #[instrument(skip(download_dir))]
    pub fn new(download_dir: impl AsRef<Path>) -> Self {
        let download_dir = download_dir.as_ref().to_path_buf();
        info!(path = ?download_dir, "Created model manager");
        Self { download_dir }
    }

    /// Check if a model is already downloaded
    #[instrument(skip(self))]
    pub fn is_downloaded(&self, spec: ModelSpec) -> bool {
        let path = self.model_path(spec);
        let exists = path.exists();
        debug!(model = ?spec, path = ?path, exists, "Checked if model is downloaded");
        exists
    }

    /// Get the local path where a model would be stored
    pub fn model_path(&self, spec: ModelSpec) -> PathBuf {
        self.download_dir.join(spec.filename())
    }

    /// Download a model from Hugging Face
    #[instrument(skip(self))]
    pub async fn download(&self, spec: ModelSpec) -> anyhow::Result<PathBuf> {
        info!(
            model = ?spec,
            size_gb = spec.size_gb(),
            "Starting model download"
        );

        // Create download directory if it doesn't exist
        tokio::fs::create_dir_all(&self.download_dir).await?;

        let api = hf_hub::api::tokio::Api::new()?;
        let repo = api.model(spec.repo_id().to_string());

        info!(
            repo = spec.repo_id(),
            filename = spec.filename(),
            "Downloading from Hugging Face"
        );

        let downloaded_path = repo.get(spec.filename()).await?;

        // Copy to our download directory
        let target_path = self.model_path(spec);
        tokio::fs::copy(&downloaded_path, &target_path).await?;

        info!(path = ?target_path, "Model download complete");

        Ok(target_path)
    }

    /// Ensure a model is available, downloading if necessary
    #[instrument(skip(self))]
    pub async fn ensure_model(&self, spec: ModelSpec) -> anyhow::Result<PathBuf> {
        if self.is_downloaded(spec) {
            info!(model = ?spec, "Model already downloaded");
            Ok(self.model_path(spec))
        } else {
            info!(model = ?spec, "Model not found locally, downloading");
            self.download(spec).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_spec_parsing() {
        assert_eq!(
            ModelSpec::parse("mistral-7b-q4"),
            Some(ModelSpec::Mistral7BInstructV03Q4)
        );
        assert_eq!(
            ModelSpec::parse("mistral-7b-instruct-v0.3-q5"),
            Some(ModelSpec::Mistral7BInstructV03Q5)
        );
        assert_eq!(ModelSpec::parse("unknown"), None);
    }

    #[test]
    fn test_model_spec_properties() {
        let spec = ModelSpec::Mistral7BInstructV03Q4;
        assert_eq!(spec.repo_id(), "MaziyarPanahi/Mistral-7B-Instruct-v0.3-GGUF");
        assert_eq!(spec.filename(), "Mistral-7B-Instruct-v0.3.Q4_K_M.gguf");
        assert_eq!(spec.tokenizer_id(), "mistralai/Mistral-7B-Instruct-v0.3");
        assert_eq!(spec.size_gb(), 4);
    }
}
