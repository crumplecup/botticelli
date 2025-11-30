# Observability Stack for Botticelli

Complete observability solution with distributed tracing, metrics, and dashboards.

## Quick Links

- **🚀 Getting Started**: [QUICK_START_OBSERVABILITY.md](QUICK_START_OBSERVABILITY.md) - Step-by-step setup
- **📊 Summary**: [OBSERVABILITY_SUMMARY.md](OBSERVABILITY_SUMMARY.md) - What was fixed and why
- **📖 Full Guide**: [OBSERVABILITY_SETUP.md](OBSERVABILITY_SETUP.md) - Comprehensive documentation
- **🐛 Troubleshooting**: [OBSERVABILITY_METRICS_JAEGER_ISSUE.md](OBSERVABILITY_METRICS_JAEGER_ISSUE.md) - Why metrics need Prometheus

## TL;DR

```bash
# 1. Start observability stack
podman-compose -f docker-compose.jaeger-only.yml up -d

# 2. Configure bot server
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export PROMETHEUS_ENDPOINT=0.0.0.0:9464
set -a; source .env; set +a

# 3. Run bot server
cargo run --bin actor-server --release

# 4. Open dashboards
# - Grafana: http://localhost:3000 (admin/admin)
# - Jaeger: http://localhost:16686
# - Prometheus: http://localhost:9090
```

## Stack Components

| Service | Purpose | Port | UI |
|---------|---------|------|-----|
| **Jaeger** | Distributed tracing | 4317 (OTLP), 16686 (UI) | http://localhost:16686 |
| **Prometheus** | Metrics collection | 9090 | http://localhost:9090 |
| **Grafana** | Dashboards | 3000 | http://localhost:3000 |

## What You Get

### Traces (Jaeger)
- Distributed tracing across services
- Span-level details for every operation
- Request flow visualization
- Performance bottleneck identification

### Metrics (Prometheus)
- LLM API performance (error rates, latency, token usage)
- System health metrics
- Real-time alerting capabilities
- Historical data for trend analysis

### Dashboards (Grafana)
- **LLM API Health**: Error rates, response times, token costs
- **Bot Health**: System status, uptime, resource usage
- **Narrative Performance**: Execution times, success rates
- **Overview**: High-level metrics across all systems

## Architecture

```
                    Bot Server
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ↓               ↓               ↓
    Traces          Metrics         Logs
   (OTLP)          (HTTP)          (TBD)
        │               │               │
        ↓               ↓               ↓
    Jaeger        Prometheus         Loki
        │               │               │
        └───────────────┼───────────────┘
                        ↓
                    Grafana
                  (Dashboards)
```

## Key Features

### OpenTelemetry Integration
- **SDK Version**: 0.31
- **Protocol**: OTLP (OpenTelemetry Protocol)
- **Transport**: gRPC (Tonic)
- **Exporters**: OTLP (traces), Prometheus (metrics)

### Metrics Instrumentation
All LLM API calls automatically tracked:
- Request counts (by provider, model)
- Error rates (by type: rate_limit, auth, network, etc.)
- Response time distributions
- Token usage (prompt, completion, total)

### Automatic Tracing
Every public function instrumented with `#[instrument]`:
- Function entry/exit
- Parameter values
- Error propagation
- Performance timing

## Directory Structure

```
.
├── docker-compose.jaeger-only.yml   # Full observability stack
├── prometheus.yml                   # Prometheus scrape config
├── grafana/
│   ├── provisioning/
│   │   ├── datasources/
│   │   │   └── datasources.yml     # Auto-configure Prometheus + Jaeger
│   │   └── dashboards/
│   │       └── dashboards.yml       # Dashboard provisioning
│   └── dashboards/
│       ├── llm-api-health.json      # LLM metrics dashboard
│       ├── bot-health.json          # System health dashboard
│       ├── narrative-performance.json # Narrative execution metrics
│       └── botticelli-overview.json # Overview dashboard
└── crates/
    ├── botticelli/src/observability.rs  # Core OTEL setup
    └── botticelli_models/src/metrics.rs # LLM metrics
```

## Configuration

### Environment Variables

```bash
# Tracing (OTLP to Jaeger)
OTEL_EXPORTER=otlp                          # Use OTLP exporter
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317  # Jaeger OTLP endpoint

# Metrics (Prometheus)
PROMETHEUS_ENDPOINT=0.0.0.0:9464            # Expose metrics endpoint

# Logging
RUST_LOG=info                               # Log level
```

### Alternative: Stdout for Development

```bash
# Use stdout exporter (no external services needed)
export OTEL_EXPORTER=stdout
export PROMETHEUS_ENDPOINT=  # Empty = disable Prometheus server

# Metrics and traces printed to stdout (useful for debugging)
cargo run --bin actor-server
```

## Metrics Reference

### LLM Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_requests_total` | Counter | provider, model | Total API requests |
| `llm_errors_total` | Counter | provider, model, error_type | Failed requests |
| `llm_duration_seconds` | Histogram | provider, model | Response time distribution |
| `llm_tokens_total` | Counter | model | Total tokens used |
| `llm_tokens_prompt_total` | Counter | model | Prompt tokens |
| `llm_tokens_completion_total` | Counter | model | Completion tokens |

### Error Types

- `rate_limit` - API rate limits exceeded
- `auth` - Authentication/authorization failures
- `network` - Network connectivity issues
- `timeout` - Request timeouts
- `invalid_request` - Malformed requests
- `unknown` - Other errors

## Common Queries

### Prometheus/PromQL

```promql
# Error rate percentage
100 * (rate(llm_errors_total[5m]) / rate(llm_requests_total[5m]))

# Average response time
rate(llm_duration_seconds_sum[5m]) / rate(llm_duration_seconds_count[5m])

# P95 latency
histogram_quantile(0.95, rate(llm_duration_seconds_bucket[5m]))

# Tokens per minute
sum by (model) (rate(llm_tokens_total[1m]) * 60)

# Top error types
topk(5, sum by (error_type) (rate(llm_errors_total[5m])))
```

## Troubleshooting

### No metrics in Grafana?

1. **Check Prometheus targets**: http://localhost:9090/targets
   - `actor-server` should be UP (green)
   
2. **Test metrics endpoint**:
   ```bash
   curl http://localhost:9464/metrics | grep llm_requests
   ```

3. **Check bot server logs**:
   ```
   INFO: Prometheus metrics server listening on http://0.0.0.0:9464/metrics
   ```

### Podman networking issues?

Update `prometheus.yml` with your host IP:
```yaml
- job_name: 'actor-server'
  static_configs:
    - targets: ['192.168.1.x:9464']  # Your host IP
```

Restart Prometheus:
```bash
podman-compose -f docker-compose.jaeger-only.yml restart prometheus
```

### No traces in Jaeger?

1. **Verify OTLP endpoint is reachable**:
   ```bash
   nc -zv localhost 4317  # Should connect
   ```

2. **Check bot server environment**:
   ```bash
   echo $OTEL_EXPORTER                    # Should be: otlp
   echo $OTEL_EXPORTER_OTLP_ENDPOINT      # Should be: http://localhost:4317
   ```

3. **Check Jaeger logs**:
   ```bash
   podman logs botticelli-jaeger
   ```

## Best Practices

### Development
- Use `OTEL_EXPORTER=stdout` for local debugging
- Disable Prometheus endpoint if not needed
- Keep `RUST_LOG=debug` for detailed traces

### Production
- Use OTLP exporter with external collectors
- Enable Prometheus metrics
- Configure appropriate log levels (info/warn)
- Set up alerting rules in Grafana

### Performance
- Metrics have minimal overhead (counters/histograms)
- Traces use batch exporting (low latency impact)
- Prometheus scrape every 15s (configurable)

## Documentation

- [QUICK_START_OBSERVABILITY.md](QUICK_START_OBSERVABILITY.md) - Quick setup guide
- [OBSERVABILITY_SETUP.md](OBSERVABILITY_SETUP.md) - Complete setup documentation
- [OBSERVABILITY_SUMMARY.md](OBSERVABILITY_SUMMARY.md) - Implementation summary
- [OBSERVABILITY_METRICS_JAEGER_ISSUE.md](OBSERVABILITY_METRICS_JAEGER_ISSUE.md) - Jaeger + metrics explanation
- [OBSERVABILITY_DASHBOARDS.md](OBSERVABILITY_DASHBOARDS.md) - Dashboard design patterns
- [grafana/dashboards/README.md](grafana/dashboards/README.md) - Dashboard documentation

## Related

- [BOT_SERVER_OBSERVABILITY_STRATEGY.md](BOT_SERVER_OBSERVABILITY_STRATEGY.md) - Overall observability strategy
- [OPENTELEMETRY_INTEGRATION_ISSUES.md](OPENTELEMETRY_INTEGRATION_ISSUES.md) - Integration challenges

## Support

Issues? Check:
1. Container logs: `podman logs <container-name>`
2. Metrics endpoint: `curl localhost:9464/metrics`
3. Prometheus targets: http://localhost:9090/targets
4. Grafana data sources: Configuration > Data Sources

Still stuck? Review the troubleshooting guides in the documentation links above.
