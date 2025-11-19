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
    @just install-node-tools
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
    @echo "✅ Cargo tools installed"

# Install node-based tools (markdownlint)
install-node-tools:
    @echo "📝 Installing node tools..."
    npm install -g markdownlint-cli2 || echo "⚠️  npm not available, skipping markdownlint"

# Update just itself
update-just:
    @echo "⚡ Updating just..."
    cargo install just --force

# Update all dependencies (Rust, cargo tools, just)
update-all: install-rust install-cargo-tools update-just
    @echo "✅ All tools updated!"

# Building
# ========

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode
build-release:
    cargo build --release

# Build with local features (all except api)
build-local:
    cargo build --features local

# Build with all features enabled
build-all:
    cargo build --all-features

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
    @echo "✅ All local tests passed!"

# Run tests and show coverage (requires cargo-tarpaulin)
test-coverage:
    @command -v cargo-tarpaulin >/dev/null 2>&1 || (echo "Installing cargo-tarpaulin..." && cargo install cargo-tarpaulin)
    cargo tarpaulin --workspace --lib --tests --out Html --output-dir coverage

# Run complete test suite including API tests (for pre-merge)
test-pre-merge: test test-doc test-api-gemini
    @echo "✅ All pre-merge tests passed!"

# Code Quality
# ============

# Run clippy linter (no warnings allowed)
# Uses local features to match test environment
lint:
    cargo clippy --workspace --features local --all-targets

# Run clippy and fix issues automatically
lint-fix:
    cargo clippy --workspace --features local --all-targets --fix --allow-dirty --allow-staged

# Check code formatting
fmt-check:
    cargo fmt -- --check

# Format all code
fmt:
    cargo fmt

# Check markdown files for issues
lint-md:
    @command -v markdownlint-cli2 >/dev/null 2>&1 || (echo "❌ markdownlint-cli2 not installed. Run: just install-node-tools" && exit 1)
    markdownlint-cli2 "**/*.md" "#target" "#node_modules"

# Run all checks (lint, format check, tests)
check-all: lint fmt-check test
    @echo "✅ All checks passed!"

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
    diesel database setup --database-url="${DATABASE_URL}" || echo "✅ Database already exists"

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

# Execute a narrative by name (searches recursively for matching TOML files)
narrate name:
    #!/usr/bin/env bash
    set -e
    
    echo "🔍 Searching for narrative: {{name}}"
    
    # Find all TOML files recursively that match the name
    MATCHES=$(find . -type f -name "*.toml" | grep -i "{{name}}" | grep -v target | grep -v node_modules || true)
    
    if [ -z "$MATCHES" ]; then
        echo "❌ No narrative found matching '{{name}}'"
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
        cargo run -p botticelli --release --features gemini,database -- run --narrative "$NARRATIVE" --save --verbose
    else
        echo "❌ Multiple narratives found matching '{{name}}':"
        echo "$MATCHES" | sed 's/^/  /'
        echo ""
        echo "💡 Please be more specific with the name"
        exit 1
    fi

# Run example narrative: generate channel posts
example-channels:
    cargo run -p botticelli --release --features database,gemini -- run --narrative crates/botticelli_narrative/narratives/generate_channel_posts.toml

# Run example narrative: generate users
example-users:
    cargo run -p botticelli --release --features database,gemini -- run --narrative crates/botticelli_narrative/narratives/generate_users.toml

# Run example narrative: generate guilds
example-guilds:
    cargo run -p botticelli --release --features database,gemini -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml

# Run example narrative: generate guilds (simplified with prompt injection)
example-guilds-simple:
    cargo run -p botticelli --release --features database,gemini -- run --narrative crates/botticelli_narrative/narratives/generate_guilds_simple.toml

# List content from a generation table
content-list table:
    cargo run -p botticelli --release --features database,gemini -- content list {{table}}

# Show specific content item
content-show table id:
    cargo run -p botticelli --release --features database,gemini -- content show {{table}} {{id}}

# TUI (Terminal User Interface)
# ==============================

# Launch TUI for a specific table
tui table:
    cargo run -p botticelli --release --features tui -- tui {{table}}

# Launch TUI for a table with all features enabled
tui-all table:
    cargo run -p botticelli --release --all-features -- tui {{table}}

# Generate test guilds and launch TUI (full workflow)
tui-test-guilds:
    #!/usr/bin/env bash
    echo "🎲 Generating test guilds..."
    cargo run -p botticelli --release --all-features -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml
    echo "✅ Content generated in table: potential_guilds"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --all-features -- tui "potential_guilds"

# Generate test channels and launch TUI (full workflow)
tui-test-channels:
    #!/usr/bin/env bash
    echo "🎲 Generating test channels..."
    cargo run -p botticelli --release --all-features -- run --narrative crates/botticelli_narrative/narratives/generate_channel_posts.toml
    echo "✅ Content generated in table: potential_posts"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --all-features -- tui "potential_posts"

# Generate test users and launch TUI (full workflow)
tui-test-users:
    #!/usr/bin/env bash
    echo "🎲 Generating test users..."
    cargo run -p botticelli --release --all-features -- run --narrative crates/botticelli_narrative/narratives/generate_users.toml
    echo "✅ Content generated in table: potential_users"
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --all-features -- tui "potential_users"

# Generate Discord infrastructure and launch TUI for review
tui-test-discord:
    #!/usr/bin/env bash
    echo "🎲 Generating Discord infrastructure..."
    cargo run -p botticelli --release --all-features -- run --narrative crates/botticelli_narrative/narratives/discord_infrastructure.toml --process-discord
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
    cargo run -p botticelli --release --all-features -- content generations

# Show details of the last generation
content-last:
    cargo run -p botticelli --release --all-features -- content last

# Launch TUI on the most recently generated table
tui-last:
    #!/usr/bin/env bash
    set -e
    echo "📊 Getting latest generation..."
    TABLE=$(cargo run -p botticelli --release --all-features -- content last --format=table-name-only 2>/dev/null || echo "")
    if [ -z "$TABLE" ]; then
        echo "❌ No content generations found"
        echo "💡 Generate content first with: just example-guilds"
        exit 1
    fi
    echo "   Table: $TABLE"
    echo ""
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --all-features -- tui "$TABLE"

# Quick TUI demo with sample data
tui-demo:
    #!/usr/bin/env bash
    set -e
    echo "🎲 Generating sample content..."
    cargo run -p botticelli --release --all-features -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml
    echo "✅ Content generated"
    echo ""
    echo "📊 Getting latest generation..."
    TABLE=$(cargo run -p botticelli --release --all-features -- content last --format=table-name-only 2>/dev/null || echo "")
    if [ -z "$TABLE" ]; then
        echo "❌ No content generations found"
        exit 1
    fi
    echo "   Table: $TABLE"
    echo ""
    echo "🖥️  Launching TUI..."
    cargo run -p botticelli --release --all-features -- tui "$TABLE"

# Full Workflow (CI/CD)
# ====================

# Run the complete CI pipeline locally (includes API tests)
ci: fmt-check lint test-pre-merge audit
    @echo "✅ CI pipeline completed successfully!"

# Prepare for commit (format, lint, local tests only)
pre-commit: fix-all test-all
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

# Benchmarking (if applicable)
# ============================

# Run benchmarks (requires bench tests)
bench:
    cargo bench --features local

# Aliases for common tasks
# ========================

alias b := build
alias t := test
alias l := lint
alias f := fmt
alias c := check-all
alias r := run
alias d := docs
