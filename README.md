# Botticelli

A Rust framework for orchestrating multi-LLM workflows with TOML-defined narratives, automated content pipelines, and social media integration.

## Overview

Botticelli enables you to define complex, multi-step LLM workflows in TOML files called "narratives." Each narrative consists of multiple "acts" that execute sequentially, with each act seeing the outputs from previous acts as context. Beyond simple workflows, Botticelli includes a bot server architecture for automated content generation, curation, and social media posting.

### Core Capabilities

- **Narrative Execution**: TOML-defined multi-act LLM workflows with context passing
- **Content Pipelines**: Generate → Critique → Refine → Curate → Post
- **Bot Server**: Long-running actors for automated content workflows
- **Social Integration**: Discord bot commands and automated posting
- **Database-Driven**: PostgreSQL-backed content storage and tracking

### Example Workflows

- Generate content → Critique → Improve → Store in database
- Curate stored content → Select best posts → Approve for publishing
- Scheduled posting → Pull approved content → Post to Discord
- Discord commands → Query data → Format response → Reply

## Features

- 🎭 **Multi-Act Narratives**: Define sequential LLM workflows in TOML
- 🔄 **Narrative Composition**: Reference narratives within narratives, use carousels for iteration
- 🎨 **Multimodal Support**: Text, images, audio, video, and documents
- 🔌 **Multiple Backends**: Gemini (Anthropic, OpenAI, and others planned)
- ⚙️ **Per-Act Configuration**: Different models, temperature, max_tokens per act
- 💾 **Database Integration**: PostgreSQL storage with automatic schema inference
- 🤖 **Bot Server**: Automated content generation, curation, and posting actors
- 📱 **Social Platforms**: Discord integration (Twitter, Reddit planned)
- 📊 **Observability**: OpenTelemetry tracing and metrics with Jaeger integration
- 🖥️ **CLI Interface**: Flexible command-line execution with Just recipes
- ⚡ **Rate Limiting**: Intelligent rate limiting with budget multipliers
- 🦀 **Type-Safe**: Full Rust type safety throughout

## Workspace Architecture

Botticelli is organized as a Cargo workspace with focused, independent crates:

### Foundation Layer

- **botticelli_error** - Error types with caller location tracking
- **botticelli_core** - Core data structures (Input, Output, Message, Role)
- **botticelli_interface** - Trait definitions for drivers and repositories

### Narrative Execution Layer

- **botticelli_narrative** - Narrative execution engine with composition support
- **botticelli_rate_limit** - Rate limiting with budget multipliers and tier management
- **botticelli_storage** - Content-addressable file storage with hash verification
- **botticelli_cache** - Caching layer for database queries and LLM responses

### Backend Integration Layer

- **botticelli_models** - LLM provider implementations (feature-gated)
  - `gemini` - Google Gemini models (1.5 Pro, 1.5 Flash, 2.0 Flash, etc.)
  - Anthropic Claude (planned)
  - OpenAI GPT (planned)
  - Local models via Ollama (planned)

### Data Persistence Layer

- **botticelli_database** - PostgreSQL with automatic schema inference and table management
- **botticelli_security** - Authentication, authorization, and security context

### Social Integration Layer

- **botticelli_social** - Social platform integrations
  - Discord bot commands and automated posting
  - Twitter integration (planned)
  - Reddit integration (planned)

### Server & Bot Layer

- **botticelli_server** - Server infrastructure with health checks and metrics
- **botticelli_bot** - Content generation, curation, and posting bots
- **botticelli_actor** - Actor-based architecture for long-running processes

### User Interface Layer

- **botticelli_tui** - Terminal UI for content review and approval

### Facade Crate

- **botticelli** - Main binary and library crate that orchestrates everything

### Using the Workspace

**Simple approach** - Use the main facade crate:

```toml
[dependencies]
botticelli = { version = "0.2", features = ["gemini", "database"] }
```

**Advanced approach** - Use individual crates:

```toml
[dependencies]
botticelli_interface = "0.2"
botticelli_models = { version = "0.2", features = ["gemini"] }
botticelli_narrative = "0.2"
# Smaller dependency tree, faster compile times
```

See individual crate READMEs in `crates/*/README.md` for detailed documentation.

## Quick Start

### Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **PostgreSQL** 14+ (optional, only if using `--save` flag)

### Installation

```bash
# Clone the repository
git clone https://github.com/crumplecup/botticelli.git
cd botticelli

# Build the project
cargo build --release

# The binary will be at ./target/release/botticelli
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
DATABASE_USER=botticelli_user
DATABASE_PASSWORD=your_password
DATABASE_NAME=botticelli  # Optional: defaults to botticelli
```

3. **Get an API key**:
   - **Gemini**: Visit [Google AI Studio](https://makersuite.google.com/app/apikey)

### Run Your First Narrative

```bash
# Set your API key (if not in .env)
export GEMINI_API_KEY="your-key-here"

# Run the example narrative
./target/release/botticelli run --narrative narrations/mint.toml --verbose
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

### Using Just for Easy Narrative Execution

If you have [Just](https://github.com/casey/just) installed (recommended for development), you can use the convenient `narrate` command:

```bash
# Search and run a narrative by name
just narrate model_options

# Works with partial names (must match exactly one file)
just narrate test_minimal

# If multiple matches, it will ask you to be more specific
just narrate generate
```

The `narrate` command:
- Recursively searches the workspace for matching `.toml` files
- Excludes build artifacts (`target/`, `node_modules/`)
- Shows helpful error messages if not found or ambiguous
- Automatically runs with the gemini feature enabled

To install Just:
```bash
cargo install just
```

## Database Setup (Optional)

If you want to save execution history with the `--save` flag, you'll need PostgreSQL.

📖 **New to PostgreSQL or need detailed setup help?** See [POSTGRES.md](POSTGRES.md) for a comprehensive step-by-step guide.

Quick setup for experienced users:

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
CREATE DATABASE botticelli;
CREATE USER botticelli_user WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE botticelli TO botticelli_user;
\q
```

### 3. Configure Database Connection

Add to your `.env` file (component-based approach recommended):

```env
# Option 1: Component-based (recommended)
DATABASE_USER=botticelli_user
DATABASE_PASSWORD=your_password
DATABASE_HOST=localhost      # Optional: defaults to localhost
DATABASE_PORT=5432            # Optional: defaults to 5432
DATABASE_NAME=botticelli       # Optional: defaults to botticelli

# Option 2: Complete URL (alternative - takes precedence)
# DATABASE_URL=postgres://botticelli_user:your_password@localhost:5432/botticelli
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
./target/release/botticelli run -n narrations/mint.toml --save

# List saved executions
./target/release/botticelli list

# View execution details
./target/release/botticelli show 1
```

## Observability & Monitoring

Botticelli includes production-ready OpenTelemetry integration for distributed tracing and metrics collection.

### Quick Start with Jaeger

Start the observability stack with Podman:

```bash
# If you already have PostgreSQL running locally (port 5432):
podman-compose -f docker-compose.jaeger-only.yml up -d

# OR, if you need both Jaeger and PostgreSQL:
podman-compose up -d  # PostgreSQL on port 5433

# Configure environment
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Build with observability
cargo build --release --features otel-otlp

# Run actor server
cargo run --release -p botticelli_actor --bin actor-server \
  --features otel-otlp,discord
```

Access Jaeger UI at **http://localhost:16686** to view traces!

### What You Get

**Distributed Tracing:**
- See execution flow across narrative acts
- Trace API calls to LLM providers
- Track database queries and social media operations
- Measure latency at each step

**Metrics Collection:**
- Bot execution counts and failures
- Narrative execution duration
- API call counts by provider
- Queue depth and throughput

**Two Modes:**

| Mode | Feature | Output | Use Case |
|------|---------|--------|----------|
| **Development** | `observability` | Stdout | Local debugging |
| **Production** | `otel-otlp` | OTLP → Jaeger/Tempo | Production monitoring |

### Environment Variables

```bash
# Exporter type
OTEL_EXPORTER=stdout          # Development (default)
OTEL_EXPORTER=otlp            # Production

# OTLP endpoint
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Log verbosity
RUST_LOG=info,botticelli=debug
```

### Docker/Podman Stack

The included `docker-compose.yml` provides:
- **Jaeger**: All-in-one collector, query, and UI
- **PostgreSQL**: Database for bot state
- Automatic networking and persistence

📖 **For detailed setup, troubleshooting, and production deployment:** See [OBSERVABILITY_SETUP.md](OBSERVABILITY_SETUP.md)

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
botticelli run --narrative <PATH> [OPTIONS]

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
botticelli run -n narrations/mint.toml

# With verbose output
botticelli run -n narrations/showcase.toml -v

# Save to database
botticelli run -n narrations/mint.toml --save

# Use custom API key
botticelli run -n narrations/mint.toml -a sk-your-key-here
```

### `list` - List stored executions

```bash
botticelli list [OPTIONS]

Options:
  -n, --name <NAME>    Filter by narrative name
  -l, --limit <N>      Maximum number of results [default: 10]
```

**Examples:**

```bash
# List recent executions
botticelli list

# Filter by name
botticelli list --name "Social Media Post Generation"

# Show more results
botticelli list --limit 50
```

### `show` - Display execution details

```bash
botticelli show <ID>
```

**Example:**

```bash
# Show execution ID 1
botticelli show 1
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
botticelli run -n narrations/mint.toml -a your-key-here
```

### "DATABASE_USER environment variable not set" or "DATABASE_URL environment variable not set"

**Problem:** Trying to use `--save` flag without database configuration.

**Solution:**
1. Follow [Database Setup](#database-setup-optional) above
2. Add database credentials to your `.env` file (component-based approach):
   ```env
   DATABASE_USER=botticelli_user
   DATABASE_PASSWORD=your_password
   DATABASE_NAME=botticelli
   ```
   Or use the complete URL:
   ```env
   DATABASE_URL=postgres://botticelli_user:password@localhost/botticelli
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

### "role 'botticelli' does not exist" or "database 'botticelli' does not exist"

**Problem:** PostgreSQL database and user haven't been created yet.

**Solution:** See [POSTGRES.md](POSTGRES.md) for step-by-step instructions on creating the database and user, or run:
```bash
# Connect as postgres superuser
sudo -u postgres psql

# Create user
CREATE USER botticelli WITH PASSWORD 'your_password';

# Create database
CREATE DATABASE botticelli OWNER botticelli;

# Grant privileges
GRANT ALL PRIVILEGES ON DATABASE botticelli TO botticelli;

# Exit
\q
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
dropdb botticelli
createdb botticelli
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
botticelli/
├── crates/                          # Workspace crates
│   ├── botticelli/                  # Main binary and facade
│   │   ├── src/
│   │   │   ├── main.rs              # CLI entry point
│   │   │   ├── commands/            # CLI commands (run, server, etc.)
│   │   │   └── config.rs            # Configuration management
│   │   └── Cargo.toml
│   ├── botticelli_core/             # Core data types
│   │   ├── src/
│   │   │   ├── input.rs             # Input variants (Text, Image, etc.)
│   │   │   ├── output.rs            # Output types
│   │   │   ├── message.rs           # Message structure
│   │   │   └── role.rs              # Role enum (User, Assistant, System)
│   │   └── Cargo.toml
│   ├── botticelli_narrative/        # Narrative execution engine
│   │   ├── src/
│   │   │   ├── core.rs              # Narrative, Act, Toc
│   │   │   ├── executor.rs          # Execution logic
│   │   │   ├── processor.rs         # Content generation processors
│   │   │   ├── extraction.rs        # JSON extraction and validation
│   │   │   └── toml.rs              # TOML parsing with multi-narrative support
│   │   ├── narratives/              # Built-in narratives
│   │   │   ├── discord/             # Discord content workflows
│   │   │   │   ├── generation_carousel.toml
│   │   │   │   ├── curate_and_approve.toml
│   │   │   │   └── json_compliance.toml
│   │   │   └── examples/            # Example narratives
│   │   └── Cargo.toml
│   ├── botticelli_database/         # PostgreSQL integration
│   │   ├── src/
│   │   │   ├── connection.rs        # Connection management
│   │   │   ├── repository.rs        # Repository implementations
│   │   │   ├── schema_inference.rs  # Automatic schema creation
│   │   │   └── table_registry.rs    # Dynamic table management
│   │   └── Cargo.toml
│   ├── botticelli_models/           # LLM backend implementations
│   │   ├── src/
│   │   │   ├── gemini/              # Google Gemini client
│   │   │   └── traits.rs            # Backend traits
│   │   └── Cargo.toml
│   ├── botticelli_social/           # Social platform integrations
│   │   ├── src/
│   │   │   ├── discord/             # Discord API wrapper
│   │   │   └── commands/            # Bot commands
│   │   └── Cargo.toml
│   ├── botticelli_bot/              # Bot implementations
│   │   ├── src/
│   │   │   ├── generation.rs        # Content generation bot
│   │   │   ├── curation.rs          # Content curation bot
│   │   │   └── posting.rs           # Scheduled posting bot
│   │   └── Cargo.toml
│   ├── botticelli_server/           # Server infrastructure
│   │   ├── src/
│   │   │   ├── health.rs            # Health checks
│   │   │   └── orchestrator.rs      # Bot orchestration
│   │   └── Cargo.toml
│   └── ...                          # Other crates
├── migrations/                      # Diesel database migrations
├── scripts/                         # Utility scripts
├── examples/                        # Example programs
├── .env.example                     # Environment variable template
├── botticelli.toml                  # Application configuration
├── justfile                         # Task runner recipes
└── Cargo.toml                       # Workspace manifest
```

## Documentation

Comprehensive guides and references:

- **[Observability Guide](OBSERVABILITY.md)** - Metrics, tracing, and monitoring
- **[Narrative TOML Spec](NARRATIVE_TOML_SPEC.md)** - Complete narrative configuration reference
- **[Discord Setup Guide](DISCORD_SETUP.md)** - Configure Discord bot integration
- **[PostgreSQL Setup](POSTGRES.md)** - Database configuration
- **[Media Storage](MEDIA_STORAGE.md)** - Media storage configuration
- **[Gemini Integration](GEMINI.md)** - Google Gemini API setup
- **[Usage Tiers](USAGE_TIERS.md)** - API rate limiting and tier management
- **[Testing Patterns](TESTING_PATTERNS.md)** - Testing strategies for narratives
- **[Planning Index](PLANNING_INDEX.md)** - Index of planning and strategy documents

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

### LLM Backend Expansion
- [ ] **Anthropic Claude** integration (Claude 3.5 Sonnet, Opus)
- [ ] **OpenAI GPT** support (GPT-4, GPT-4 Turbo)
- [ ] **Local models** via Ollama (Llama 3, Mistral, etc.)
- [ ] **Hugging Face** inference API support

### Social Platform Integration
- [ ] **Twitter/X** bot commands and automated posting
- [ ] **Reddit** integration for subreddit management
- [ ] **Telegram** bot support
- [ ] **Mastodon** integration

### Bot Server Enhancements
- [ ] **Observability**: Structured metrics, tracing, and dashboards
- [ ] **Health monitoring**: Detailed health checks and status reporting
- [ ] **Dynamic configuration**: Hot-reload of bot parameters
- [ ] **Content approval workflow**: Human-in-the-loop via web UI or TUI
- [ ] **Multi-platform posting**: Cross-post approved content to multiple platforms
- [ ] **A/B testing**: Track engagement metrics and optimize content strategies

### Narrative System Improvements
- [ ] **Streaming output**: Real-time token streaming during execution
- [ ] **Parallel execution**: Run independent acts concurrently
- [ ] **Variable substitution**: Template variables in prompts and content
- [ ] **Conditional execution**: If/else logic and dynamic branching
- [ ] **Retry policies**: Configurable retry with exponential backoff
- [ ] **Cost tracking**: Per-execution token usage and cost analysis

### Developer Experience
- [ ] **Web UI**: Browser-based narrative editor and execution monitor
- [ ] **Narrative templates**: Library of reusable workflow patterns
- [ ] **Testing framework**: Unit tests for narratives with mock responses
- [ ] **Documentation generator**: Auto-generate docs from narrative TOML
- [ ] **Migration tools**: Upgrade narratives between schema versions

### Production Readiness
- [ ] **Docker images**: Official container images for easy deployment
- [ ] **Kubernetes operators**: Native k8s deployment and scaling
- [ ] **Backup/restore**: Database backup automation and point-in-time recovery
- [ ] **Multi-tenancy**: Isolated workspaces for multiple users/organizations
- [ ] **API server**: RESTful API for narrative execution and management

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
