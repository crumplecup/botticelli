# Boticelli

A unified Rust library and CLI for executing multi-act LLM narratives with support for multiple backends (Gemini, Anthropic, etc.) and optional PostgreSQL persistence.

## Overview

Boticelli enables you to define complex, multi-step LLM workflows in TOML files called "narratives." Each narrative consists of multiple "acts" that execute sequentially, with each act seeing the outputs from previous acts as context. This enables powerful workflows like:

- Generate content → Critique → Improve
- Analyze image → Summarize → Translate
- Research topic → Draft outline → Write sections
- Transcribe audio → Summarize → Extract action items

## Features

- 🎭 **Multi-Act Narratives**: Define sequential LLM workflows in TOML
- 🔄 **Context Passing**: Each act sees all previous outputs
- 🎨 **Multimodal Support**: Text, images, audio, video, and documents
- 🔌 **Multiple Backends**: Gemini (more coming soon)
- ⚙️ **Per-Act Configuration**: Different models, temperature, max_tokens per act
- 💾 **Optional Persistence**: Store executions in PostgreSQL
- 🖥️ **CLI Interface**: Easy command-line execution
- 🦀 **Type-Safe**: Full Rust type safety throughout

## Quick Start

### Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **PostgreSQL** 14+ (optional, only if using `--save` flag)

### Installation

```bash
# Clone the repository
git clone https://github.com/crumplecup/boticelli.git
cd boticelli

# Build the project
cargo build --release

# The binary will be at ./target/release/boticelli
```

### Basic Configuration

1. **Create a `.env` file** in the project root:

```bash
cp .env.example .env
```

2. **Add your API key(s)**:

```env
# Required for Gemini backend
GEMINI_API_KEY=your_gemini_api_key_here

# Optional: For logging
RUST_LOG=info

# Optional: Only needed if using --save flag (component-based)
DATABASE_USER=boticelli_user
DATABASE_PASSWORD=your_password
DATABASE_NAME=boticelli  # Optional: defaults to boticelli
```

3. **Get an API key**:
   - **Gemini**: Visit [Google AI Studio](https://makersuite.google.com/app/apikey)

### Run Your First Narrative

```bash
# Set your API key (if not in .env)
export GEMINI_API_KEY="your-key-here"

# Run the example narrative
./target/release/boticelli run --narrative narrations/mint.toml --verbose
```

You should see output like:

```
📖 Loading narrative from "narrations/mint.toml"...
✓ Loaded: Social Media Post Generation
  Description: A three-act narrative for generating engaging social media content
  Acts: 3

🚀 Executing narrative...

Executing 3 acts in sequence:

  ✓ Act 1/3: brainstorm (245 chars)
  ✓ Act 2/3: draft (512 chars)
  ✓ Act 3/3: refine (498 chars)

✓ Execution completed in 12.34s
  Total acts: 3

📊 Results:

  Act 1: brainstorm
    Response: Here are 5 ideas for social media posts about Rust programming:
    1. "Why Rust's borrow checker...

  Act 2: draft
    Response: 🦀 Ever wondered why Rust is taking the programming world by storm?...

  Act 3: refine
    Response: 🦀 Why Rust is revolutionizing systems programming:...
```

## Database Setup (Optional)

If you want to save execution history with the `--save` flag, you'll need PostgreSQL:

### 1. Install PostgreSQL

**Ubuntu/Debian:**
```bash
sudo apt-get install postgresql postgresql-contrib
```

**macOS:**
```bash
brew install postgresql@14
brew services start postgresql@14
```

**Windows:**
Download from [postgresql.org](https://www.postgresql.org/download/windows/)

### 2. Create Database

```bash
# Connect to PostgreSQL
psql postgres

# Create database and user
CREATE DATABASE boticelli;
CREATE USER boticelli_user WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE boticelli TO boticelli_user;
\q
```

### 3. Configure Database Connection

Add to your `.env` file (component-based approach recommended):

```env
# Option 1: Component-based (recommended)
DATABASE_USER=boticelli_user
DATABASE_PASSWORD=your_password
DATABASE_HOST=localhost      # Optional: defaults to localhost
DATABASE_PORT=5432            # Optional: defaults to 5432
DATABASE_NAME=boticelli       # Optional: defaults to boticelli

# Option 2: Complete URL (alternative - takes precedence)
# DATABASE_URL=postgres://boticelli_user:your_password@localhost:5432/boticelli
```

The component-based approach composes the connection URL automatically and makes it easier to manage credentials separately.

### 4. Run Migrations

```bash
# Install diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Run migrations
diesel migration run
```

### 5. Test Database Integration

```bash
# Run narrative and save to database
./target/release/boticelli run -n narrations/mint.toml --save

# List saved executions
./target/release/boticelli list

# View execution details
./target/release/boticelli show 1
```

## Creating Narratives

Narratives are defined in TOML files with three main sections:

### Simple Text Narrative

```toml
[metadata]
name = "My First Narrative"
description = "A simple two-act narrative"

[toc]
order = ["greet", "farewell"]

[acts]
greet = "Say hello in a friendly way"
farewell = "Say goodbye with a warm message"
```

### Advanced Multimodal Narrative

```toml
[metadata]
name = "Image Analysis Pipeline"
description = "Analyze and describe an image"

[toc]
order = ["analyze", "summarize"]

# Act with image input and custom configuration
[acts.analyze]
model = "gemini-pro-vision"
temperature = 0.3

[[acts.analyze.input]]
type = "text"
content = "Describe this image in detail"

[[acts.analyze.input]]
type = "image"
mime = "image/png"
url = "https://example.com/image.png"

# Act with just text (uses previous output as context)
[acts]
summarize = "Summarize the analysis in one paragraph"
```

### Input Types Supported

- **text**: Plain text prompts
- **image**: PNG, JPEG, WebP, GIF (via URL, base64, or file path)
- **audio**: MP3, WAV, OGG (via URL, base64, or file path)
- **video**: MP4, WebM (via URL, base64, or file path)
- **document**: PDF, DOCX, TXT (via URL, base64, or file path)

See `narrations/showcase.toml` for a comprehensive example and `NARRATIVE_TOML_SPEC.md` for full specification.

## CLI Reference

### `run` - Execute a narrative

```bash
boticelli run --narrative <PATH> [OPTIONS]

Options:
  -n, --narrative <PATH>   Path to narrative TOML file (required)
  -b, --backend <NAME>     LLM backend to use [default: gemini]
  -a, --api-key <KEY>      API key (or use environment variable)
  -s, --save               Save execution to database
  -v, --verbose            Show detailed progress
```

**Examples:**

```bash
# Basic execution
boticelli run -n narrations/mint.toml

# With verbose output
boticelli run -n narrations/showcase.toml -v

# Save to database
boticelli run -n narrations/mint.toml --save

# Use custom API key
boticelli run -n narrations/mint.toml -a sk-your-key-here
```

### `list` - List stored executions

```bash
boticelli list [OPTIONS]

Options:
  -n, --name <NAME>    Filter by narrative name
  -l, --limit <N>      Maximum number of results [default: 10]
```

**Examples:**

```bash
# List recent executions
boticelli list

# Filter by name
boticelli list --name "Social Media Post Generation"

# Show more results
boticelli list --limit 50
```

### `show` - Display execution details

```bash
boticelli show <ID>
```

**Example:**

```bash
# Show execution ID 1
boticelli show 1
```

## Troubleshooting

### "GEMINI_API_KEY not provided"

**Problem:** API key not found in environment or `.env` file.

**Solution:**
```bash
# Option 1: Add to .env file
echo 'GEMINI_API_KEY=your-key-here' >> .env

# Option 2: Export in shell
export GEMINI_API_KEY="your-key-here"

# Option 3: Pass via command line
boticelli run -n narrations/mint.toml -a your-key-here
```

### "DATABASE_USER environment variable not set" or "DATABASE_URL environment variable not set"

**Problem:** Trying to use `--save` flag without database configuration.

**Solution:**
1. Follow [Database Setup](#database-setup-optional) above
2. Add database credentials to your `.env` file (component-based approach):
   ```env
   DATABASE_USER=boticelli_user
   DATABASE_PASSWORD=your_password
   DATABASE_NAME=boticelli
   ```
   Or use the complete URL:
   ```env
   DATABASE_URL=postgres://boticelli_user:password@localhost/boticelli
   ```
3. Run `diesel migration run`

### "Failed to connect to database"

**Problem:** PostgreSQL not running or wrong credentials.

**Solution:**
```bash
# Check if PostgreSQL is running
# Ubuntu/Debian:
sudo systemctl status postgresql

# macOS:
brew services list

# Test connection
psql $DATABASE_URL
```

### "Act prompt cannot be empty"

**Problem:** Empty or whitespace-only act in narrative TOML.

**Solution:** Ensure all acts have non-empty prompts:
```toml
# Bad
[acts]
my_act = ""

# Good
[acts]
my_act = "Describe the task to perform"
```

### Migrations fail or schema out of sync

**Problem:** Database schema doesn't match code expectations.

**Solution:**
```bash
# Reset database (WARNING: deletes all data)
diesel database reset

# Or manually drop and recreate
dropdb boticelli
createdb boticelli
diesel migration run
```

## Development

### Building from Source

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release

# With all features
cargo build --all-features
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_narrative_valid

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Check for issues
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Project Structure

```
boticelli/
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # CLI implementation
│   ├── core.rs             # Core types (Input, Output, etc.)
│   ├── error.rs            # Error handling
│   ├── interface/          # Traits (BoticelliDriver, etc.)
│   ├── models/             # Backend implementations
│   │   └── gemini.rs       # Gemini client
│   ├── narrative/          # Narrative system
│   │   ├── core.rs         # Narrative, Metadata, Toc
│   │   ├── provider.rs     # NarrativeProvider trait
│   │   ├── executor.rs     # NarrativeExecutor
│   │   ├── repository.rs   # NarrativeRepository trait
│   │   ├── in_memory_repository.rs
│   │   └── toml.rs         # TOML parsing
│   └── database/           # PostgreSQL backend
│       ├── narrative_models.rs
│       ├── narrative_repository.rs
│       └── narrative_conversions.rs
├── narrations/             # Example narratives
│   ├── mint.toml
│   └── showcase.toml
├── migrations/             # Database migrations
├── tests/                  # Integration tests
└── Cargo.toml
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run clippy (`cargo clippy --all-targets --all-features`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Roadmap

- [ ] Additional LLM backends (Anthropic Claude, OpenAI, Hugging Face)
- [ ] Streaming token output during execution
- [ ] Parallel execution of independent acts
- [ ] Variable substitution in prompts
- [ ] Conditional act execution
- [ ] Web UI for narrative management
- [ ] Export/import execution results
- [ ] Retry logic with exponential backoff
- [ ] Cost tracking per execution

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Built with [Diesel](https://diesel.rs/) for database operations
- CLI powered by [clap](https://github.com/clap-rs/clap)
- Async runtime by [Tokio](https://tokio.rs/)
- Gemini integration via [gemini-rust](https://github.com/avastmick/gemini-rust)
