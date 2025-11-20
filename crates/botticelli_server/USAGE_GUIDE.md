# Botticelli Local Inference Server - Usage Guide

## Quick Start

The Botticelli server provides automatic model download and management for local LLM inference.

### 1. List Available Models

```bash
cargo run -p botticelli_server -- --list
```

This shows all supported models with their IDs, sizes, and descriptions.

### 2. Download and Prepare a Model

```bash
cargo run -p botticelli_server --model mistral-7b-q4 --download-dir ~/inference_models
```

This will:
- Check if the model is already downloaded
- If not, automatically download it from Hugging Face (~4GB for Q4 quantization)
- Save it to the specified directory
- Display the command to start the inference server

### 3. Start the Inference Server

After the model is downloaded, use the displayed `mistralrs-server` command:

```bash
mistralrs-server --port 8080 \
  gguf -m ~/inference_models \
  -f Mistral-7B-Instruct-v0.3.Q4_K_M.gguf \
  -t mistralai/Mistral-7B-Instruct-v0.3
```

### 4. Test the Server

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "default",
    "messages": [
      {"role": "user", "content": "Explain quantum computing in simple terms"}
    ],
    "max_tokens": 100
  }'
```

## Available Models

### Mistral 7B Instruct v0.3

Three quantization levels available:

| ID | Size | Quantization | Use Case |
|----|------|--------------|----------|
| `mistral-7b-q4` | ~4GB | Q4_K_M | Good balance of quality and speed |
| `mistral-7b-q5` | ~5GB | Q5_K_M | Better quality, slightly slower |
| `mistral-7b-q8` | ~7GB | Q8_0 | Highest quality, more memory |

## Command Line Options

```
botticelli_server [OPTIONS]

Options:
  -m, --model <MODEL>                Model to use (e.g., "mistral-7b-q4")
  -d, --download-dir <DOWNLOAD_DIR>  Directory to download/store models [default: ./models]
  -p, --port <PORT>                  Port to run the server on [default: 8080]
  -l, --list                         List available models and exit
  -h, --help                         Print help
  -V, --version                      Print version
```

## Model Storage

Models are stored in the directory specified by `--download-dir`:

```
~/inference_models/
  ├── Mistral-7B-Instruct-v0.3.Q4_K_M.gguf
  ├── Mistral-7B-Instruct-v0.3.Q5_K_M.gguf
  └── Mistral-7B-Instruct-v0.3.Q8_0.gguf
```

Once downloaded, models are reused across runs.

## System Requirements

### Minimum Requirements (Q4 quantization)
- RAM: 8GB minimum, 16GB recommended
- Disk: 5GB free space per model
- CPU: Modern x86_64 or ARM64 processor

### For Better Performance
- RAM: 16GB+ for Q5/Q8 quantizations
- GPU: Metal (macOS), CUDA (NVIDIA), or ROCm (AMD) support coming soon

## Troubleshooting

### Download Fails
- Check internet connection
- Verify Hugging Face is accessible
- Ensure sufficient disk space

### Server Won't Start
- Verify mistralrs-server is installed: `cargo install --git https://github.com/EricLBuehler/mistral.rs.git mistralrs-server`
- Check that the model file exists in the download directory
- Ensure the port is not already in use

### Nonsense Output
- This usually indicates wrong model format or corrupted download
- Delete the model file and re-download
- Verify you're using a GGUF format model (not safetensors or other formats)

## Next Steps

- Integration with Botticelli's text generation pipeline (coming soon)
- Direct library usage without separate server process (coming soon)
- GPU acceleration support (coming soon)
