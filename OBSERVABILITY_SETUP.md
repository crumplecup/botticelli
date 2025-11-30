# Observability Setup Guide

This guide shows how to set up distributed tracing, metrics collection, and dashboards for Botticelli using OpenTelemetry, Jaeger, Prometheus, and Grafana.

## Quick Start

### 1. Start the Observability Stack

**Recommended: Full Observability Stack** (Jaeger + Prometheus + Grafana):
```bash
# Using just recipes (recommended)
just obs-up

# Or directly with podman/docker
podman-compose -f docker-compose.observability.yml up -d
docker-compose -f docker-compose.observability.yml up -d
```

This starts:
- **Jaeger**: Distributed tracing UI on http://localhost:16686
- **Prometheus**: Metrics collection on http://localhost:9090
- **Grafana**: Visualization dashboards on http://localhost:3000 (admin/admin)

**Alternative: Jaeger Only** (if you already have PostgreSQL):
```bash
# Podman
podman-compose -f docker-compose.jaeger-only.yml up -d

# Docker
docker-compose -f docker-compose.jaeger-only.yml up -d
```

This starts:
- **Jaeger**: Distributed tracing UI on http://localhost:16686
- **PostgreSQL**: Database for bot state persistence (port 5433 to avoid conflicts)

### 2. Verify Stack Health

Wait 30-60 seconds for services to fully initialize, then run:
```bash
just test-observability
```

This comprehensive test validates:
- Container health (all services running)
- Service endpoints (HTTP APIs responding)
- Prometheus targets (metrics scraping configured)
- Grafana datasources (Prometheus + Jaeger connected)
- Grafana dashboards (3 dashboards provisioned)
- Metrics availability (ready to receive data)
- Trace ingestion (Jaeger collecting traces)

Expected output:
```
✓ Jaeger UI accessible at http://localhost:16686
✓ Prometheus has 2/2 targets UP
✓ Grafana datasources: 2 configured (Prometheus + Jaeger)
✓ Grafana dashboards: 3 found
⚠ Metric 'narrative_json_failures' not found (bot not running yet)
```

### 3. Configure Environment

Copy the example environment file:
```bash
cp .env.example .env
```

Edit `.env` and set:
```bash
# Enable OTLP exporter for traces + metrics
OTEL_EXPORTER=otlp

# Point to Jaeger collector
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Optional: Increase log verbosity
RUST_LOG=info,botticelli=debug
```

### 4. Build with Observability

```bash
# Build with OTLP support
cargo build --release --features otel-otlp

# Or for actor server
cargo build --release -p botticelli_actor --bin actor-server --features otel-otlp,discord
```

### 5. Run and View Data

Run your bot:
```bash
# Load environment variables
source .env

# Run actor server with observability
cargo run --release -p botticelli_actor --bin actor-server --features otel-otlp,discord
```

**View Traces in Jaeger:**
1. Open http://localhost:16686
2. Select "botticelli-actor-server" from the service dropdown
3. Click "Find Traces"
4. Explore your distributed traces!

**View Metrics in Prometheus:**
1. Open http://localhost:9090
2. Try queries like:
   - `rate(llm_api_requests_total[5m])` - API request rate
   - `llm_api_errors_total / llm_api_requests_total` - Error rate
   - `histogram_quantile(0.95, rate(llm_api_duration_seconds_bucket[5m]))` - 95th percentile latency

**View Dashboards in Grafana:**
1. Open http://localhost:3000 (login: admin/admin)
2. Navigate to **Dashboards** → **Botticelli** folder
3. Choose from 3 pre-configured dashboards:
   - **LLM API Performance** - Request rates, errors, latency, token usage
   - **Narrative Execution** - JSON parsing, act duration, execution flow
   - **Bot Pipeline** - Generation → Curation → Publishing flow

See [OBSERVABILITY_DASHBOARDS.md](OBSERVABILITY_DASHBOARDS.md) for dashboard details.

## Architecture

```
┌─────────────────────────────────────────────────┐
│         Botticelli Application                  │
│  (#[instrument] + metrics::counter!())          │
└──────────────┬──────────────────────────────────┘
               │ OpenTelemetry SDK
               │ (traces + metrics)
               ▼
┌──────────────────────────────────────────────────┐
│           OTLP Exporter (gRPC)                   │
└──────────────┬───────────────────────────────────┘
               │ Port 4317
               ▼
       ┌───────────────┐
       │               │
       ▼               ▼
┌─────────────┐  ┌─────────────┐
│   Jaeger    │  │ Prometheus  │
│ (Traces)    │  │ (Metrics)   │
│ Port 16686  │  │ Port 9090   │
└──────┬──────┘  └──────┬──────┘
       │                │
       │                │ Scrapes Jaeger metrics
       │                └─────────┐
       │                          │
       ▼                          ▼
┌──────────────────────────────────────┐
│         Grafana Dashboards           │
│  Jaeger Datasource + Prometheus      │
│            Port 3000                 │
└──────────────────────────────────────┘
```

**Data Flow:**
1. Application emits traces → OTLP → Jaeger
2. Application emits metrics → OTLP → Prometheus (via scraping)
3. Prometheus scrapes Jaeger's internal metrics
4. Grafana queries both Prometheus + Jaeger datasources
5. Pre-configured dashboards visualize everything

## Feature Flags

### observability (Base)
- Enables OpenTelemetry tracing
- Stdout exporter (human-readable, development)
- Metrics provider
- No external dependencies

```bash
cargo run --features observability
```

### otel-otlp (Production)
- Includes `observability` features
- Adds OTLP exporter (gRPC)
- Exports to Jaeger, SigNoz, Grafana Tempo, etc.
- Production-grade batch export

```bash
cargo run --features otel-otlp
```

## Environment Variables

### OTEL_EXPORTER
Controls which exporter backend to use.

- `stdout` (default): Human-readable traces to terminal
- `otlp`: Export to OTLP collector (requires `otel-otlp` feature)

### OTEL_EXPORTER_OTLP_ENDPOINT
OTLP collector endpoint (only used when `OTEL_EXPORTER=otlp`).

- Default: `http://localhost:4317`
- Docker/Podman: `http://localhost:4317`
- Kubernetes: `http://otel-collector:4317`
- Remote: `https://your-collector.example.com:4317`

### RUST_LOG
Controls log verbosity.

Examples:
```bash
# Info level for everything
RUST_LOG=info

# Debug level for Botticelli
RUST_LOG=debug

# Targeted debugging
RUST_LOG=info,botticelli_actor=debug,botticelli_narrative=trace
```

## Metrics

Botticelli automatically collects and exports these metrics when running with `OTEL_EXPORTER=otlp`:

### LLM API Metrics (NEW!)
- `llm_api_requests_total` - Total API requests by provider/model
- `llm_api_errors_total` - Total API errors by provider/model/error_type
- `llm_api_duration_seconds` - Request duration histogram (p50/p95/p99)
- `llm_api_tokens_total` - Token usage by provider/model/token_type

**Labels**: `provider` (gemini/claude/openai), `model` (gemini-2.0-flash-exp, etc.), `error_type` (timeout/rate_limit/invalid_request)

### Narrative Metrics
- `narrative_executions_total` - Narrative execution count
- `narrative_duration_seconds` - Narrative execution duration
- `narrative_act_duration_seconds` - Individual act duration
- `narrative_json_success_total` - JSON extraction successes
- `narrative_json_failures_total` - JSON extraction failures

**Labels**: `narrative_name`, `act_name`, `extraction_type`

### Bot Metrics
- `bot_executions_total` - Total bot executions
- `bot_failures_total` - Total bot failures  
- `bot_duration_seconds` - Bot execution duration (histogram)
- `bot_queue_depth` - Pending content in queue (gauge)
- `bot_time_since_success_seconds` - Time since last success (gauge)

**Labels**: `actor_name`, `skill_name`

### Pipeline Metrics
- `pipeline_generated_total` - Posts generated
- `pipeline_curated_total` - Posts curated
- `pipeline_published_total` - Posts published
- `pipeline_stage_latency_seconds` - Pipeline stage latency

**Labels**: `stage` (generation/curation/publishing)

All metrics include labels (tags) for filtering and aggregation in Prometheus/Grafana.

## Just Recipes

Convenient commands for managing the observability stack:

```bash
# Start the observability stack
just obs-up

# Stop the observability stack
just obs-down

# Restart the observability stack
just obs-restart

# View logs (all services)
just obs-logs

# View logs (specific service)
just obs-logs jaeger
just obs-logs prometheus
just obs-logs grafana

# Test observability integration
just test-observability
```

## Troubleshooting

### Quick Diagnostics

**Run the comprehensive test suite:**
```bash
just test-observability
```

This will check:
- All containers running
- Service endpoints accessible
- Prometheus scraping configured
- Grafana datasources connected
- Dashboards provisioned
- Metrics being collected

### Traces not appearing in Jaeger

1. **Check collector is running:**
   ```bash
   just obs-logs jaeger
   # or
   podman ps | grep jaeger
   docker ps | grep jaeger
   ```

2. **Verify OTLP endpoint is reachable:**
   ```bash
   curl http://localhost:4317
   # Should connect (may return error, but connection works)
   ```

3. **Check environment variables:**
   ```bash
   echo $OTEL_EXPORTER
   echo $OTEL_EXPORTER_OTLP_ENDPOINT
   ```

4. **Verify feature flag:**
   ```bash
   cargo run --features otel-otlp  # NOT just observability
   ```

5. **Check application logs:**
   ```bash
   RUST_LOG=debug cargo run --features otel-otlp
   # Look for "Observability initialized (OTEL_EXPORTER="otlp")" message
   ```

### Metrics not appearing in Prometheus

1. **Check Prometheus targets:**
   ```bash
   curl -s http://localhost:9090/api/v1/targets | jq
   # All targets should show "up"
   ```

2. **Check if bot is running:**
   ```bash
   # Metrics only appear when the bot is actively running
   ps aux | grep actor-server
   ```

3. **Query for any metrics:**
   ```bash
   curl -s http://localhost:9090/api/v1/label/__name__/values | jq
   # Should show llm_*, narrative_*, bot_* metrics
   ```

4. **Check Prometheus logs:**
   ```bash
   just obs-logs prometheus
   # Look for scraping errors
   ```

### Dashboards not appearing in Grafana

1. **Wait for provisioning:**
   ```bash
   # Grafana takes 30-60 seconds to provision on first start
   sleep 60
   just test-observability
   ```

2. **Check provisioning logs:**
   ```bash
   just obs-logs grafana | grep -i provision
   # Should show "Provisioning dashboards"
   ```

3. **Manually verify dashboards:**
   ```bash
   curl -u admin:admin http://localhost:3000/api/search?type=dash-db | jq
   # Should show 3 dashboards
   ```

4. **Check dashboard files:**
   ```bash
   ls -la grafana/dashboards/*.json
   # Should show 3 JSON files
   ```

### High memory usage

Batch exporter buffers spans before export. To reduce memory:

1. **Increase export frequency** (requires code change)
2. **Reduce span sampling** (future feature)
3. **Use stdout exporter** for development

### Slow performance

OpenTelemetry has minimal overhead (<5%), but:

1. **Use batch exporter** (default for OTLP)
2. **Avoid excessive `#[instrument]`** on tight loops
3. **Profile first** to confirm it's observability

### Connection refused errors

If you see "Connection refused" errors:

1. **Check port mapping:**
   ```bash
   podman port botticelli-jaeger
   # Should show: 4317/tcp -> 0.0.0.0:4317
   ```

2. **Check firewall:**
   ```bash
   sudo firewall-cmd --list-ports  # Should include 4317/tcp
   ```

3. **Try localhost vs 127.0.0.1:**
   ```bash
   # Sometimes one works better than the other
   OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
   ```

## Production Deployment

### Docker/Podman Container

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features otel-otlp -p botticelli_actor --bin actor-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/actor-server /usr/local/bin/
ENV OTEL_EXPORTER=otlp
ENV OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
CMD ["actor-server"]
```

### Kubernetes

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: botticelli-config
data:
  OTEL_EXPORTER: "otlp"
  OTEL_EXPORTER_OTLP_ENDPOINT: "http://otel-collector.observability.svc:4317"
  RUST_LOG: "info,botticelli=debug"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: botticelli-actor-server
spec:
  replicas: 1
  selector:
    matchLabels:
      app: botticelli
  template:
    metadata:
      labels:
        app: botticelli
    spec:
      containers:
      - name: actor-server
        image: botticelli-actor-server:latest
        envFrom:
        - configMapRef:
            name: botticelli-config
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: botticelli-secrets
              key: database-url
        - name: DISCORD_TOKEN
          valueFrom:
            secretKeyRef:
              name: botticelli-secrets
              key: discord-token
```

### Alternative Collectors

#### SigNoz
```bash
# Point to SigNoz collector
OTEL_EXPORTER_OTLP_ENDPOINT=http://signoz-collector:4317
```

#### Grafana Tempo
```bash
# Point to Tempo OTLP receiver
OTEL_EXPORTER_OTLP_ENDPOINT=http://tempo:4317
```

#### Honeycomb
```bash
# Honeycomb uses standard OTLP
OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io:443
# Add API key via headers (requires code changes for now)
```

## Advanced Configuration

### Custom Service Name

```rust
use botticelli::ObservabilityConfig;

let config = ObservabilityConfig::new("my-custom-service")
    .with_version("1.0.0")
    .with_metrics(true)
    .with_json_logs(true);

botticelli::init_observability_with_config(config)?;
```

### Disable Metrics

```rust
let config = ObservabilityConfig::new("my-service")
    .with_metrics(false);  // Only traces, no metrics

botticelli::init_observability_with_config(config)?;
```

### Programmatic Exporter Selection

```rust
use botticelli::{ObservabilityConfig, ExporterBackend};

let config = ObservabilityConfig::new("my-service")
    .with_exporter(ExporterBackend::Otlp {
        endpoint: "http://custom-collector:4317".to_string(),
    });

botticelli::init_observability_with_config(config)?;
```

## Performance Impact

OpenTelemetry has minimal overhead when properly configured:

- **Tracing**: <2% CPU overhead
- **Metrics**: <1% CPU overhead  
- **Memory**: ~50MB for batch buffers
- **Network**: Minimal (batched exports every 5 seconds)

**Best Practices:**
1. Use batch exporter (default for OTLP)
2. Avoid `#[instrument]` on tight loops
3. Use appropriate sampling in production
4. Monitor the observability stack itself

## Next Steps

1. **Explore Pre-Built Dashboards**
   - See [OBSERVABILITY_DASHBOARDS.md](OBSERVABILITY_DASHBOARDS.md) for dashboard guide
   - Learn how to read each panel
   - Understand key metrics and alerts

2. **Add Custom Metrics**
   - Use `metrics::counter!()` for event counts
   - Use `metrics::histogram!()` for latency tracking
   - Use `metrics::gauge!()` for point-in-time values
   - See [metrics crate docs](https://docs.rs/metrics/)

3. **Set Up Alerts**
   - Configure Grafana alerts for error rates
   - Set up notification channels (Slack/Discord/email)
   - Define SLOs (Service Level Objectives)
   - Monitor critical thresholds

4. **Production Deployment**
   - See Kubernetes/Docker sections below
   - Configure persistent storage for Prometheus
   - Set retention policies
   - Enable authentication

## See Also

- [OBSERVABILITY_DASHBOARDS.md](OBSERVABILITY_DASHBOARDS.md) - Dashboard guide and usage
- [OBSERVABILITY.md](OBSERVABILITY.md) - Observability strategy and design
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
