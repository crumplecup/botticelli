# Automated Model Download Plan

## Status: ✅ Phase 1-3 Complete

The basic implementation of automated model download is complete. Users can now:
- List available models with `--list`
- Automatically download models by specifying `--model <id>`
- Models are cached and reused across runs

See [USAGE_GUIDE.md](./USAGE_GUIDE.md) for complete usage instructions.

## Problem
Users shouldn't need to manually find, download, and configure GGUF models. The current setup is error-prone and requires knowledge of model formats, quantization levels, and directory structures.

## Solution
Automate model selection and download based on user preferences (model size, quantization level, use case).

## Implementation Phases

### Phase 1: Model Registry ✅ COMPLETE
Create a built-in registry of recommended models with metadata:

```rust
pub struct ModelSpec {
    pub id: String,                    // "mistral-7b-instruct-v0.2"
    pub hf_repo: String,               // "TheBloke/Mistral-7B-Instruct-v0.2-GGUF"
    pub filename: String,              // "mistral-7b-instruct-v0.2.Q4_K_M.gguf"
    pub quantization: Quantization,    // Q4_K_M, Q5_K_M, Q8_0, etc.
    pub size_gb: f32,                  // Approximate download size
    pub ram_gb: f32,                   // Minimum RAM required
    pub tokenizer_repo: String,        // "mistralai/Mistral-7B-Instruct-v0.2"
    pub description: String,
    pub use_cases: Vec<String>,
}

pub enum Quantization {
    Q4_K_M,  // 4-bit, medium quality, smallest
    Q5_K_M,  // 5-bit, better quality
    Q8_0,    // 8-bit, highest quality
}
```

Built-in registry:
- Mistral 7B Instruct (various quantizations)
- Llama 2/3 models
- Phi-3 models
- Gemma models

**Implementation:** `ModelSpec` enum with three Mistral 7B v0.3 variants (Q4, Q5, Q8) implemented in `src/models.rs`.

### Phase 2: Download Manager ✅ COMPLETE

```rust
use hf_hub::api::sync::Api;

pub struct ModelDownloader {
    cache_dir: PathBuf,
    api: Api,
}

impl ModelDownloader {
    pub fn download_model(&self, spec: &ModelSpec) -> Result<PathBuf, ServerError> {
        // Download from HF Hub
        // Show progress bar
        // Verify checksum
        // Return path to downloaded file
    }
}
```

**Implementation:** `ModelManager` struct with `download()`, `is_downloaded()`, and `ensure_model()` methods using `hf-hub` crate.

### Phase 3: CLI Interface ✅ COMPLETE

```bash
# List available models
botticelli_server models list

# Download a specific model
botticelli_server models download mistral-7b-q4

# Start server with auto-download if needed
botticelli_server start --model mistral-7b-q4 --port 8080

# Interactive model selection
botticelli_server models select
```

**Implementation:** CLI with `--list`, `--model`, `--download-dir`, and `--port` flags. Automatic download on first use.

### Phase 4: Configuration - NOT YET IMPLEMENTED
Update server config to support model specs:

```toml
[model]
# Use a built-in model spec
spec = "mistral-7b-q4"

# Or specify custom HF model
# hf_repo = "TheBloke/custom-model-GGUF"
# filename = "model.Q4_K_M.gguf"
# tokenizer_repo = "org/tokenizer-repo"

# Cache directory for downloaded models
cache_dir = "~/.cache/botticelli/models"

[server]
port = 8080
host = "127.0.0.1"
```

### Phase 5: Smart Defaults - NOT YET IMPLEMENTED
- Detect system RAM and recommend appropriate quantization
- Suggest models based on available disk space
- Warn if insufficient resources for selected model

### Phase 6: Actual Server Integration - NOT YET IMPLEMENTED
Currently, the tool downloads models and displays the `mistralrs-server` command to run. Future work:
- Embed mistralrs directly and start server programmatically
- No need for separate `mistralrs-server` binary
- Full integration with Botticelli's generation pipeline

## Dependencies to Add
```toml
hf-hub = "0.3"           # Hugging Face Hub API
indicatif = "0.17"       # Progress bars
byte-unit = "5"          # Size formatting
sysinfo = "0.30"         # System resource detection
```

## User Experience

**First Time Setup:**
```bash
$ botticelli_server start
No model configured. Would you like to select one? [Y/n] y

Available models:
1. Mistral 7B Instruct Q4 (3.8 GB, 6 GB RAM) - Fast, good quality
2. Mistral 7B Instruct Q5 (4.8 GB, 7 GB RAM) - Better quality
3. Mistral 7B Instruct Q8 (7.2 GB, 10 GB RAM) - Best quality
4. Phi-3 Mini Q4 (2.3 GB, 4 GB RAM) - Smallest, fastest

Your system: 16 GB RAM, 50 GB free disk space
Recommended: Option 2 (Mistral 7B Instruct Q5)

Select model [1-4]: 2
Downloading Mistral 7B Instruct Q5...
[████████████████████] 4.8 GB / 4.8 GB (2 min remaining)

Model downloaded successfully!
Starting server on http://localhost:8080...
```

**Subsequent Runs:**
```bash
$ botticelli_server start
Using cached model: Mistral 7B Instruct Q5
Starting server on http://localhost:8080...
```

## Benefits
1. **Zero manual configuration** - Just run and go
2. **Smart defaults** - System-aware recommendations
3. **Reproducible** - Config files specify exact models
4. **Offline-capable** - Once downloaded, works offline
5. **Multiple models** - Easy to download and switch between models

## Next Steps
1. Implement ModelSpec registry with 3-4 popular models
2. Add hf-hub integration for downloads
3. Create CLI commands for model management
4. Add interactive model selection
5. Update SERVER_GUIDE.md with new workflow
