# Setting Up mistralrs-server for Botticelli

This guide explains how to install and configure the `mistralrs-server` binary from the [mistral.rs](https://github.com/EricLBuehler/mistral.rs) project.

## What is mistralrs-server?

`mistralrs-server` is a high-performance inference server that provides an OpenAI-compatible HTTP API for running local LLM models. Botticelli uses it as a backend for local inference.

## Installation

### Option 1: Build from Source (Recommended for GPU support)

1. **Clone the mistral.rs repository:**
```bash
cd ~/repos  # or wherever you keep your projects
git clone https://github.com/EricLBuehler/mistral.rs.git
cd mistral.rs
```

2. **Install with CPU support only:**
```bash
cargo install --path mistralrs-server
```

3. **OR install with NVIDIA GPU support (CUDA):**
```bash
# Make sure CUDA toolkit is installed first
sudo pacman -S cuda cuda-tools

# Build with CUDA and FlashAttention
cargo install --path mistralrs-server --features "cuda flash-attn"
```

4. **Verify installation:**
```bash
which mistralrs-server
mistralrs-server --help
```

### Option 2: Use Pre-built Binary (if available)

Check the [mistral.rs releases page](https://github.com/EricLBuehler/mistral.rs/releases) for pre-built binaries.

## Getting Models

### Recommended Models for Getting Started

**Small (Good for testing, 4-8GB RAM):**
- `microsoft/Phi-3-mini-128k-instruct` - Very capable, runs on CPU
- `HuggingFaceTB/SmolLM3-1.7B-Instruct` - Tiny but surprisingly good

**Medium (Better quality, 16-24GB RAM or 8GB+ VRAM):**
- `mistralai/Mistral-7B-Instruct-v0.3` - Excellent general purpose
- `HuggingFaceTB/SmolLM3-3B-Instruct` - Good balance

**Large (Best quality, 32GB+ RAM or 16GB+ VRAM):**
- `mistralai/Mixtral-8x7B-Instruct-v0.1` - Top tier quality

### Models are Downloaded Automatically!

Unlike the previous guide, **you don't need to manually download models**. The `mistralrs-server` will download them automatically from Hugging Face when you start it.

## Running the Server

### Basic Usage (CPU)

```bash
# Smallest model (good for testing)
mistralrs-server --port 8080 plain -m HuggingFaceTB/SmolLM3-1.7B-Instruct

# Medium quality model
mistralrs-server --port 8080 plain -m mistralai/Mistral-7B-Instruct-v0.3
```

### With GPU (CUDA)

#### Using Local SafeTensors Model

If you've already downloaded a model locally:

```bash
# Use locally downloaded SafeTensors model
mistralrs-server --port 8080 plain \
  -m /home/erik/repos/inference_models/Mistral-7B-Instruct-v0.2 \
  -a mistral

# With quantization to reduce VRAM usage (8-bit)
mistralrs-server --port 8080 --isq Q8_0 plain \
  -m /home/erik/repos/inference_models/Mistral-7B-Instruct-v0.2 \
  -a mistral

# With quantization (4-bit, even less VRAM)
mistralrs-server --port 8080 --isq Q4_K_M plain \
  -m /home/erik/repos/inference_models/Mistral-7B-Instruct-v0.2 \
  -a mistral
```

#### Using Hugging Face Model IDs (Auto-download)

```bash
# Let it auto-detect GPU
mistralrs-server --port 8080 plain -m mistralai/Mistral-7B-Instruct-v0.3

# With quantization to reduce VRAM usage (8-bit)
mistralrs-server --port 8080 --isq Q8_0 plain -m mistralai/Mistral-7B-Instruct-v0.3

# With quantization (4-bit, even less VRAM)
mistralrs-server --port 8080 --isq Q4_K_M plain -m mistralai/Mistral-7B-Instruct-v0.3
```

### Interactive Mode (for testing)

```bash
# Test the model in your terminal before using with Botticelli
mistralrs-server -i plain -m HuggingFaceTB/SmolLM3-1.7B-Instruct
```

### Important Notes

1. **First run will be slow** - The model needs to download (can be several GB)
2. **Models are cached** - Subsequent runs will be much faster
3. **Default cache location**: `~/.cache/huggingface/hub/`
4. **Check VRAM usage**: Use `nvidia-smi` to monitor GPU memory

## Testing the Server

Once the server is running, test it with curl:

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "mistralai/Mistral-7B-Instruct-v0.3",
    "messages": [
      {"role": "user", "content": "Hello! How are you?"}
    ],
    "max_tokens": 100
  }'
```

## Troubleshooting

### CUDA not found

```bash
# Verify CUDA installation
nvidia-smi
nvcc --version

# Add to ~/.bashrc or ~/.zshrc
export PATH=/opt/cuda/bin:$PATH
export LD_LIBRARY_PATH=/opt/cuda/lib64:$LD_LIBRARY_PATH
```

### Out of memory errors

- Use a smaller model (e.g., SmolLM3-1.7B instead of Mistral-7B)
- Add quantization: `--isq Q4_K_M` or `--isq Q3_K_M`
- Close other GPU-using applications

### Model downloads are slow

- Consider using a VPN if Hugging Face is slow in your region
- Manually download large models with `huggingface-cli`:

```bash
pip install huggingface-hub
huggingface-cli download mistralai/Mistral-7B-Instruct-v0.3
```

### Port already in use

```bash
# Use a different port
mistralrs-server --port 8081 plain -m HuggingFaceTB/SmolLM3-1.7B-Instruct
```

## Using with Botticelli

Once your `mistralrs-server` is running, configure Botticelli to connect to it:

```toml
# In your Botticelli config
[inference]
backend = "local"
api_url = "http://localhost:8080"
model = "mistralai/Mistral-7B-Instruct-v0.3"  # Match the model you started
```

## Performance Tips

1. **Use quantization** to reduce memory usage: `--isq Q4_K_M`
2. **Use FlashAttention** if you built with `--features flash-attn`
3. **Monitor GPU usage** with `nvidia-smi` to optimize batch sizes
4. **Use interactive mode** (`-i`) first to test before integrating
5. **Cache models** - First run is slow, subsequent runs are fast

## Further Reading

- [mistral.rs documentation](https://github.com/EricLBuehler/mistral.rs)
- [Hugging Face model hub](https://huggingface.co/models)
- [Quantization guide](https://github.com/EricLBuehler/mistral.rs/blob/master/docs/QUANTS.md)
