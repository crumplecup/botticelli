# Quick Start: Full Observability Stack

## What You Get

- **Jaeger**: Distributed tracing (spans, traces)
- **Prometheus**: Metrics collection (counters, histograms, gauges)
- **Grafana**: Dashboards for both traces and metrics

## Start the Stack

### Using Podman (Recommended)

```bash
# Start observability stack
podman-compose -f docker-compose.jaeger-only.yml up -d

# Verify all containers are running
podman ps

# Should see:
# - botticelli-jaeger
# - botticelli-prometheus
# - botticelli-grafana
```

### Using Docker

```bash
# Start observability stack
docker-compose -f docker-compose.jaeger-only.yml up -d

# Verify
docker ps
```

## Start Your Bot Server

```bash
# Configure environment
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export PROMETHEUS_ENDPOINT=0.0.0.0:9464

# Load other env vars from .env
set -a; source .env; set +a

# Run bot server
cargo run --bin actor-server --release
```

## Access the UIs

### Jaeger (Traces)
- URL: http://localhost:16686
- What to see:
  - Service: Select "botticelli-actor-server"
  - Operations: LLM API calls, narrative executions
  - Traces: Click on any trace to see span details

### Prometheus (Metrics)
- URL: http://localhost:9090
- What to check:
  - Status > Targets: Verify "actor-server" is UP
  - Graph tab, try queries:
    - `llm_requests_total` - Total LLM requests
    - `llm_errors_total` - Failed requests
    - `rate(llm_requests_total[5m])` - Request rate

### Grafana (Dashboards)
- URL: http://localhost:3000
- Login: `admin` / `admin`
- Pre-configured dashboards:
  - **LLM API Health**: Error rates, latency, token usage
  - **Bot Health**: Overall system health
  - **Narrative Performance**: Execution times
  - **Botticelli Overview**: High-level metrics

## Verify Everything Works

### 1. Check Metrics Endpoint

```bash
curl http://localhost:9464/metrics | grep llm_requests
```

Should see output like:
```
# HELP llm_requests Total LLM API requests
# TYPE llm_requests counter
llm_requests{model="gemini-1.5-flash",provider="gemini"} 42
```

### 2. Check Prometheus Scraping

1. Open http://localhost:9090/targets
2. Find "actor-server" job
3. Status should be "UP" (green)
4. Last scrape should be recent (< 30s ago)

### 3. Check Grafana Data Sources

1. Open http://localhost:3000
2. Go to Configuration > Data Sources
3. Should see:
   - **Prometheus** (default) - Status: OK
   - **Jaeger** - Status: OK

### 4. View a Dashboard

1. In Grafana, go to Dashboards
2. Open "LLM API Health"
3. Select time range: Last 15 minutes
4. You should see:
   - Request counts
   - Error rates (% failed)
   - Response time graphs
   - Token usage

If no data appears:
- Ensure bot server is running
- Trigger some activity (run a command)
- Wait 15-30 seconds for metrics to be scraped
- Refresh dashboard

## Troubleshooting

### No Metrics in Grafana

**Check 1: Is Prometheus scraping the bot server?**
```bash
# From Prometheus UI (localhost:9090/targets)
# actor-server should show State: UP
```

If DOWN:
```bash
# Test metrics endpoint directly
curl http://localhost:9464/metrics

# If this works but Prometheus can't reach it:
# - Podman users: Verify host.containers.internal resolves
# - Docker users: Edit prometheus.yml, change to host.docker.internal
```

**Check 2: Is bot server exposing metrics?**
```bash
# Check bot server logs
# Should see on startup:
# INFO: Prometheus metrics server listening on http://0.0.0.0:9464/metrics
```

If not:
```bash
# Verify environment variable is set
echo $PROMETHEUS_ENDPOINT
# Should output: 0.0.0.0:9464
```

**Check 3: Is Grafana connected to Prometheus?**
```bash
# Grafana UI > Configuration > Data Sources > Prometheus
# Click "Test" button
# Should show: "Data source is working"
```

### Podman Host Networking

If Prometheus can't scrape bot server:

1. Check if `host.containers.internal` works:
   ```bash
   podman exec botticelli-prometheus ping host.containers.internal
   ```

2. If ping fails, get your host IP:
   ```bash
   ip addr show | grep 'inet ' | grep -v 127.0.0.1
   ```

3. Update `prometheus.yml`:
   ```yaml
   - job_name: 'actor-server'
     static_configs:
       - targets: ['YOUR_HOST_IP:9464']  # Use IP from step 2
   ```

4. Restart Prometheus:
   ```bash
   podman-compose -f docker-compose.jaeger-only.yml restart prometheus
   ```

### Dashboard Shows "No Data"

**Fix 1: Adjust time range**
- In Grafana dashboard, top-right corner
- Change from "Last 1h" to "Last 5 minutes"
- Click refresh

**Fix 2: Trigger activity**
```bash
# Generate some metrics by triggering bot activity
# Wait 15-30 seconds
# Refresh dashboard
```

**Fix 3: Check metric names**
- Grafana dashboard queries might use old metric names
- Go to Prometheus UI (localhost:9090)
- Click "Graph"
- Start typing `llm_` - autocomplete shows available metrics
- Verify metric names match what dashboard queries use

## Metrics Reference

### LLM API Metrics

All metrics include labels for `provider` and `model`.

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `llm_requests_total` | Counter | Total API requests | provider, model |
| `llm_errors_total` | Counter | Failed requests | provider, model, error_type |
| `llm_duration_seconds` | Histogram | API call latency | provider, model |
| `llm_tokens_total` | Counter | Total tokens | model |
| `llm_tokens_prompt_total` | Counter | Prompt tokens | model |
| `llm_tokens_completion_total` | Counter | Completion tokens | model |

### Useful Queries

**Error rate (percentage)**:
```promql
100 * (
  rate(llm_errors_total[5m]) 
  / 
  rate(llm_requests_total[5m])
)
```

**Average response time**:
```promql
rate(llm_duration_seconds_sum[5m]) 
/ 
rate(llm_duration_seconds_count[5m])
```

**P95 latency** (requires histogram):
```promql
histogram_quantile(0.95, rate(llm_duration_seconds_bucket[5m]))
```

**Tokens per minute by model**:
```promql
sum by (model) (rate(llm_tokens_total[1m]) * 60)
```

**Requests by provider**:
```promql
sum by (provider) (rate(llm_requests_total[5m]))
```

## Next Steps

1. ✅ Run bot server with metrics enabled
2. ✅ Verify Prometheus is scraping (`/targets` page)
3. ✅ Open Grafana dashboards
4. ✅ Trigger some bot activity
5. ✅ Watch metrics populate in real-time

## Stopping the Stack

```bash
# Podman
podman-compose -f docker-compose.jaeger-only.yml down

# Keep data volumes:
podman-compose -f docker-compose.jaeger-only.yml down

# Delete data volumes too:
podman-compose -f docker-compose.jaeger-only.yml down -v
```

## Architecture Summary

```
┌────────────────┐
│  Bot Server    │
│                │
│ - Emits traces │ ──OTLP:4317──▶ ┌─────────┐
│ - Emits metrics│ ──HTTP:9464──▶ │ Jaeger  │
└────────────────┘                 └─────────┘
                                        │
                                   (storage)
                                        ↓
                   ┌──────────────┐ ┌──────────┐
                   │ Prometheus   │ │ Jaeger   │
                   │ (scrapes     │ │ (stores  │
                   │  metrics)    │ │  traces) │
                   └──────┬───────┘ └────┬─────┘
                          │              │
                          └────┬─────────┘
                               ↓
                        ┌──────────┐
                        │ Grafana  │
                        │ (dashboards,│
                        │  queries)  │
                        └──────────┘
```

## Documentation

- **OBSERVABILITY_SETUP.md**: Detailed setup guide
- **OBSERVABILITY_METRICS_JAEGER_ISSUE.md**: Why Jaeger alone isn't enough
- **grafana/dashboards/README.md**: Dashboard documentation
- **OBSERVABILITY_DASHBOARDS.md**: Dashboard design patterns

## Support

If you see errors or unexpected behavior:

1. Check logs:
   ```bash
   # Bot server logs (in terminal where it's running)
   
   # Container logs
   podman logs botticelli-prometheus
   podman logs botticelli-grafana
   podman logs botticelli-jaeger
   ```

2. Verify network connectivity:
   ```bash
   # From inside prometheus container
   podman exec botticelli-prometheus wget -O- http://host.containers.internal:9464/metrics
   ```

3. Check configuration:
   ```bash
   # Verify prometheus config
   podman exec botticelli-prometheus cat /etc/prometheus/prometheus.yml
   ```
