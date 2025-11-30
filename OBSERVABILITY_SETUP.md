# Observability Setup Guide

This guide shows how to set up distributed tracing and metrics collection for Botticelli using OpenTelemetry and Jaeger.

## Quick Start

### 1. Start the Observability Stack

Using Podman:
```bash
podman-compose up -d
```

Using Docker:
```bash
docker-compose up -d
```

This starts:
- **Jaeger**: Distributed tracing UI on http://localhost:16686
- **PostgreSQL**: Database for bot state persistence

### 2. Configure Environment

Copy the example environment file:
```bash
cp .env.example .env
```

Edit `.env` and set:
```bash
# Enable OTLP exporter
OTEL_EXPORTER=otlp

# Point to Jaeger collector
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Optional: Increase log verbosity
RUST_LOG=info,botticelli=debug
```

### 3. Build with Observability

```bash
# Build with OTLP support
cargo build --release --features otel-otlp

# Or for actor server
cargo build --release -p botticelli_actor --bin actor-server --features otel-otlp,discord
```

### 4. Run and View Traces

Run your bot:
```bash
# Load environment variables
source .env

# Run actor server with observability
cargo run --release -p botticelli_actor --bin actor-server --features otel-otlp,discord
```

View traces in Jaeger:
1. Open http://localhost:16686
2. Select "botticelli-actor-server" from the service dropdown
3. Click "Find Traces"
4. Explore your distributed traces!

## Architecture

```
┌─────────────────┐
│  Botticelli     │
│  Application    │
│  (with #[inst-  │
│   rument])      │
└────────┬────────┘
         │ OpenTelemetry SDK
         │ (traces + metrics)
         ▼
┌─────────────────┐
│  OTLP Exporter  │
│  (gRPC/Tonic)   │
└────────┬────────┘
         │ Port 4317
         ▼
┌─────────────────┐
│  Jaeger         │
│  Collector      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Jaeger UI      │
│  Port 16686     │
└─────────────────┘
```

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

Botticelli automatically collects and exports these metrics:

### Bot Metrics
- `bot.executions` - Total bot executions
- `bot.failures` - Total bot failures  
- `bot.duration` - Bot execution duration (histogram)
- `bot.queue_depth` - Pending content in queue (gauge)
- `bot.time_since_success` - Time since last success (gauge)

### Narrative Metrics
- `narrative.executions` - Narrative execution count
- `narrative.duration` - Narrative execution duration
- `narrative.act.duration` - Individual act duration
- `narrative.json.success` - JSON extraction successes
- `narrative.json.failures` - JSON extraction failures

### Pipeline Metrics
- `pipeline.generated` - Posts generated
- `pipeline.curated` - Posts curated
- `pipeline.published` - Posts published
- `pipeline.stage_latency` - Pipeline stage latency

All metrics include labels (tags) for filtering and aggregation.

## Troubleshooting

### Traces not appearing in Jaeger

1. **Check collector is running:**
   ```bash
   podman ps | grep jaeger
   # or
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
   # Look for "Observability initialized" message
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

## See Also

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [Botticelli Observability Design](OPENTELEMETRY_INTEGRATION_PLAN.md)
