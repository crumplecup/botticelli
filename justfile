# Boticelli Development Justfile
#
# Common tasks for building, testing, and maintaining the Boticelli project.
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

# Build with all features enabled
build-all:
    cargo build --all-features

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

# Run all tests
test:
    #!/usr/bin/env bash
    set +u
    if [ -z "${DATABASE_URL}" ]; then \
        echo "⚠️  Skipping database integration tests (DATABASE_URL not set)"; \
        cargo test --all-features --lib; \
        cargo test --all-features --bins; \
        cargo test --all-features --test discord_processor_test; \
        cargo test --all-features --test narrative_test; \
        cargo test --all-features --test rate_limit_integration_test; \
        cargo test --all-features --test rate_limit_tier_test; \
        cargo test --all-features --test storage_test; \
    else \
        cargo test --all-features; \
    fi

# Run all tests including database integration (requires DATABASE_URL)
test-all:
    #!/usr/bin/env bash
    set +u
    test -n "${DATABASE_URL}" || (echo "❌ DATABASE_URL not set. These tests require a PostgreSQL database." && exit 1)
    cargo test --all-features

# Run tests with output
test-verbose:
    cargo test --all-features -- --nocapture

# Run tests for specific feature
test-feature feature:
    cargo test --features {{feature}}

# Run database tests only
test-db:
    cargo test --features database

# Run a specific test by name
test-one name:
    cargo test --all-features {{name}} -- --nocapture

# Run tests and show coverage (requires cargo-tarpaulin)
test-coverage:
    @command -v cargo-tarpaulin >/dev/null 2>&1 || (echo "Installing cargo-tarpaulin..." && cargo install cargo-tarpaulin)
    cargo tarpaulin --all-features --out Html --output-dir coverage

# Code Quality
# ============

# Run clippy linter (no warnings allowed)
lint:
    cargo clippy --all-features --all-targets

# Run clippy and fix issues automatically
lint-fix:
    cargo clippy --all-features --all-targets --fix --allow-dirty --allow-staged

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

# Watch for changes and run tests
watch:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x 'test --all-features'

# Watch and run specific command on changes
watch-cmd cmd:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x '{{cmd}}'

# Run the binary in development mode
run *args:
    cargo run -- {{args}}

# Run with database features enabled
run-db *args:
    cargo run --features database -- {{args}}

# Run with all features
run-all *args:
    cargo run --all-features -- {{args}}

# Content Generation Examples
# ===========================

# Run example narrative: generate channel posts
example-channels:
    cargo run --features database,gemini -- run --narrative narratives/generate_channel_posts.toml

# Run example narrative: generate users
example-users:
    cargo run --features database,gemini -- run --narrative narratives/generate_users.toml

# Run example narrative: generate guilds
example-guilds:
    cargo run --features database,gemini -- run --narrative narratives/generate_guilds.toml

# Run example narrative: generate guilds (simplified with prompt injection)
example-guilds-simple:
    cargo run --features database,gemini -- run --narrative narratives/generate_guilds_simple.toml

# List content from a generation table
content-list table:
    cargo run --features database,gemini -- content list {{table}}

# Show specific content item
content-show table id:
    cargo run --features database,gemini -- content show {{table}} {{id}}

# TUI (Terminal User Interface)
# ==============================

# Launch TUI for a specific table
tui table:
    cargo run --features tui -- tui {{table}}

# Launch TUI for a table with all features enabled
tui-all table:
    cargo run --all-features -- tui {{table}}

# Generate test guilds and launch TUI (full workflow)
tui-test-guilds:
    #!/usr/bin/env bash
    echo "🎲 Generating test guilds..."
    TABLE=$(cargo run --all-features -- run --narrative narratives/generate_guilds.toml 2>&1 | grep -o "guilds_gen_[0-9]*" | head -1)
    if [ -z "$TABLE" ]; then
        echo "❌ Failed to generate content or extract table name"
        echo "💡 Trying with default table name..."
        TABLE="guilds_gen_001"
    fi
    echo "✅ Content generated in table: $TABLE"
    echo "🖥️  Launching TUI..."
    cargo run --all-features -- tui "$TABLE"

# Generate test channels and launch TUI (full workflow)
tui-test-channels:
    #!/usr/bin/env bash
    echo "🎲 Generating test channels..."
    TABLE=$(cargo run --all-features -- run --narrative narratives/generate_channel_posts.toml 2>&1 | grep -o "channel_posts_[0-9]*" | head -1)
    if [ -z "$TABLE" ]; then
        echo "❌ Failed to generate content or extract table name"
        echo "💡 Trying with default table name..."
        TABLE="channel_posts_001"
    fi
    echo "✅ Content generated in table: $TABLE"
    echo "🖥️  Launching TUI..."
    cargo run --all-features -- tui "$TABLE"

# Generate test users and launch TUI (full workflow)
tui-test-users:
    #!/usr/bin/env bash
    echo "🎲 Generating test users..."
    TABLE=$(cargo run --all-features -- run --narrative narratives/generate_users.toml 2>&1 | grep -o "users_gen_[0-9]*" | head -1)
    if [ -z "$TABLE" ]; then
        echo "❌ Failed to generate content or extract table name"
        echo "💡 Trying with default table name..."
        TABLE="users_gen_001"
    fi
    echo "✅ Content generated in table: $TABLE"
    echo "🖥️  Launching TUI..."
    cargo run --all-features -- tui "$TABLE"

# Generate Discord infrastructure and launch TUI for review
tui-test-discord:
    #!/usr/bin/env bash
    echo "🎲 Generating Discord infrastructure..."
    cargo run --all-features -- run --narrative narratives/discord_infrastructure.toml --process-discord
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

# Quick TUI demo with sample data (uses example guilds)
tui-demo: example-guilds
    @echo "🖥️  Launching TUI demo..."
    @TABLE=$(psql "${DATABASE_URL}" -t -c "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename LIKE 'guilds_gen%' ORDER BY tablename DESC LIMIT 1;" | tr -d ' ')
    @if [ -z "$$TABLE" ]; then \
        echo "❌ No guilds tables found. Run: just example-guilds"; \
        exit 1; \
    fi
    @echo "   Table: $$TABLE"
    @cargo run --all-features -- tui "$$TABLE"

# Full Workflow (CI/CD)
# ====================

# Run the complete CI pipeline locally
ci: fmt-check lint test-all audit
    @echo "✅ CI pipeline completed successfully!"

# Prepare for commit (format, lint, test)
pre-commit: fix-all test
    @echo "✅ Ready to commit!"

# Prepare for release (all checks + release build)
pre-release: ci build-release-all
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
    cargo doc --all-features --no-deps --open

# Check documentation for issues
docs-check:
    cargo doc --all-features --no-deps

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
    @echo "Lines of Rust code:"
    @find src -name '*.rs' -exec wc -l {} + | tail -1
    @echo ""
    @echo "Lines of test code:"
    @find tests -name '*.rs' -exec wc -l {} + 2>/dev/null | tail -1 || echo "  0 tests (no tests/ directory)"
    @echo ""
    @echo "Number of dependencies:"
    @grep -c "^name =" Cargo.lock
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
    @grep '^\[features\]' -A 20 Cargo.toml | grep -v '^\[' | grep '='

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
    cargo bench --all-features

# Aliases for common tasks
# ========================

alias b := build
alias t := test
alias l := lint
alias f := fmt
alias c := check-all
alias r := run
alias d := docs
