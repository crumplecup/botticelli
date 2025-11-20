# Botticelli Local Inference Server Guide

A beginner-friendly guide to running your own local AI inference server using Botticelli.

## What is This?

This server lets you run AI models locally on your own hardware instead of using cloud APIs like Gemini. Benefits include:

- **Privacy**: Your data never leaves your machine
- **Cost**: No API fees after initial setup
- **Offline**: Works without internet connection
- **Control**: Full control over model selection and parameters

## Prerequisites

### Hardware Requirements

**Minimum:**
- CPU: Modern multi-core processor (4+ cores recommended)
- RAM: 16GB (8GB might work for small models)
- Disk: 10-50GB free space (varies by model size)

**Recommended:**
- GPU: NVIDIA GPU with 8GB+ VRAM (significantly faster inference)
- RAM: 32GB+
- Disk: 100GB+ for multiple models

### Software Requirements (Manjaro Linux)

1. **Install Rust** (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. **Install CUDA Toolkit** (for GPU acceleration - optional but recommended):
```bash
# Check if you have an NVIDIA GPU
lspci | grep -i nvidia

# Install CUDA via Manjaro repos
sudo pacman -S cuda cuda-tools

# Add CUDA to your PATH (add to ~/.bashrc or ~/.zshrc)
export PATH=/opt/cuda/bin:$PATH
export LD_LIBRARY_PATH=/opt/cuda/lib64:$LD_LIBRARY_PATH
```

3. **Install Git LFS** (for downloading large model files):
```bash
sudo pacman -S git-lfs
git lfs install
```

## Step 1: Choose a Model

Models are automatically downloaded from [Hugging Face](https://huggingface.co) when you start the server.

### Recommended Starting Models

**Small Models (4-8GB RAM or 2-4GB VRAM):**
- `HuggingFaceTB/SmolLM3-1.7B-Instruct` - Great for testing
- `microsoft/Phi-3-mini-128k-instruct` - Compact but capable

**Medium Models (16-24GB RAM or 8GB+ VRAM):**
- `mistralai/Mistral-7B-Instruct-v0.2` - Excellent general purpose (recommended)
- `mistralai/Mistral-7B-Instruct-v0.3` - Latest version
- `HuggingFaceTB/SmolLM3-3B-Instruct` - Good balance

**Large Models (32GB+ RAM or 16GB+ VRAM):**
- `mistralai/Mixtral-8x7B-Instruct-v0.1` - Top tier quality

### Memory Requirements

You can reduce memory usage with quantization:
- **No quantization**: Full quality, most memory
- **8-bit (`--isq Q8_0`)**: ~50% memory, minimal quality loss
- **4-bit (`--isq Q4_K_M`)**: ~25% memory, slight quality loss
- **3-bit (`--isq Q3_K_M`)**: ~20% memory, noticeable quality loss

## Step 2: Install mistralrs-server

See [MISTRALRS_SETUP.md](./MISTRALRS_SETUP.md) for complete installation instructions, or quick install:

```bash
# Clone mistral.rs
cd ~/repos
git clone https://github.com/EricLBuehler/mistral.rs.git
cd mistral.rs

# Install with GPU support (Manjaro/Arch)
cargo install --path mistralrs-server --features "cuda flash-attn"

# OR install CPU-only
cargo install --path mistralrs-server
```

## Step 3: Install and Run mistralrs-server

**Important:** See [MISTRALRS_SETUP.md](./MISTRALRS_SETUP.md) for complete installation instructions.

### Quick Start

1. **Install mistralrs-server:**
```bash
# CPU only
cargo install --path ~/repos/mistral.rs/mistralrs-server

# OR with GPU support (recommended)
cargo install --path ~/repos/mistral.rs/mistralrs-server --features "cuda flash-attn"
```

2. **Download a GGUF model** (required for local files):
```bash
# Create models directory
mkdir -p ~/repos/inference_models
cd ~/repos/inference_models

# Download GGUF model (Q4_K_M quantization - good balance, ~4.4GB)
wget https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/resolve/main/mistral-7b-instruct-v0.2.Q4_K_M.gguf
```

3. **Run the server**:

If you have safetensors format (downloaded from HuggingFace with git clone):
```bash
mistralrs-server --port 8080 plain \
  -m /home/erik/repos/inference_models/Mistral-7B-Instruct-v0.2 \
  -a mistral
```

If you downloaded GGUF format:
```bash
# Option 1: Use HuggingFace repo for both model and tokenizer (easiest)
mistralrs-server --port 8080 gguf \
  -m TheBloke/Mistral-7B-Instruct-v0.2-GGUF \
  -f mistral-7b-instruct-v0.2.Q4_K_M.gguf \
  -t mistralai/Mistral-7B-Instruct-v0.2

# Option 2: Use local GGUF with HuggingFace tokenizer
mistralrs-server --port 8080 gguf \
  -m /home/erik/repos/inference_models \
  -f mistral-7b-instruct-v0.2.Q4_K_M.gguf \
  -t mistralai/Mistral-7B-Instruct-v0.2
```

**Important:** 
- `plain` command works with safetensors format (what you get from `git clone`)
- GGUF models require three arguments: `-m` (model path/repo), `-f` (filename), `-t` (tokenizer)
- Use `-a mistral` to specify the model architecture for plain format

You should see output like:
```
Downloading model from HuggingFace...
Loading model...
Server listening on http://127.0.0.1:8080
```

## Step 4: Test the Server

In another terminal:

```bash
# Simple test request (OpenAI-compatible API)
# Use "default" as the model name - mistral.rs will use whatever model you loaded
# Note: temperature is NOT YET WORKING - mistral.rs has a bug with this parameter
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

**Important Notes:**
- The `"model"` field in the request should be `"default"` to use whatever model the server loaded
- Temperature parameter currently causes errors in mistral.rs - omit it for now

## Step 5: Integrate with Botticelli

Update your `botticelli.toml` configuration:

```toml
[inference]
backend = "local"
api_url = "http://localhost:8080"
model = "mistralai/Mistral-7B-Instruct-v0.2"  # Must match the model you started

# No API key needed for local server!
```

## Troubleshooting

### "Out of Memory" Errors

**Problem:** Model is too large for your GPU/RAM

**Solutions:**
1. Try a smaller model (e.g., Phi-2 instead of Mixtral)
2. Use CPU mode: Set `device = "cpu"` in config
3. Add quantization (4-bit or 8-bit) - reduces memory but slightly impacts quality

### Slow Inference on CPU

**Problem:** Generation takes minutes instead of seconds

**Solutions:**
1. Use a GPU if available
2. Reduce `max_tokens` to generate shorter responses
3. Try a smaller model
4. Increase `num_threads` to match your CPU cores

### CUDA Not Found

**Problem:** Server doesn't detect NVIDIA GPU

**Solutions:**
```bash
# Verify CUDA installation
nvidia-smi

# Check CUDA version
nvcc --version

# Reinstall CUDA if needed
sudo pacman -S cuda cuda-tools

# Ensure PATH is set
export PATH=/opt/cuda/bin:$PATH
export LD_LIBRARY_PATH=/opt/cuda/lib64:$LD_LIBRARY_PATH
```

### Model Download Failed

**Problem:** Git LFS files not downloading

**Solutions:**
```bash
# Ensure git-lfs is installed
git lfs install

# Try downloading again
cd ~/botticelli-models
rm -rf Mistral-7B-Instruct-v0.2
git lfs clone https://huggingface.co/mistralai/Mistral-7B-Instruct-v0.2
```

## Performance Tips

### For NVIDIA GPUs

1. **Use CUDA 12+** for best performance
2. **Enable tensor cores** (automatic on Volta and newer)
3. **Monitor GPU usage**: `watch -n 1 nvidia-smi`

### For CPU

1. **Set optimal thread count**: Match your physical cores
2. **Use quantized models**: 4-bit or 8-bit versions
3. **Close other applications** to free RAM

### General

1. **Keep model in fast storage**: SSD preferred over HDD
2. **Use smaller context windows**: Reduces memory usage
3. **Batch requests**: Process multiple prompts together when possible

## Model Selection Guide

### By Use Case

**Creative Writing:**
- `mistralai/Mistral-7B-Instruct-v0.2`
- Temperature: 0.8-1.0

**Code Generation:**
- `codellama/CodeLlama-7b-Instruct-hf`
- Temperature: 0.2-0.5

**Question Answering:**
- `mistralai/Mixtral-8x7B-Instruct-v0.1` (if you have resources)
- Temperature: 0.3-0.7

**Chat/Conversation:**
- `microsoft/phi-2` (lightweight)
- `mistralai/Mistral-7B-Instruct-v0.2` (better quality)

### By Hardware

**8GB RAM / No GPU:**
- `microsoft/phi-2` (2.7B parameters)

**16GB RAM / No GPU:**
- `mistralai/Mistral-7B-Instruct-v0.2` (7B parameters)

**16GB RAM / 8GB VRAM:**
- `mistralai/Mistral-7B-Instruct-v0.2`
- `meta-llama/Llama-2-7b-chat-hf`

**32GB+ RAM / 12GB+ VRAM:**
- `mistralai/Mixtral-8x7B-Instruct-v0.1`
- Any 13B parameter model

## Running as a System Service

To have the server start automatically on boot:

```bash
# Create systemd service file
sudo nano /etc/systemd/system/botticelli-server.service
```

Add:
```ini
[Unit]
Description=Botticelli Local Inference Server
After=network.target

[Service]
Type=simple
User=erik
WorkingDirectory=/home/erik/repos/botticelli
ExecStart=/home/erik/repos/botticelli/target/release/botticelli_server --config /home/erik/server_config.toml
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable botticelli-server
sudo systemctl start botticelli-server

# Check status
sudo systemctl status botticelli-server

# View logs
journalctl -u botticelli-server -f
```

## Security Considerations

### Local Network Access

By default, the server only accepts connections from localhost (`127.0.0.1`). To allow access from other devices on your network:

```toml
[server]
host = "0.0.0.0"  # Listen on all interfaces
port = 8080
```

**Warning:** Only do this on trusted networks! Anyone on your network can access the server.

### Adding Authentication (Recommended for network access)

```toml
[server]
host = "0.0.0.0"
port = 8080
api_key = "your-secret-key-here"  # Generate a strong random key
```

Then include in requests:
```bash
curl -X POST http://192.168.1.100:8080/v1/generate \
  -H "Authorization: Bearer your-secret-key-here" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello world"}'
```

## Next Steps

1. **Experiment with different models** to find the best fit for your hardware and use case
2. **Tune generation parameters** (temperature, top_p, max_tokens) for your needs
3. **Monitor resource usage** and optimize based on your bottlenecks
4. **Integrate with Botticelli workflows** for narrative generation, content creation, etc.

## Additional Resources

- [Hugging Face Model Hub](https://huggingface.co/models)
- [mistral.rs Documentation](https://github.com/EricLBuehler/mistral.rs)
- [CUDA Installation Guide](https://docs.nvidia.com/cuda/cuda-installation-guide-linux/)
- [Model Quantization Guide](https://huggingface.co/docs/transformers/main/en/quantization)

## Getting Help

If you encounter issues:

1. Check the troubleshooting section above
2. Review server logs for error messages
3. Verify your hardware meets minimum requirements
4. Ensure all dependencies are correctly installed
5. Open an issue on the Botticelli GitHub repository with:
   - Your hardware specs (CPU, RAM, GPU)
   - OS version (Manjaro version, kernel)
   - Model you're trying to run
   - Full error message or logs
