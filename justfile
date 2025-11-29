# Botticelli Development Justfile
#
# Common tasks for building, testing, and maintaining the Botticelli project.
# Run `just` or `just --list` to see all available commands.

# Load environment variables from .env file
set dotenv-load

# Default recipe to display help
default:
    @just --list

# Development Setup
# ================

# Install all development dependencies (Rust, cargo tools, node tools)
setup:
    @echo "📦 Installing development dependencies..."
    @just install-rust
    @just install-cargo-tools
    @echo "✅ Setup complete!"

# Install or update Rust toolchain
install-rust:
    @echo "🦀 Installing/updating Rust toolchain..."
    rustup update stable
    rustup default stable
    rustup component add clippy rustfmt

# Install required cargo plugins
install-cargo-tools:
    @echo "🔧 Installing cargo tools..."
    cargo install diesel_cli --no-default-features --features postgres || true
    cargo install cargo-audit || true
    cargo install cargo-watch || true
    cargo install cargo-hack || true
    cargo install cargo-dist || true
    cargo install omnibor-cli || true
    cargo install cargo-nextest || true
    @echo "✅ Cargo tools installed"

# Update just itself
update-just:
    @echo "⚡ Updating just..."
    cargo install just || true

# Update all dependencies (Rust, cargo tools, just)
update-all: install-rust install-cargo-tools update-just
    @echo "✅ All tools updated!"

# Building and Checking
# ======================



# Build specific package or all workspace with local features
build PACKAGE="":
    #!/usr/bin/env bash
    if [ -z "{{PACKAGE}}" ]; then
        cargo build --release --features local
    else
        # Check if package has local feature
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{PACKAGE}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            cargo build --release --package {{PACKAGE}} --features local
        else
            cargo build --release --package {{PACKAGE}}
        fi
    fi

# Build with local features (all except api)
build-local:
    cargo build --features local

# Build with all features enabled
build-all:
    cargo build --all-features

# Build an example for a specific package
build-example package example:
    #!/usr/bin/env bash
    # Check if package has a 'local' feature, use it if available
    if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
       jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
        echo "🔨 Building example '{{example}}' for {{package}} with local features"
        cargo build --example {{example}} -p {{package}} --features local
    else
        echo "🔨 Building example '{{example}}' for {{package}} without features"
        cargo build --example {{example}} -p {{package}}
    fi

# Run an example for a specific package
run-example package example *args='':
    #!/usr/bin/env bash
    # Check if package has a 'local' feature, use it if available
    if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
       jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
        echo "🚀 Running example '{{example}}' for {{package}} with local features"
        cargo run --example {{example}} -p {{package}} --features local -- {{args}}
    else
        echo "🚀 Running example '{{example}}' for {{package}} without features"
        cargo run --example {{example}} -p {{package}} -- {{args}}
    fi

# Build release with local features
build-release-local:
    cargo build --release --features local

# Build release with all features
build-release-all:
    cargo build --release --all-features

# Clean build artifacts
clean:
    cargo clean

# Clean and rebuild
rebuild: clean build

# Testing
# =======

# Run LOCAL tests only (fast, no API keys required)
# Uses local features (gemini, database, discord) but NOT api
test:
    cargo test --workspace --features local --lib --tests

# Run LOCAL tests with verbose output
test-verbose:
    cargo test --workspace --features local --lib --tests -- --nocapture

# Run doctests (usually fast)
test-doc:
    cargo test --workspace --features local --doc

# Run a specific test by name (local only)
test-one name:
    cargo test --workspace --features local --lib --tests {{name}} -- --nocapture

# Run tests for a specific package, optionally filtering by test name
test-package package test_name="":
    #!/usr/bin/env bash
    # Check if package has a 'local' feature, use it if available
    if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
       jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
        echo "📦 Testing {{package}} with local features"
        if [ -n "{{test_name}}" ]; then
            cargo test -p {{package}} --features local --lib --tests {{test_name}} -- --nocapture
        else
            cargo test -p {{package}} --features local --lib --tests
        fi
    else
        echo "📦 Testing {{package}} without features"
        if [ -n "{{test_name}}" ]; then
            cargo test -p {{package}} --lib --tests {{test_name}} -- --nocapture
        else
            cargo test -p {{package}} --lib --tests
        fi
    fi

# Run API tests for Gemini (requires GEMINI_API_KEY)
test-api-gemini:
    #!/usr/bin/env bash
    set +u
    test -n "${GEMINI_API_KEY}" || (echo "❌ GEMINI_API_KEY not set. API tests require this environment variable." && exit 1)
    cargo test --workspace --features gemini,api

# Run ALL API tests (requires all API keys, expensive!)
test-api-all:
    #!/usr/bin/env bash
    set +u
    test -n "${GEMINI_API_KEY}" || (echo "⚠️  Warning: GEMINI_API_KEY not set" && exit 0)
    echo "🚀 Running all API tests (this will consume API quotas)..."
    cargo test --workspace --all-features

# Run database tests (requires DATABASE_URL)
test-db:
    #!/usr/bin/env bash
    set +u
    test -n "${DATABASE_URL}" || (echo "❌ DATABASE_URL not set. Database tests require a PostgreSQL database." && exit 1)
    cargo test --workspace --features database

# Run the full test suite: local + doc tests
test-all: test test-doc

# Run tests and show coverage (requires cargo-tarpaulin)
test-coverage:
    @command -v cargo-tarpaulin >/dev/null 2>&1 || (echo "Installing cargo-tarpaulin..." && cargo install cargo-tarpaulin)
    cargo tarpaulin --workspace --lib --tests --out Html --output-dir coverage

# Run complete test suite including API tests (for pre-merge)
test-pre-merge: test test-doc test-api-gemini

# Code Quality
# ============

# Check compilation (all features by default, or specific package)
check package="":
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        echo "🔍 Checking all packages with all features..."
        cargo check --all-features
    else
        echo "🔍 Checking package: {{package}}"
        cargo check -p "{{package}}"
    fi

# Run clippy linter (no warnings allowed)
# Uses local features to match test environment
lint package='':
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        echo "🔍 Linting entire workspace with local features"
        cargo clippy --workspace --features local --all-targets
    else
        # Check if package has a 'local' feature, use it if available
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            echo "🔍 Linting {{package}} with local features"
            cargo clippy -p {{package}} --features local --all-targets
        else
            echo "🔍 Linting {{package}} without features"
            cargo clippy -p {{package}} --all-targets
        fi
    fi

# Run clippy and fix issues automatically
lint-fix:
    cargo clippy --workspace --features local --all-targets --fix --allow-dirty --allow-staged

# Check code formatting
fmt-check:
    cargo fmt --all -- --check

# Format all code
fmt:
    cargo fmt --all

# Check markdown files for issues
lint-md:
    @command -v markdownlint-cli2 >/dev/null 2>&1 || (echo "❌ markdownlint-cli2 not installed. Run: just install-node-tools" && exit 1)
    markdownlint-cli2 "**/*.md" "#target" "#node_modules"

# Test various feature gate combinations (requires cargo-hack)
check-features:
    @command -v cargo-hack >/dev/null 2>&1 || (echo "❌ cargo-hack not installed. Run: cargo install cargo-hack" && exit 1)
    ./scripts/feature-gate-check.sh

# Run all checks (lint, format check, tests)
check-all package='':
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        echo "🔍 Running all checks on entire workspace..."
        just fmt
        just lint
        just test-all
    else
        echo "🔍 Running all checks on {{package}}..."
        just fmt
        just lint "{{package}}"
        just test-package "{{package}}"
        # Run doc tests for the package if it has any
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            cargo test -p "{{package}}" --features local --doc
        else
            cargo test -p "{{package}}" --doc
        fi
    fi
    echo "✅ All checks passed!"

# Fix all auto-fixable issues
fix-all: fmt lint-fix
    @echo "✅ Auto-fixes applied!"

# Security
# ========

# Check for security vulnerabilities in dependencies
audit:
    cargo audit

# Update dependencies and check for vulnerabilities
audit-fix:
    cargo update
    cargo audit

# Database
# ========

# Run database migrations
db-migrate:
    diesel migration run

# Revert last database migration
db-revert:
    diesel migration revert

# Redo last migration (revert then run)
db-redo:
    diesel migration redo

# Reset database (revert all, then run all)
db-reset:
    diesel migration revert --all
    diesel migration run

# Create a new migration
db-migration name:
    diesel migration generate {{name}}

# Check database connection
db-check:
    #!/usr/bin/env bash
    set +u
    echo "🔍 Checking database connection..."
    diesel database setup --database-url="${DATABASE_URL}" && echo "✅ Database setup complete" || echo "⚠️  Database setup failed or already exists"

# Setup database from scratch
db-setup:
    diesel setup

# Development
# ===========

# Watch for changes and run local tests
watch:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x 'test --workspace --features local --lib --tests'

# Watch and run specific command on changes
watch-cmd cmd:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x '{{cmd}}'

# Run the binary in development mode
run *args:
    cargo run -p botticelli -- {{args}}

# Run with database features enabled
run-db *args:
    cargo run -p botticelli --features database -- {{args}}

# Run with all features
run-all *args:
    cargo run -p botticelli --all-features -- {{args}}

# Content Generation Examples
# ===========================

# Execute a narrative by name (supports file.narrative syntax for multi-narrative files)
narrate PATTERN:
    #!/usr/bin/env bash
    set -e
    
    # Check if pattern contains a dot (file.narrative_name syntax)
    PATTERN="{{PATTERN}}"
    if [[ "$PATTERN" == *.* ]]; then
        FILE_PART="${PATTERN%.*}"
        NARRATIVE_NAME="${PATTERN##*.}"
        
        echo "🔍 Searching for multi-narrative file: ${FILE_PART}"
        NARRATIVE_FILE=$(find ./crates/botticelli_narrative/narratives -type f -path "*/${FILE_PART}.toml" | head -1)
        
        if [ -z "$NARRATIVE_FILE" ]; then
            echo "❌ No narrative file found matching '${FILE_PART}'"
            echo ""
            echo "📂 Available narratives:"
            find crates/botticelli_narrative/narratives -type f -name "*.toml" 2>/dev/null | sed 's|crates/botticelli_narrative/narratives/||' | sed 's/\.toml$//' | sort || echo "  (no narratives directory)"
            exit 1
        fi
        
        echo "✓ Found: $NARRATIVE_FILE"
        echo "✓ Loading narrative: ${NARRATIVE_NAME}"
        echo ""
        echo "🚀 Executing narrative..."
        STATE_DIR="${BOTTICELLI_STATE_DIR:-.narrative_state}"
        cargo run -p botticelli --release --features local -- run \
            --narrative "$NARRATIVE_FILE" \
            --narrative-name "${NARRATIVE_NAME}" \
            --save \
            --state-dir "$STATE_DIR" \
            --process-discord \
            --verbose
    else
        # Original behavior: search for file by name
        echo "🔍 Searching for narrative: {{PATTERN}}"
        
        # Find all TOML files recursively that match the name
        MATCHES=$(find ./crates/botticelli_narrative/narratives -type f -name "*.toml" | grep -i "{{PATTERN}}" | grep -v target | grep -v node_modules || true)
        
        if [ -z "$MATCHES" ]; then
            echo "❌ No narrative found matching '{{PATTERN}}'"
            echo ""
            echo "📂 Available narratives:"
            find crates/botticelli_narrative/narratives -type f -name "*.toml" 2>/dev/null | sed 's|crates/botticelli_narrative/narratives/||' | sed 's/\.toml$//' | sort || echo "  (no narratives directory)"
            exit 1
        fi
        
        # Count matches
        COUNT=$(echo "$MATCHES" | wc -l)
        
        if [ "$COUNT" -eq 1 ]; then
            NARRATIVE="$MATCHES"
            echo "✓ Found: $NARRATIVE"
            echo ""
            echo "🚀 Executing narrative..."
            STATE_DIR="${BOTTICELLI_STATE_DIR:-.narrative_state}"
            cargo run -p botticelli --release --features local -- run \
                --narrative "$NARRATIVE" \
                --save \
                --state-dir "$STATE_DIR" \
                --process-discord \
                --verbose
        else
            echo "❌ Multiple narratives found matching '{{PATTERN}}':"
            echo "$MATCHES" | sed 's/^/  /'
            echo ""
            echo "💡 Please be more specific with the name"
            exit 1
        fi
    fi
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "✅ Narrative execution completed successfully"
    else
        echo ""
        echo "❌ Narrative execution failed"
        exit 1
    fi

# Execute a narrative from tests for testing purposes
test-narrate path:
    #!/usr/bin/env bash
    set -e
    cargo run -p botticelli --features local -- run --narrative "{{path}}" --save --process-discord

# Run example narrative: generate channel posts
example-channels:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_channel_posts.toml

# Run example narrative: generate users
example-users:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_users.toml

# Run example narrative: generate guilds
example-guilds:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml

# Run example narrative: generate guilds (simplified with prompt injection)
example-guilds-simple:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds_simple.toml

# List content from a generation table
content-list table:
    cargo run -p botticelli --release --features local -- content list {{table}}

# Show specific content item
content-show table id:
    cargo run -p botticelli --release --features local -- content show {{table}} {{id}}

# Model Server Management
# =======================

# List available models for download
server-models:
    cargo run -p botticelli --release --features server -- server list

# Download a model by name
server-download model:
    cargo run -p botticelli --release --features server -- server download {{model}}

# Start the inference server
server-start model="mistral":
    cargo run -p botticelli --release --features server -- server start {{model}}

# Stop the inference server
server-stop:
    cargo run -p botticelli --release --features server -- server stop

# Check server status
server-status:
    cargo run -p botticelli --release --features server -- server status

# TUI (Terminal User Interface)
# ==============================

# Launch TUI for a specific table
tui table:
    cargo run -p botticelli --release --features tui -- tui {{table}}

# Launch TUI server management view
tui-server:
    cargo run -p botticelli --release --features tui,server -- tui-server

# Launch TUI for a table with all features enabled
tui-all table:
    cargo run -p botticelli --release --features local -- tui {{table}}

# Generate test guilds and launch TUI (full workflow)
tui-test-guilds:
    #!/usr/bin/env bash
    echo "🎲 Generating test guilds..."
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml
    echo "✅ Content generated in table: potential_guilds"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --features local -- tui "potential_guilds"

# Generate test channels and launch TUI (full workflow)
tui-test-channels:
    #!/usr/bin/env bash
    echo "🎲 Generating test channels..."
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_channel_posts.toml
    echo "✅ Content generated in table: potential_posts"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --features local -- tui "potential_posts"

# Generate test users and launch TUI (full workflow)
tui-test-users:
    #!/usr/bin/env bash
    echo "🎲 Generating test users..."
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_users.toml
    echo "✅ Content generated in table: potential_users"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --features local -- tui "potential_users"

# Generate Discord infrastructure and launch TUI for review
tui-test-discord:
    #!/usr/bin/env bash
    echo "🎲 Generating Discord infrastructure..."
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/discord_infrastructure.toml --process-discord
    echo "✅ Discord infrastructure generated"
    echo "💡 Note: Discord infrastructure uses fixed IDs, check discord_guilds table directly"
    echo "🖥️  To review generated content, use:"
    echo "   just content-list discord_guilds"

# List all content generation tables in database  
tui-list-tables:
    #!/usr/bin/env bash
    echo "📋 Content Generation Tables:"
    echo "============================="
    psql "${DATABASE_URL}" -c "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename LIKE '%_gen_%' OR tablename LIKE '%_generation_%' ORDER BY tablename;" -t

# List all content generations with tracking metadata
content-generations:
    cargo run -p botticelli --release --features local -- content generations

# Show details of the last generation
content-last:
    cargo run -p botticelli --release --features local -- content last

# Launch TUI on the most recently generated table
tui-last:
    #!/usr/bin/env bash
    set -e
    echo "📊 Getting latest generation..."
    TABLE=$(cargo run -p botticelli --release --features local -- content last --format=table-name-only 2>/dev/null || echo "")
    if [ -z "$TABLE" ]; then
        echo "❌ No content generations found"
        echo "💡 Generate content first with: just example-guilds"
        exit 1
    fi
    echo "   Table: $TABLE"
    echo ""
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --features local -- tui "$TABLE"

# Quick TUI demo with sample data
tui-demo:
    #!/usr/bin/env bash
    set -e
    echo "🎲 Generating sample content..."
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml
    echo "✅ Content generated"
    echo ""
    echo "📊 Getting latest generation..."
    TABLE=$(cargo run -p botticelli --release --features local -- content last --format=table-name-only 2>/dev/null || echo "")
    if [ -z "$TABLE" ]; then
        echo "❌ No content generations found"
        exit 1
    fi
    echo "   Table: $TABLE"
    echo ""
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --features local -- tui "$TABLE"

# Full Workflow (CI/CD)
# ====================

# Run the complete CI pipeline locally (includes API tests)
ci: fmt-check lint check-features test-pre-merge audit
    @echo "✅ CI pipeline completed successfully!"

# Prepare for commit (format, lint, local tests, feature checks)
pre-commit: fix-all check-features test-all
    @echo "✅ Ready to commit!"

# Prepare for merge (all checks including API tests)
pre-merge: pre-commit test-api-gemini
    @echo "✅ Ready to merge!"

# Prepare for release (all checks + release build)
pre-release: ci build-release-local
    @echo "✅ Ready for release!"

# Git helpers
# ===========

# Stage all changes and show status
stage:
    git add -A
    git status --short

# Quick commit with message
commit msg: pre-commit stage
    git commit -m "{{msg}}"

# Quick commit and push to current branch
push msg: 
    @just commit "{{msg}}"
    git push origin $(git branch --show-current)

# Documentation
# =============

# Generate and open Rust documentation
docs:
    cargo doc --workspace --features local --no-deps --open

# Check documentation for issues
docs-check:
    cargo doc --workspace --features local --no-deps

# Build and view documentation for a specific crate
docs-crate crate:
    cargo doc --package {{crate}} --no-deps --open

# Information
# ===========

# Show project statistics
stats:
    @echo "📊 Project Statistics"
    @echo "===================="
    @echo ""
    @echo "Workspace crates:"
    @ls -1d crates/*/ | wc -l
    @echo ""
    @echo "Lines of Rust code (all crates):"
    @find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + 2>/dev/null | tail -1 || echo "  0"
    @echo ""
    @echo "Lines of test code:"
    @find crates/*/tests tests -name '*.rs' 2>/dev/null -exec wc -l {} + 2>/dev/null | tail -1 || echo "  0"
    @echo ""
    @echo "Number of dependencies:"
    @grep -c "^name =" Cargo.lock 2>/dev/null || echo "  0"
    @echo ""
    @echo "Database migrations:"
    @ls migrations/ 2>/dev/null | grep -v "^total" | wc -l || echo "  0 migrations"

# Show environment information
env:
    #!/usr/bin/env bash
    set +u
    echo "🔧 Environment Information"
    echo "========================="
    echo ""
    echo "Rust version:"
    rustc --version
    echo ""
    echo "Cargo version:"
    cargo --version
    echo ""
    echo "Just version:"
    just --version
    echo ""
    echo "Diesel CLI:"
    diesel --version 2>/dev/null || echo "  Not installed"
    echo ""
    echo "Database URL:"
    echo "  ${DATABASE_URL:-Not set}"

# Show available features
features:
    @echo "🎛️  Available Features"
    @echo "===================="
    @echo ""
    @echo "Main crate features:"
    @grep '^\[features\]' -A 20 crates/botticelli/Cargo.toml | grep -v '^\[' | grep '='

# Utility
# =======

# Remove generated files and caches
clean-all: clean
    @echo "🧹 Deep cleaning..."
    rm -rf target/
    rm -rf coverage/
    rm -f Cargo.lock
    @echo "✅ All build artifacts removed"

# Check for outdated dependencies
outdated:
    @command -v cargo-outdated >/dev/null 2>&1 || (echo "Installing cargo-outdated..." && cargo install cargo-outdated)
    cargo outdated

# Update dependencies to latest compatible versions
update-deps:
    cargo update
    @echo "✅ Dependencies updated. Run 'just test' to verify."

# Generate OmniBOR artifact tree for supply chain transparency
omnibor:
    @command -v omnibor >/dev/null 2>&1 || (echo "Installing omnibor-cli..." && cargo install omnibor-cli)
    omnibor --help > /dev/null && echo "✅ OmniBOR installed" || echo "❌ OmniBOR not found - install with: cargo install omnibor"

# Run all security checks
security: audit omnibor
    @echo "✅ Security checks completed!"

# Release Management
# ==================

# Build distribution artifacts for current platform
dist-build:
    dist build

# Build and check distribution artifacts (doesn't upload)
dist-check:
    dist build --check

# Generate release configuration
dist-init:
    dist init

# Plan a release (preview changes)
dist-plan:
    dist plan

# Generate CI workflow files
dist-generate:
    dist generate

# Benchmarking (if applicable)
# ============================

# Run benchmarks (requires bench tests)
bench:
    cargo bench --features local

# Bot Server Management
# ======================

# Start the bot server with all three bots (generation, curation, posting)
bot-server:
    @echo "🤖 Starting Botticelli bot server..."
    @echo "📝 Generation bot: Every 6 hours"
    @echo "🎯 Curation bot: Every 12 hours, processes until queue empty"
    @echo "📤 Posting bot: Every 2-4 hours with jitter"
    cargo run --release --features bots --bin botticelli -- server

# Start only the generation bot (for testing)
bot-generate:
    @echo "📝 Starting generation bot only..."
    cargo run --release --features local --bin botticelli -- server --only generation

# Start only the curation bot (for testing)
bot-curate:
    @echo "🎯 Starting curation bot only..."
    cargo run --release --features local --bin botticelli -- server --only curation

# Start only the posting bot (for testing)
bot-post:
    @echo "📤 Starting posting bot only..."
    cargo run --release --features local --bin botticelli -- server \
        --posting-narrative ./crates/botticelli_narrative/narratives/discord/posting.toml \
        --posting-name scheduled_post

# Aliases for common tasks
# ========================

alias b := build
alias t := test
alias l := lint
alias f := fmt
alias c := check-all
alias r := run
alias d := docs

# Run tests with timing information using nextest
test-timings:
    cargo nextest run --workspace --features local

# Install nextest if not present
install-nextest:
    cargo install cargo-nextest --locked
