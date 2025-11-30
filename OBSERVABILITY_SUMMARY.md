# Observability Stack - Setup Complete ✅

## What Was the Issue?

You had Jaeger running and receiving data from the bot server, but **Jaeger doesn't support metrics** - it only handles traces. The dashboards couldn't show LLM error rates, token usage, etc. because that data wasn't being stored anywhere Grafana could query it.

## Solution Implemented

Added **Prometheus** for metrics collection alongside Jaeger for traces.

### Architecture

```
Bot Server
  ├─ Traces  → OTLP (port 4317) → Jaeger
  └─ Metrics → HTTP (port 9464) → Prometheus
                                       ↓
                                   Grafana
                                  (queries both)
```

## Files Updated

1. **docker-compose.jaeger-only.yml**
   - Added Prometheus service
   - Added Grafana service (was missing)
   - Added persistent volumes for data

2. **prometheus.yml**
   - Updated target from `host.docker.internal` to `host.containers.internal` (Podman)
   - Already had correct scrape config for actor-server

3. **New Documentation**
   - `QUICK_START_OBSERVABILITY.md` - Step-by-step startup guide
   - `OBSERVABILITY_METRICS_JAEGER_ISSUE.md` - Root cause explanation

## What's Already Working

✅ Bot server emits traces via OTLP  
✅ Bot server exposes Prometheus metrics endpoint  
✅ Metrics instrumentation in code (`LlmMetrics`)  
✅ Grafana dashboards created (4 dashboards in `grafana/dashboards/`)  
✅ Grafana data source provisioning configured  

## What You Need to Do

### 1. Restart the Observability Stack

```bash
# Stop current Jaeger-only setup
podman-compose -f docker-compose.jaeger-only.yml down

# Start full stack (Jaeger + Prometheus + Grafana)
podman-compose -f docker-compose.jaeger-only.yml up -d

# Verify all 3 containers are running
podman ps
```

### 2. Start Bot Server with Metrics Enabled

```bash
# Set observability environment variables
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export PROMETHEUS_ENDPOINT=0.0.0.0:9464

# Load other variables from .env
set -a; source .env; set +a

# Run bot server
cargo run --bin actor-server --release
```

Look for this log line:
```
INFO: Prometheus metrics server listening on http://0.0.0.0:9464/metrics
```

### 3. Verify Metrics are Being Scraped

```bash
# Test metrics endpoint directly
curl http://localhost:9464/metrics | head -20

# Check Prometheus targets (should show actor-server as UP)
# Open browser: http://localhost:9090/targets
```

### 4. Open Grafana Dashboards

1. Go to http://localhost:3000
2. Login: `admin` / `admin`
3. Navigate to Dashboards
4. Open "LLM API Health" dashboard
5. Set time range to "Last 15 minutes"
6. Trigger some bot activity
7. Watch metrics populate!

## Available Dashboards

Located in `grafana/dashboards/`:

1. **llm-api-health.json** - Error rates, latency, token usage by provider
2. **bot-health.json** - Overall system health metrics
3. **narrative-performance.json** - Narrative execution performance
4. **botticelli-overview.json** - High-level overview

## Metrics Being Tracked

From `crates/botticelli_models/src/metrics.rs`:

- `llm_requests_total` - Total API requests (by provider, model)
- `llm_errors_total` - Failed requests (by provider, model, error_type)
- `llm_duration_seconds` - Response time histogram
- `llm_tokens_total` - Total tokens used
- `llm_tokens_prompt_total` - Prompt tokens
- `llm_tokens_completion_total` - Completion tokens

Error types tracked: rate_limit, auth, network, timeout, invalid_request, unknown

## Troubleshooting

### "No data" in Grafana

**Check Prometheus is scraping:**
```bash
# Prometheus UI: http://localhost:9090/targets
# actor-server should be UP (green)
```

**If DOWN, test connectivity:**
```bash
# From inside Prometheus container
podman exec botticelli-prometheus wget -O- http://host.containers.internal:9464/metrics
```

**If that fails, use host IP instead:**
```bash
# Get your host IP
ip addr show | grep 'inet ' | grep -v 127.0.0.1

# Edit prometheus.yml, replace host.containers.internal with your IP
# Restart: podman-compose -f docker-compose.jaeger-only.yml restart prometheus
```

### Bot server not exposing metrics

Check environment variable:
```bash
echo $PROMETHEUS_ENDPOINT
# Should output: 0.0.0.0:9464
```

Check bot server logs for:
```
INFO: Prometheus metrics server listening on http://0.0.0.0:9464/metrics
```

## Next Steps

1. ✅ Restart observability stack with Prometheus + Grafana
2. ✅ Start bot server with `PROMETHEUS_ENDPOINT=0.0.0.0:9464`
3. ✅ Verify Prometheus scraping works
4. ✅ Open Grafana dashboards
5. ⏭️  Run bot server for a while, collect metrics
6. ⏭️  Analyze error rates, latency patterns
7. ⏭️  Customize dashboards based on your needs

## Documentation Index

- **QUICK_START_OBSERVABILITY.md** - Quick setup guide (START HERE)
- **OBSERVABILITY_SETUP.md** - Comprehensive setup documentation
- **OBSERVABILITY_METRICS_JAEGER_ISSUE.md** - Why Jaeger alone wasn't enough
- **OBSERVABILITY_DASHBOARDS.md** - Dashboard design patterns
- **grafana/dashboards/README.md** - Dashboard documentation

## Key Takeaways

1. **Jaeger = Traces only** - Distributed tracing, span analysis
2. **Prometheus = Metrics** - Counters, gauges, histograms
3. **Grafana = Visualization** - Queries both backends for dashboards
4. **OTLP is a protocol**, not a storage backend
5. Metrics require explicit endpoint (`PROMETHEUS_ENDPOINT`)
6. Prometheus **pulls** metrics (scrapes HTTP endpoint)
7. Jaeger **receives** traces (pushed via OTLP)

## Success Criteria

You'll know it's working when:

✅ `podman ps` shows 3 containers (jaeger, prometheus, grafana)  
✅ `curl localhost:9464/metrics` returns Prometheus data  
✅ Prometheus UI (localhost:9090/targets) shows actor-server UP  
✅ Grafana dashboards show non-zero metrics  
✅ Dashboards update in real-time as bot server runs  

## Current Status

🔧 **Infrastructure**: Complete and ready  
📊 **Dashboards**: Created (4 dashboards)  
📈 **Metrics**: Instrumented in code  
🎯 **Next**: Start the stack and verify data flow  

---

**tl;dr**: Restart observability stack with new `docker-compose.jaeger-only.yml`, set `PROMETHEUS_ENDPOINT=0.0.0.0:9464`, run bot server, open http://localhost:3000, watch dashboards populate.
