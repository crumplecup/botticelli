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

# Run tests: just test [package] [test_name]
# Examples: 
#   just test                              # Run all tests with local features
#   just test botticelli                   # Run all tests for botticelli package
#   just test botticelli table_references  # Run specific test in botticelli
test PACKAGE="" TEST="":
    #!/usr/bin/env bash
    if [ -z "{{PACKAGE}}" ]; then
        # No package specified - run all tests with local features
        cargo test --workspace --features local --lib --tests
    elif [ -z "{{TEST}}" ]; then
        # Package specified, no test - run all tests for package
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{PACKAGE}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            cargo test --package {{PACKAGE}} --features local --lib --tests
        else
            cargo test --package {{PACKAGE}} --lib --tests
        fi
    else
        # Package and test specified - run specific test
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{PACKAGE}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            cargo test --package {{PACKAGE}} --features local --lib --tests {{TEST}} -- --nocapture
        else
            cargo test --package {{PACKAGE}} --lib --tests {{TEST}} -- --nocapture
        fi
    fi

# Run LOCAL tests with verbose output
test-verbose:
    cargo test --workspace --features local --lib --tests -- --nocapture

# Run doctests (usually fast)
test-doc:
    cargo test --workspace --features local --doc

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

# Quick test that metrics API is functional (no container/network required)
test-metrics:
    @echo "🔍 Testing metrics API..."
    cargo test --package botticelli_actor --test metrics_collection_test -- --nocapture

# Run API tests for Gemini (requires GEMINI_API_KEY)
test-api-gemini:
    #!/usr/bin/env bash
    set +u
    test -n "${GEMINI_API_KEY}" || (echo "❌ GEMINI_API_KEY not set. API tests require this environment variable." && exit 1)
    
    LOG_FILE="/tmp/botticelli-test-api-gemini.log"
    rm -f "$LOG_FILE"
    
    if cargo test --workspace --features gemini,api 2>&1 | tee "$LOG_FILE"; then
        if [ -s "$LOG_FILE" ] && grep -qE "^(warning:|error:|\s+\^|error\[|test result:.*FAILED)" "$LOG_FILE"; then
            echo "⚠️  API tests completed with warnings/errors. See: $LOG_FILE"
            exit 1
        else
            echo "✅ All API tests passed!"
            rm -f "$LOG_FILE"
        fi
    else
        echo "❌ API tests failed. See: $LOG_FILE"
        exit 1
    fi

# Run API tests for Anthropic (requires ANTHROPIC_API_KEY) - optional package and test name
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
    
    if cargo test $PACKAGE_FLAG --features local,api $TEST_NAME -- --nocapture --show-output 2>&1 | tee "$LOG_FILE"; then
        if [ -s "$LOG_FILE" ] && grep -qE "^(warning:|error:|\s+\^|error\[|test result:.*FAILED)" "$LOG_FILE"; then
            echo "⚠️  API tests completed with warnings/errors. See: $LOG_FILE"
            exit 1
        else
            echo "✅ All API tests passed!"
            rm -f "$LOG_FILE"
        fi
    else
        echo "❌ API tests failed. See: $LOG_FILE"
        exit 1
    fi

# Run ALL API tests (requires all API keys, expensive!)
test-api-all:
    #!/usr/bin/env bash
    set +u
    test -n "${GEMINI_API_KEY}" || (echo "⚠️  Warning: GEMINI_API_KEY not set" && exit 0)
    
    LOG_FILE="/tmp/botticelli-test-api-all.log"
    rm -f "$LOG_FILE"
    
    echo "🚀 Running all API tests (this will consume API quotas)..."
    if cargo test --workspace --all-features 2>&1 | tee "$LOG_FILE"; then
        if [ -s "$LOG_FILE" ] && grep -qE "^(warning:|error:|\s+\^|error\[|test result:.*FAILED)" "$LOG_FILE"; then
            echo "⚠️  API tests completed with warnings/errors. See: $LOG_FILE"
            exit 1
        else
            echo "✅ All API tests passed!"
            rm -f "$LOG_FILE"
        fi
    else
        echo "❌ API tests failed. See: $LOG_FILE"
        exit 1
    fi

# Run database tests (requires DATABASE_URL)
test-db:
    #!/usr/bin/env bash
    set +u
    test -n "${DATABASE_URL}" || (echo "❌ DATABASE_URL not set. Database tests require a PostgreSQL database." && exit 1)
    cargo test --workspace --features database

# Run local + doc tests (no linting)
test-full: test test-doc

# Run tests and show coverage (requires cargo-tarpaulin)
test-coverage:
    @command -v cargo-tarpaulin >/dev/null 2>&1 || (echo "Installing cargo-tarpaulin..." && cargo install cargo-tarpaulin)
    cargo tarpaulin --workspace --lib --tests --out Html --output-dir coverage

# Run complete test suite including API tests (for pre-merge)
test-pre-merge: test test-doc test-api-gemini

# Observability
# =============

# Test observability stack (Jaeger, Prometheus, Grafana)
test-observability:
    @./scripts/test-observability.sh

# Verify metrics pipeline is working (Application → Prometheus → Grafana)
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

# Run actor server with observability enabled (reads .env automatically)
run-actor-server:
    cargo run --bin actor-server --release --features "discord,otel-otlp"

# Container Management
# ===================

# Build the actor-server container image (alias: bot-build)
container-build:
    @echo "🐳 Building actor-server container..."
    podman build -t botticelli-actor-server:latest -f Containerfile .

# Alias for container-build
alias bot-build := container-build

# Run the actor-server container (requires .env file and observability stack) (alias: bot-run)
container-run:
    @echo "🚀 Starting actor-server container..."
    podman run -d \
        --name botticelli-actor-server \
        --env-file .env \
        -e OTEL_EXPORTER=otlp \
        -e OTEL_EXPORTER_OTLP_ENDPOINT=http://host.containers.internal:4318 \
        -p 9090:9090 \
        --network host \
        botticelli-actor-server:latest

# Alias for container-run
alias bot-run := container-run

# Start all services (observability + actor-server) with docker-compose
bot-up:
    @echo "🚀 Starting all Botticelli services..."
    podman-compose up -d

# Stop all services
bot-down:
    @echo "🛑 Stopping all Botticelli services..."
    podman-compose down

# Restart actor-server service only
bot-restart:
    @echo "🔄 Restarting actor-server..."
    podman-compose restart actor-server

# View actor-server logs
bot-logs:
    @echo "📋 Actor-server logs:"
    podman logs -f botticelli-actor-server

# Rebuild and restart actor-server
bot-rebuild: container-build
    @echo "🔄 Rebuilding and restarting actor-server..."
    podman-compose up -d --force-recreate actor-server

# Stop and remove the actor-server container
container-stop:
    @echo "🛑 Stopping actor-server container..."
    podman stop botticelli-actor-server || true
    podman rm botticelli-actor-server || true

# View actor-server container logs
container-logs:
    podman logs -f botticelli-actor-server

# Restart the actor-server container
container-restart: container-stop container-run

# Rebuild and restart the actor-server container
container-rebuild: container-build container-restart

# Complete container setup: build image and start with observability
container-setup: obs-up container-build container-run
    @echo "✅ Actor server container running with observability"
    @echo "📊 Grafana: http://localhost:3000"
    @echo "📈 Prometheus: http://localhost:9091"
    @echo "🔍 Jaeger: http://localhost:16686"
    @echo "📊 Metrics: http://localhost:9090/metrics"

# Chat Interface
# ==============

# Run chat interface locally for development
chat force="":
    @echo "💬 Starting chat interface..."
    {{ if force == "rebuild" { "cargo clean -p botticelli_tui" } else { "" } }}
    cargo run --package botticelli_tui --bin botticelli-chat --features "cli"

# Build the chat interface container image
chat-build:
    @echo "🐳 Building chat interface container..."
    podman build -t botticelli-chat:latest -f Containerfile.chat .

# Start the complete chat stack (postgres, mcp-server, chat interface)
chat-up:
    @echo "🚀 Starting chat interface stack..."
    podman-compose -f docker-compose.chat.yml up

# Start chat stack in background
chat-up-bg:
    @echo "🚀 Starting chat interface stack in background..."
    podman-compose -f docker-compose.chat.yml up -d

# Start chat stack with Jaeger for debugging
chat-up-debug:
    @echo "🚀 Starting chat interface stack with Jaeger..."
    podman-compose -f docker-compose.chat.yml --profile debug up

# Stop the chat stack
chat-down:
    @echo "🛑 Stopping chat interface stack..."
    podman-compose -f docker-compose.chat.yml down

# View chat logs
chat-logs service="":
    #!/usr/bin/env bash
    if [ -z "{{service}}" ]; then
        podman-compose -f docker-compose.chat.yml logs -f
    else
        podman logs -f botticelli-chat-{{service}}
    fi

# Rebuild and restart chat interface
chat-rebuild: chat-build
    @echo "🔄 Rebuilding and restarting chat interface..."
    podman-compose -f docker-compose.chat.yml up -d --force-recreate chat

# Build MCP HTTP server binary (required for auto-start)
build-mcp-server:
    @echo "🔧 Building MCP HTTP server..."
    cargo build --bin botticelli-mcp-http --features="http,database,llm"

# Run chat interface locally (skip health checks for development)
chat-local:
    @echo "💬 Starting chat interface locally..."
    cargo run --bin botticelli-chat --features="cli,tui" -- --skip-health-checks

# Run chat interface with health checks and auto-start (builds MCP server if needed)
chat-local-check: build-mcp-server
    @echo "💬 Starting chat interface with health checks and auto-start..."
    cargo run --bin botticelli-chat --features="cli,tui"

# Start services in background (for testing/automation)
chat-local-bg: build-mcp-server
    @echo "🔧 Starting services in background..."
    @echo "Note: Use 'pkill botticelli-chat' or 'pkill mcp-server' to stop"
    nohup cargo run --bin botticelli-chat --features="cli,tui" > /tmp/botticelli-chat.log 2>&1 &
    @echo "✅ Services starting... Check /tmp/botticelli-chat.log for output"

# Run automated demo with actor exercising all MCP tools
chat-demo: build-mcp-server
    @echo "🎬 Starting Botticelli Automated Demo..."
    cargo run --bin demo --features="demo"

# Run complete workflow demo (test mode)
demo-workflow-test:
    @echo "🎬 Running workflow demo in test mode..."
    cargo run --bin demo-workflow -- --test-mode

# Run complete workflow demo (with database)
# NOTE: Requires PostgreSQL and MCP server to be running.
# Run 'just chat-local' in another terminal first, or use 'just demo-workflow-auto'
demo-workflow: build-mcp-server
    @echo "🎬 Running workflow demo with database validation..."
    @echo "⚠️  Ensure 'just chat-local' is running in another terminal"
    cargo run --bin demo-workflow

# Auto-start services and run workflow demo
demo-workflow-auto:
    @echo "🎬 Starting services and running workflow demo..."
    @echo "📝 Note: This will start services in the background"
    just chat-local-bg
    @sleep 5
    just demo-workflow
    @echo "✅ Demo complete. Services still running in background."

# Run chat interface in container mode
chat-container:
    @echo "💬 Starting chat interface in container mode..."
    cargo run --bin botticelli-chat --features="cli,tui" -- --mode container --skip-health-checks

# Run chat interface with verbose logging
chat-verbose:
    @echo "💬 Starting chat interface with verbose logging..."
    cargo run --bin botticelli-chat --features="cli,tui" -- --skip-health-checks --verbose

# Complete chat setup: build and start all services
chat-setup: chat-build chat-up-bg
    @echo "✅ Chat interface running"
    @echo "💬 Chat: Attached to terminal"
    @echo "🗄️  PostgreSQL: localhost:5433"
    @echo "🔧 MCP Server: Running in background"

# Code Quality
# ============

# Check compilation (all features by default, or specific package)
check package="":
    #!/usr/bin/env bash
    if [ -z "{{package}}" ]; then
        echo "🔍 Checking all packages with all features..."
        cargo check --all-features
    else
        # Check if package has a 'local' feature, use it if available
        if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
           jq -e ".packages[] | select(.name == \"{{package}}\") | .features | has(\"local\")" >/dev/null 2>&1; then
            echo "🔍 Checking package: {{package}} with local features"
            cargo check -p "{{package}}" --features local
        else
            echo "🔍 Checking package: {{package}}"
            cargo check -p "{{package}}"
        fi
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
    #!/usr/bin/env bash
    set -e
    command -v cargo-hack >/dev/null 2>&1 || (echo "❌ cargo-hack not installed. Run: cargo install cargo-hack" && exit 1)
    
    LOG_FILE="/tmp/botticelli-check-features.log"
    rm -f "$LOG_FILE"
    
    # Run feature gate checks and capture output
    if ./scripts/feature-gate-check.sh 2>&1 | tee "$LOG_FILE"; then
        if [ -s "$LOG_FILE" ] && grep -qE "^(warning:|error:|\s+\^|error\[)" "$LOG_FILE"; then
            echo "⚠️  Feature gate checks completed with warnings/errors. See: $LOG_FILE"
            exit 1
        else
            echo "✅ All feature gate checks passed!"
            rm -f "$LOG_FILE"
        fi
    else
        echo "❌ Feature gate checks failed. See: $LOG_FILE"
        exit 1
    fi

# Run all checks (lint, format check, tests)
test-all package='':
    #!/usr/bin/env bash
    set -uo pipefail  # Removed -e so we can capture exit codes
    LOG_FILE="/tmp/botticelli_check_all.log"
    rm -f "$LOG_FILE"
    EXIT_CODE=0
    
    if [ -z "{{package}}" ]; then
        echo "🔍 Running all checks on entire workspace..."
        
        # Run fmt (errors only)
        cargo fmt --all
        
        # Run lint (show output and log warnings/errors)
        echo "🔍 Linting entire workspace with local features"
        if ! cargo clippy --workspace --features local --all-targets 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
        
        # Run tests (show output and log failures)
        if ! cargo test --workspace --features local --lib --tests 2>&1 | tee -a "$LOG_FILE"; then
            EXIT_CODE=1
        fi
        
        # Report results
        if [ $EXIT_CODE -ne 0 ]; then
            echo ""
            echo "⚠️  Checks completed with warnings/errors. Full log saved to: $LOG_FILE"
            exit 1
        else
            echo ""
            echo "✅ All checks passed!"
            rm -f "$LOG_FILE"
        fi
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

# Launch new Botticelli TUI with full MCP integration
tui:
    #!/usr/bin/env bash
    set +u

    # Check for API key
    if [ -z "${ANTHROPIC_API_KEY}" ]; then
        echo "❌ ANTHROPIC_API_KEY environment variable not set"
        echo ""
        echo "Please set your Anthropic API key:"
        echo "  export ANTHROPIC_API_KEY=sk-ant-..."
        echo ""
        echo "Or add it to your .env file in the project root"
        exit 1
    fi

    # Create narratives directory if it doesn't exist
    if [ ! -d "narratives" ]; then
        echo "📁 Creating narratives/ directory..."
        mkdir -p narratives
    fi

    # Show startup message
    echo "🚀 Starting Botticelli TUI..."
    echo ""
    echo "Features enabled:"
    echo "  💬 Chat with Claude (MCP tool integration)"
    echo "  📝 Narrative tools (create, validate, list, load)"
    echo "  🔧 5 tools available (echo, create_narrative, validate_narrative, list_narratives, load_narrative)"
    echo "  ⚡ Async tool execution with real-time UI updates"
    echo ""
    echo "Controls:"
    echo "  Tab       - Switch views (Chat → Narratives → Editor → Settings)"
    echo "  Shift+Tab - Cycle views backwards"
    echo "  Ctrl+Q    - Quit"
    echo "  Ctrl+C    - Quit"
    echo ""
    echo "Set RUST_LOG for debug output:"
    echo "  RUST_LOG=botticelli_tui=debug,botticelli_mcp_client=debug just tui"
    echo ""

    # Run with proper logging
    RUST_LOG="${RUST_LOG:-botticelli_tui=info,botticelli_mcp_client=info}" \
        cargo run --bin tui --features anthropic

# Launch chat TUI interface (old interface - for backward compatibility)
tui-chat:
    cargo run --bin botticelli-chat --features="cli,tui" -- --skip-health-checks

# Launch TUI with debug logging enabled
tui-debug:
    #!/usr/bin/env bash
    set +u

    if [ -z "${ANTHROPIC_API_KEY}" ]; then
        echo "❌ ANTHROPIC_API_KEY not set"
        exit 1
    fi

    mkdir -p narratives

    echo "🔍 Starting Botticelli TUI with debug logging..."
    echo ""
    RUST_LOG=botticelli_tui=debug,botticelli_mcp_client=debug,botticelli_mcp=debug \
        cargo run --bin tui --features anthropic

# Run TUI example with debug logging
tui-example:
    #!/usr/bin/env bash
    set +u

    if [ -z "${ANTHROPIC_API_KEY}" ]; then
        echo "❌ ANTHROPIC_API_KEY not set"
        exit 1
    fi

    mkdir -p narratives

    echo "🚀 Running TUI example..."
    echo ""
    RUST_LOG=botticelli_tui=debug,botticelli_mcp_client=debug \
        cargo run --example chat_with_mcp --features anthropic

# Launch TUI for a specific table
tui-table table:
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
pre-commit: fix-all check-features test-full
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
alias c := check
alias r := run
alias d := docs

# Run tests with timing information using nextest
test-timings:
    cargo nextest run --workspace --features local

# Install nextest if not present
install-nextest:
    cargo install cargo-nextest --locked

# Database Management
# ===================

# Export host database to SQL file
db-export output="botticelli_backup.sql":
    @echo "📦 Exporting host database to {{output}}..."
    pg_dump -U botticelli -h localhost -d botticelli -f {{output}}
    @echo "✅ Database exported"

# Import SQL file to host database
db-import input="botticelli_backup.sql":
    @echo "📥 Importing {{input}} to host database..."
    psql -U botticelli -h localhost -d botticelli -f {{input}}
    @echo "✅ Database imported"

# Snapshot container database to stdout
db-snapshot-container:
    @echo "📸 Creating container database snapshot..." >&2
    podman exec botticelli-bot-server pg_dump -U botticelli -d botticelli

# Restore snapshot to container from file
db-restore-container input="botticelli_backup.sql":
    @echo "♻️  Restoring {{input}} to container database..."
    cat {{input}} | podman exec -i botticelli-bot-server psql -U botticelli -d botticelli
    @echo "✅ Database restored"

# Sync host database to container
db-sync-to-container:
    @echo "🔄 Syncing host database to container..."
    @echo "⚠️  This will overwrite container database!"
    @read -p "Continue? [y/N] " -n 1 -r; \
    if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
        pg_dump -U botticelli -h localhost -d botticelli | \
            podman exec -i botticelli-bot-server psql -U botticelli -d botticelli; \
        echo "✅ Sync complete"; \
    else \
        echo "❌ Cancelled"; \
    fi

# Sync container database to host
db-sync-from-container:
    @echo "🔄 Syncing container database to host..."
    @echo "⚠️  This will overwrite host database!"
    @read -p "Continue? [y/N] " -n 1 -r; \
    if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
        podman exec botticelli-bot-server pg_dump -U botticelli -d botticelli | \
            psql -U botticelli -h localhost -d botticelli; \
        echo "✅ Sync complete"; \
    else \
        echo "❌ Cancelled"; \
    fi

# Compare row counts between host and container databases
db-compare:
    @echo "📊 Comparing databases..."
    @echo "\n🏠 Host database (localhost:5432):"
    @psql -U botticelli -h localhost -d botticelli -c "SELECT 'narrative_executions' as table, COUNT(*) FROM narrative_executions UNION ALL SELECT 'act_executions', COUNT(*) FROM act_executions UNION ALL SELECT 'media_references', COUNT(*) FROM media_references;"
    @echo "\n📦 Container database:"
    @podman exec botticelli-bot-server psql -U botticelli -d botticelli -c "SELECT 'narrative_executions' as table, COUNT(*) FROM narrative_executions UNION ALL SELECT 'act_executions', COUNT(*) FROM act_executions UNION ALL SELECT 'media_references', COUNT(*) FROM media_references;"

# Backup both host and container databases with timestamp
db-backup-all:
    #!/usr/bin/env bash
    timestamp=$(date +%Y%m%d_%H%M%S)
    echo "💾 Creating timestamped backups..."
    just db-export "backup_host_${timestamp}.sql"
    just db-snapshot-container > "backup_container_${timestamp}.sql"
    echo "✅ Backups created:"
    echo "   - backup_host_${timestamp}.sql"
    echo "   - backup_container_${timestamp}.sql"

# Test API-based features (expensive - use sparingly)
# Usage: just test-api [package]
