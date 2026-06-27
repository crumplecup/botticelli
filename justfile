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

# Install all development dependencies
setup:
    @just install-rust
    @just install-cargo-tools

# Install or update Rust toolchain
install-rust:
    rustup update stable
    rustup default stable
    rustup component add clippy rustfmt

# Install required cargo plugins
install-cargo-tools:
    cargo install cargo-audit || true
    cargo install cargo-watch || true
    cargo install cargo-hack || true
    cargo install cargo-dist || true
    cargo install omnibor-cli || true
    cargo install cargo-nextest || true

# Update just itself
update-just:
    cargo install just || true

# Update all dependencies (Rust, cargo tools, just)
update-all: install-rust install-cargo-tools update-just

# Building
# ========

# Build specific package or all workspace
build PACKAGE="":
    #!/usr/bin/env bash
    if [ -z "{{PACKAGE}}" ]; then
        cargo build --release
    else
        cargo build --release --package {{PACKAGE}}
    fi

# Build the workspace (debug, all features)
build-all:
    cargo build --all-features

# Build release with all features
build-release-all:
    cargo build --release --all-features

# Build an example for a specific package
build-example package example:
    cargo build --example {{example}} -p {{package}}

# Run an example for a specific package
run-example package example *args='':
    cargo run --example {{example}} -p {{package}} -- {{args}}

# Clean build artifacts
clean:
    cargo clean

# Clean and rebuild
rebuild: clean build

# Testing
# =======

# Run tests.  Args: PACKAGE (optional), TEST filter (optional).
test PACKAGE="" TEST="":
    #!/usr/bin/env bash
    if [ -z "{{PACKAGE}}" ]; then
        cargo test --workspace --all-features --lib --tests
    elif [ -z "{{TEST}}" ]; then
        cargo test --package {{PACKAGE}} --all-features --lib --tests
    else
        cargo test --package {{PACKAGE}} --all-features --lib --tests {{TEST}} -- --nocapture
    fi

# Run tests with verbose output
test-verbose:
    cargo test --workspace --all-features --lib --tests -- --nocapture

# Run doctests
test-doc:
    cargo test --workspace --doc

# Run tests for a specific package, optionally filtering by test name
test-package package test_name="":
    #!/usr/bin/env bash
    if [ -n "{{test_name}}" ]; then
        cargo test -p {{package}} --all-features --lib --tests {{test_name}} -- --nocapture
    else
        cargo test -p {{package}} --all-features --lib --tests
    fi

# Run API tests (requires ANTHROPIC_API_KEY; expensive — use sparingly).  Args: package, test_name.
test-api package="" test_name="":
    #!/usr/bin/env bash
    set +u

    LOG_FILE="/tmp/botticelli-test-api.log"
    rm -f "$LOG_FILE"

    PACKAGE_FLAG=""
    if [ -n "{{package}}" ]; then
        PACKAGE_FLAG="-p {{package}}"
    fi

    TEST_NAME=""
    if [ -n "{{test_name}}" ]; then
        TEST_NAME="{{test_name}}"
    fi

    if cargo test $PACKAGE_FLAG --features api $TEST_NAME -- --nocapture --show-output 2>&1 | tee "$LOG_FILE"; then
        if [ -s "$LOG_FILE" ] && grep -qE "^(warning:|error:|\s+\^|error\[|test result:.*FAILED)" "$LOG_FILE"; then
            echo "API tests completed with warnings/errors. See: $LOG_FILE"
            exit 1
        else
            echo "All API tests passed!"
            rm -f "$LOG_FILE"
        fi
    else
        echo "API tests failed. See: $LOG_FILE"
        exit 1
    fi

# Run tests with timing information using nextest
test-timings:
    cargo nextest run --workspace

# Install nextest if not present
install-nextest:
    cargo install cargo-nextest --locked

# Quick test that metrics API is functional
test-metrics:
    cargo test --package botticelli_actor --test metrics_collection_test -- --nocapture

# Run local + doc tests (no linting)
test-full: test test-doc

# Code Quality
# ============

# Check compilation (optionally for a specific package)
check package="":
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        cargo check --all-features --all-targets
    else
        cargo check -p "{{package}}" --all-features --all-targets
    fi

# Run clippy linter (optionally for a specific package)
lint package='':
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        cargo clippy --workspace --all-features --all-targets
    else
        cargo clippy -p {{package}} --all-features --all-targets
    fi

# Run clippy and fix issues automatically
lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Check code formatting
fmt-check:
    cargo fmt --all -- --check

# Format all code
fmt:
    cargo fmt --all

# Check markdown files for issues
lint-md:
    @command -v markdownlint-cli2 >/dev/null 2>&1 || (echo "markdownlint-cli2 not installed. Run: npm install -g markdownlint-cli2" && exit 1)
    markdownlint-cli2 "**/*.md" "#target" "#node_modules"

# Run clippy + fmt + tests for a package (full workspace is slow — prefer a package arg).
check-all package='':
    #!/usr/bin/env bash
    set -uo pipefail
    LOG_FILE="/tmp/botticelli_check_all.log"
    rm -f "$LOG_FILE"
    EXIT_CODE=0

    if [ -z "{{package}}" ]; then
        cargo fmt --all
        if ! cargo clippy --workspace --all-features --all-targets 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
        if ! cargo test --workspace --all-features --lib --tests 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
    else
        cargo fmt --all
        if ! cargo clippy -p {{package}} --all-features --all-targets 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
        if ! cargo test -p {{package}} --all-features --lib --tests 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
        cargo test -p {{package}} --all-features --doc 2>&1 | tee -a "$LOG_FILE" || EXIT_CODE=1
    fi

    if [ $EXIT_CODE -ne 0 ]; then
        echo ""
        echo "Checks completed with warnings/errors. Full log saved to: $LOG_FILE"
        exit 1
    else
        echo ""
        echo "All checks passed!"
        rm -f "$LOG_FILE"
    fi

# Test feature gate combinations with cargo-hack (mirrors CI workflow)
check-features:
    cargo check --workspace --no-default-features
    cargo check --workspace
    cargo check --workspace --all-features
    cargo hack check --workspace --feature-powerset --exclude-features api --optional-deps --depth 2
    cargo clippy --workspace --no-default-features -- -D warnings
    cargo clippy --workspace -- -D warnings
    cargo clippy --workspace --all-features -- -D warnings

# Fix all auto-fixable issues
fix-all: fmt lint-fix

# Security
# ========

# Check for security vulnerabilities in dependencies
audit:
    cargo audit

# Update dependencies and check for vulnerabilities
audit-fix:
    cargo update
    cargo audit

# Generate OmniBOR artifact tree for supply chain transparency
omnibor:
    @command -v omnibor >/dev/null 2>&1 || (echo "Installing omnibor-cli..." && cargo install omnibor-cli)
    omnibor --help > /dev/null && echo "OmniBOR installed" || echo "OmniBOR not found - install with: cargo install omnibor"

# Run all security checks
security: audit omnibor

# TUI (Terminal User Interface)
# ==============================

# Launch TUI operator console.
# provider: server (default) | gemini | anthropic | ollama
# model: leave blank to use provider default
tui provider="server" model="" server_url="":
    #!/usr/bin/env bash
    mkdir -p narratives
    MODEL_ARG=""
    SERVER_URL_ARG=""
    if [ -n "{{model}}" ]; then MODEL_ARG="--model {{model}}"; fi
    if [ -n "{{server_url}}" ]; then SERVER_URL_ARG="--server-url {{server_url}}"; fi
    RUST_LOG="${RUST_LOG:-botticelli_tui=info,botticelli_mcp_client=info}" \
        cargo run --package botticelli_tui --bin botticelli-tui -- \
        --provider {{provider}} $SERVER_URL_ARG $MODEL_ARG

# Launch TUI with debug logging. Same optional args as `tui`.
tui-debug provider="server" model="" server_url="":
    #!/usr/bin/env bash
    mkdir -p narratives
    MODEL_ARG=""
    SERVER_URL_ARG=""
    if [ -n "{{model}}" ]; then MODEL_ARG="--model {{model}}"; fi
    if [ -n "{{server_url}}" ]; then SERVER_URL_ARG="--server-url {{server_url}}"; fi
    RUST_LOG=botticelli_tui=debug,botticelli_server=debug,botticelli_mcp_client=debug,botticelli_mcp=debug,botticelli_models=debug \
        cargo run --package botticelli_tui --bin botticelli-tui -- \
        --provider {{provider}} $SERVER_URL_ARG $MODEL_ARG

# Bot Server Management
# ======================

# Start the bot server with all three bots (generation, curation, posting)
bot-server:
    cargo run --release --features bots --bin botticelli -- server

# Start only the generation bot (for testing)
bot-generate:
    cargo run --release --features bots --bin botticelli -- server --only generation

# Start only the curation bot (for testing)
bot-curate:
    cargo run --release --features bots --bin botticelli -- server --only curation

# Start only the posting bot (for testing)
bot-post:
    cargo run --release --features bots --bin botticelli -- server \
        --posting-narrative ./crates/botticelli_narrative/narratives/discord/posting.toml \
        --posting-name scheduled_post

# Actor Server
# ============

# Run actor server with observability enabled (reads .env automatically)
run-actor-server:
    cargo run --bin actor-server --release --package botticelli_actor --features "discord,otel-otlp"

# Container Management
# ===================

# Build the actor-server container image
container-build:
    podman build -t botticelli-actor-server:latest -f Containerfile .

alias bot-build := container-build

# Run the actor-server container (requires .env file and observability stack)
container-run:
    podman run -d \
        --name botticelli-actor-server \
        --env-file .env \
        -e OTEL_EXPORTER=otlp \
        -e OTEL_EXPORTER_OTLP_ENDPOINT=http://host.containers.internal:4318 \
        -p 9090:9090 \
        --network host \
        botticelli-actor-server:latest

alias bot-run := container-run

# Start all services with docker-compose
bot-up:
    podman-compose up -d

# Stop all services
bot-down:
    podman-compose down

# Restart actor-server service only
bot-restart:
    podman-compose restart actor-server

# View actor-server logs
bot-logs:
    podman logs -f botticelli-actor-server

# Rebuild and restart actor-server
bot-rebuild: container-build
    podman-compose up -d --force-recreate actor-server

# Stop and remove the actor-server container
container-stop:
    podman stop botticelli-actor-server || true
    podman rm botticelli-actor-server || true

# View actor-server container logs
container-logs:
    podman logs -f botticelli-actor-server

# Restart the actor-server container
container-restart: container-stop container-run

# Rebuild and restart the actor-server container
container-rebuild: container-build container-restart

# Observability
# =============

# Test observability stack (Jaeger, Prometheus, Grafana)
test-observability:
    @./scripts/test-observability.sh

# Verify metrics pipeline is working
verify-metrics:
    @./scripts/verify-metrics.sh

# Start observability stack
obs-up:
    podman-compose -f docker-compose.observability.yml up -d

# Stop observability stack
obs-down:
    podman-compose -f docker-compose.observability.yml down

# View observability logs
obs-logs service="":
    #!/usr/bin/env bash
    if [ -z "{{service}}" ]; then
        podman-compose -f docker-compose.observability.yml logs -f
    else
        podman logs -f botticelli-{{service}}
    fi

# Restart observability stack
obs-restart:
    podman-compose -f docker-compose.observability.yml restart

# Complete container setup: build image and start with observability
container-setup: obs-up container-build container-run
    @echo "Actor server container running with observability"
    @echo "  Grafana:    http://localhost:3000"
    @echo "  Prometheus: http://localhost:9091"
    @echo "  Jaeger:     http://localhost:16686"
    @echo "  Metrics:    http://localhost:9090/metrics"

# Narrative Execution
# ===================

# Execute a narrative by name (supports file.narrative syntax for multi-narrative files)
narrate PATTERN:
    #!/usr/bin/env bash
    set -e

    PATTERN="{{PATTERN}}"
    if [[ "$PATTERN" == *.* ]]; then
        FILE_PART="${PATTERN%.*}"
        NARRATIVE_NAME="${PATTERN##*.}"

        NARRATIVE_FILE=$(find ./crates/botticelli_narrative/narratives -type f -path "*/${FILE_PART}.toml" | head -1)

        if [ -z "$NARRATIVE_FILE" ]; then
            echo "No narrative file found matching '${FILE_PART}'"
            echo ""
            echo "Available narratives:"
            find crates/botticelli_narrative/narratives -type f -name "*.toml" 2>/dev/null | sed 's|crates/botticelli_narrative/narratives/||' | sed 's/\.toml$//' | sort || echo "  (no narratives directory)"
            exit 1
        fi

        STATE_DIR="${BOTTICELLI_STATE_DIR:-.narrative_state}"
        cargo run -p botticelli --release --features local -- run \
            --narrative "$NARRATIVE_FILE" \
            --narrative-name "${NARRATIVE_NAME}" \
            --save \
            --state-dir "$STATE_DIR" \
            --process-discord \
            --verbose
    else
        MATCHES=$(find ./crates/botticelli_narrative/narratives -type f -name "*.toml" | grep -i "{{PATTERN}}" | grep -v target | grep -v node_modules || true)

        if [ -z "$MATCHES" ]; then
            echo "No narrative found matching '{{PATTERN}}'"
            echo ""
            echo "Available narratives:"
            find crates/botticelli_narrative/narratives -type f -name "*.toml" 2>/dev/null | sed 's|crates/botticelli_narrative/narratives/||' | sed 's/\.toml$//' | sort || echo "  (no narratives directory)"
            exit 1
        fi

        COUNT=$(echo "$MATCHES" | wc -l)

        if [ "$COUNT" -eq 1 ]; then
            NARRATIVE="$MATCHES"
            STATE_DIR="${BOTTICELLI_STATE_DIR:-.narrative_state}"
            cargo run -p botticelli --release --features local -- run \
                --narrative "$NARRATIVE" \
                --save \
                --state-dir "$STATE_DIR" \
                --process-discord \
                --verbose
        else
            echo "Multiple narratives found matching '{{PATTERN}}':"
            echo "$MATCHES" | sed 's/^/  /'
            echo ""
            echo "Please be more specific with the name"
            exit 1
        fi
    fi

# Run example narrative: generate channel posts
example-channels:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_channel_posts.toml

# Run example narrative: generate users
example-users:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_users.toml

# Run example narrative: generate guilds
example-guilds:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds.toml

# Run example narrative: generate guilds (simplified)
example-guilds-simple:
    cargo run -p botticelli --release --features local -- run --narrative crates/botticelli_narrative/narratives/generate_guilds_simple.toml

# Content Management
# ==================

# List content from a generation table
content-list table:
    cargo run -p botticelli --release --features local -- content list {{table}}

# Show specific content item
content-show table id:
    cargo run -p botticelli --release --features local -- content show {{table}} {{id}}

# List all content generations with tracking metadata
content-generations:
    cargo run -p botticelli --release --features local -- content generations

# Show details of the last generation
content-last:
    cargo run -p botticelli --release --features local -- content last

# Development
# ===========

# Watch for changes and run tests
watch:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x 'test --workspace --lib --tests'

# Watch and run specific command on changes
watch-cmd cmd:
    @command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
    cargo watch -x '{{cmd}}'

# Run the binary in development mode
run *args:
    cargo run -p botticelli -- {{args}}

# Run with all features
run-all *args:
    cargo run -p botticelli --all-features -- {{args}}

# CI/CD
# =====

# Run the complete CI pipeline locally
ci: fmt-check lint check-features test-full audit

# Prepare for commit (format, lint, local tests, feature checks)
pre-commit: fix-all check-features test-full

# Prepare for merge (all checks including API tests)
pre-merge: pre-commit test-api

# Prepare for release (all checks + release build)
pre-release: ci build-release-all

# Git Helpers
# ===========

# Stage all changes and show status
stage:
    git add -A
    git status --short

# Documentation
# =============

# Generate and open Rust documentation
docs:
    cargo doc --workspace --no-deps --open

# Check documentation for issues
docs-check:
    cargo doc --workspace --no-deps

# Build and view documentation for a specific crate
docs-crate crate:
    cargo doc --package {{crate}} --no-deps --open

# Information
# ===========

# Show project statistics
stats:
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

# Show environment information
env:
    #!/usr/bin/env bash
    set +u
    rustc --version
    cargo --version
    just --version
    echo "ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY:+(set)}"
    echo "GEMINI_API_KEY:    ${GEMINI_API_KEY:+(set)}"

# Show available features in main crate
features:
    @grep '^\[features\]' -A 30 crates/botticelli/Cargo.toml | grep -v '^\[' | grep '='

# Utility
# =======

# Remove generated files and caches
clean-all: clean
    rm -rf target/
    rm -rf coverage/
    rm -f Cargo.lock

# Check for outdated dependencies
outdated:
    @command -v cargo-outdated >/dev/null 2>&1 || (echo "Installing cargo-outdated..." && cargo install cargo-outdated)
    cargo outdated

# Update dependencies to latest compatible versions
update-deps:
    cargo update

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

# Benchmarking
# ============

# Run benchmarks
bench:
    cargo bench

# Aliases
# =======

alias b := build
alias t := test
alias l := lint
alias f := fmt
alias c := check
alias r := run
alias d := docs
