# Observability Metrics + Jaeger Issue

## Problem

You've set up the bot server with OpenTelemetry and can see traces in Jaeger, but dashboards show no metrics data.

## Root Cause

**Jaeger does not support OpenTelemetry metrics.** Jaeger is a distributed tracing backend only.

- ✅ Jaeger supports: **Traces** (spans, distributed tracing)
- ❌ Jaeger does NOT support: **Metrics** (counters, histograms, gauges)

## What's Actually Happening

1. Bot server is correctly emitting metrics via OTLP
2. Metrics are being sent to `localhost:4317` (OTLP endpoint)
3. Jaeger receives the data but **silently ignores metrics**
4. Only traces are stored and displayed

## Solution: Add Prometheus for Metrics

You need a separate backend for metrics. The standard solution is **Prometheus + Grafana**:

```
Bot Server → OTLP Exporter → {
    Traces   → Jaeger     (existing)
    Metrics  → Prometheus (need to add)
}
Grafana → Queries both Jaeger + Prometheus
```

### Architecture

```
┌─────────────┐
│  Bot Server │
└──────┬──────┘
       │ OTLP (traces + metrics)
       ↓
┌──────────────────┐
│ OTEL Collector   │
├──────────────────┤
│ Receives OTLP    │
│ Splits data:     │
│  - Traces → Jaeger     │
│  - Metrics → Prometheus│
└──────────────────┘
       ↓       ↓
   ┌───────┐ ┌───────────┐
   │Jaeger │ │Prometheus │
   └───┬───┘ └─────┬─────┘
       │           │
       └───┬───────┘
           ↓
      ┌─────────┐
      │ Grafana │ ← Dashboards
      └─────────┘
```

## Implementation Steps

### Option 1: Prometheus Pull Model (Simpler)

Bot server exposes `/metrics` endpoint, Prometheus scrapes it.

**Already implemented!** Our code has Prometheus exporter built-in.

1. Update bot server config to use Prometheus endpoint:
   ```bash
   export PROMETHEUS_ENDPOINT="0.0.0.0:9464"
   ```

2. Add Prometheus to docker-compose:
   ```yaml
   prometheus:
     image: prom/prometheus:latest
     ports:
       - "9090:9090"
     volumes:
       - ./prometheus.yml:/etc/prometheus/prometheus.yml
     command:
       - '--config.file=/etc/prometheus/prometheus.yml'
   ```

3. Create `prometheus.yml`:
   ```yaml
   global:
     scrape_interval: 15s
   
   scrape_configs:
     - job_name: 'botticelli'
       static_configs:
         - targets: ['host.containers.internal:9464']  # For Podman
           # or: - targets: ['host.docker.internal:9464']  # For Docker
   ```

4. Configure Grafana data source:
   - Add Prometheus: `http://prometheus:9090`
   - Import dashboards

### Option 2: OTLP Push Model (More Complex)

Use OpenTelemetry Collector to route metrics.

1. Keep existing OTLP exporter config
2. Add OTEL Collector to docker-compose
3. Configure collector to:
   - Forward traces → Jaeger
   - Forward metrics → Prometheus (via remote write)

**Recommendation: Start with Option 1** (Prometheus pull). It's simpler and already implemented in the codebase.

## Quick Fix for Testing

Update your bot server startup:

```bash
# Keep Jaeger for traces
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Add Prometheus for metrics
export PROMETHEUS_ENDPOINT=0.0.0.0:9464

# Run bot server
cargo run --bin bot-server --release
```

Then in another terminal:
```bash
# Test metrics endpoint
curl http://localhost:9464/metrics
```

You should see Prometheus-formatted metrics output.

## Metrics Currently Tracked

From `crates/botticelli_models/src/metrics.rs`:

- `llm.requests` - Total API requests (by provider, model)
- `llm.errors` - Failed requests (by provider, model, error_type)
- `llm.duration` - API call latency histogram (seconds)
- `llm.tokens` - Total tokens used
- `llm.tokens.prompt` - Prompt tokens
- `llm.tokens.completion` - Completion tokens

## Grafana Dashboard Queries (Once Prometheus is Added)

### LLM Error Rate
```promql
rate(llm_errors_total[5m]) / rate(llm_requests_total[5m])
```

### Average Response Time
```promql
rate(llm_duration_sum[5m]) / rate(llm_duration_count[5m])
```

### Tokens per Minute
```promql
rate(llm_tokens_total[1m]) * 60
```

### Requests by Provider
```promql
sum by (provider) (rate(llm_requests_total[5m]))
```

## Next Steps

1. ✅ Verify metrics endpoint works: `curl localhost:9464/metrics`
2. Add Prometheus to `docker-compose.jaeger-only.yml`
3. Create `prometheus.yml` scrape config
4. Update Grafana to add Prometheus data source
5. Import/create dashboards with above queries

## Why This Wasn't Obvious

Common misconception: "OTLP collector = unified observability backend"

Reality: 
- OTLP is a *protocol* (transport format)
- Backends specialize: Jaeger (traces), Prometheus (metrics), Loki (logs)
- Collector acts as router, not storage
- Each signal type needs appropriate backend

Jaeger's documentation focuses on traces, doesn't explicitly state "we don't do metrics."

## Reference

- OpenTelemetry Collector: https://opentelemetry.io/docs/collector/
- Prometheus OTLP receiver: https://prometheus.io/docs/prometheus/latest/feature_flags/#otlp-receiver
- Our implementation: `crates/botticelli/src/observability.rs` (lines 256-320)
