# Botticelli Observability Guide

## Overview

Botticelli includes a comprehensive observability system built on `tracing` and custom metrics to provide visibility into bot operations, narrative execution, and API usage.

## Architecture

### Tracing
- **Foundation**: Built on `tracing` crate with structured logging
- **Instrumentation**: All public functions use `#[instrument]` attribute
- **Levels**: trace, debug, info, warn, error
- **Spans**: Hierarchical context tracking through execution flows

### Metrics
- **Custom System**: Lightweight, thread-safe metrics without external dependencies
- **Types**: Counters, gauges, histograms
- **Storage**: In-memory atomic operations
- **Export**: JSON HTTP endpoint for external monitoring

## Configuration

### Environment Variables

```bash
# Tracing level (default: info)
export RUST_LOG=botticelli=debug,botticelli_narrative=trace

# Metrics server (default: disabled)
export BOTTICELLI_METRICS_PORT=9090
```

### botticelli.toml

```toml
[observability]
# Enable metrics collection
metrics_enabled = true

# Metrics HTTP server port
metrics_port = 9090

# Tracing configuration
[observability.tracing]
level = "info"
format = "pretty"  # or "json"
```

## Metrics

### Bot Metrics

**Generation Bot**
- `generation.runs.total` (counter) - Total generation cycles
- `generation.runs.success` (counter) - Successful cycles
- `generation.runs.failed` (counter) - Failed cycles
- `generation.posts.created` (counter) - Posts created
- `generation.duration.seconds` (histogram) - Cycle duration

**Curation Bot**
- `curation.runs.total` (counter) - Total curation cycles
- `curation.runs.success` (counter) - Successful cycles  
- `curation.runs.failed` (counter) - Failed cycles
- `curation.posts.evaluated` (counter) - Posts evaluated
- `curation.posts.approved` (counter) - Posts approved
- `curation.duration.seconds` (histogram) - Cycle duration

**Posting Bot**
- `posting.runs.total` (counter) - Total posting cycles
- `posting.runs.success` (counter) - Successful cycles
- `posting.runs.failed` (counter) - Failed cycles
- `posting.posts.sent` (counter) - Posts sent to Discord
- `posting.duration.seconds` (histogram) - Cycle duration

### Narrative Metrics

- `narrative.executions.total` (counter) - Total narrative executions
- `narrative.executions.success` (counter) - Successful executions
- `narrative.executions.failed` (counter) - Failed executions
- `narrative.duration.seconds` (histogram) - Execution duration
- `narrative.acts.processed` (counter) - Acts processed
- `narrative.json.extractions` (counter) - JSON extraction attempts
- `narrative.json.failures` (counter) - JSON parsing failures

### API Metrics

- `api.requests.total` (counter) - Total API requests
- `api.requests.success` (counter) - Successful requests
- `api.requests.failed` (counter) - Failed requests
- `api.tokens.input` (counter) - Input tokens consumed
- `api.tokens.output` (counter) - Output tokens generated
- `api.latency.seconds` (histogram) - Request latency
- `api.rate_limit.wait.seconds` (histogram) - Rate limit wait time

### Database Metrics

- `database.queries.total` (counter) - Total queries
- `database.queries.success` (counter) - Successful queries
- `database.queries.failed` (counter) - Failed queries
- `database.tables.created` (counter) - Tables created
- `database.rows.inserted` (counter) - Rows inserted
- `database.rows.deleted` (counter) - Rows deleted

## Usage

### Accessing Metrics

**HTTP Endpoint** (when metrics server enabled):
```bash
curl http://localhost:9090/metrics
```

**Response Format**:
```json
{
  "counters": {
    "generation.runs.total": 42,
    "generation.posts.created": 126
  },
  "gauges": {
    "system.memory.bytes": 1048576
  },
  "histograms": {
    "generation.duration.seconds": {
      "count": 42,
      "sum": 3600.5,
      "min": 45.2,
      "max": 120.8,
      "mean": 85.7
    }
  }
}
```

### Programmatic Access

```rust
use botticelli_interface::MetricsCollector;

let metrics = MetricsCollector::global();

// Increment counter
metrics.increment_counter("my.counter", 1);

// Set gauge
metrics.set_gauge("my.gauge", 42.0);

// Record histogram value
metrics.record_histogram("my.duration", 1.5);

// Get current values
let snapshot = metrics.snapshot();
println!("{}", serde_json::to_string_pretty(&snapshot)?);
```

### Tracing in Code

All public functions should be instrumented:

```rust
use tracing::{debug, error, info, instrument};

#[instrument(skip(connection), fields(table_name))]
pub async fn process_data(
    connection: &mut PgConnection,
    table_name: &str,
) -> Result<(), Error> {
    info!("Starting data processing");
    
    debug!(table = %table_name, "Loading data");
    let data = load_data(connection, table_name).await?;
    
    info!(count = data.len(), "Loaded records");
    
    match process_records(&data).await {
        Ok(result) => {
            info!(processed = result.count, "Processing complete");
            Ok(())
        }
        Err(e) => {
            error!(error = ?e, "Processing failed");
            Err(e)
        }
    }
}
```

## Monitoring Best Practices

### Key Metrics to Monitor

1. **Bot Health**
   - Success/failure rates
   - Cycle durations
   - Queue depths (posts pending in tables)

2. **API Usage**
   - Request rates approaching limits
   - Token consumption trends
   - Error rates by type

3. **Database Performance**
   - Query latencies
   - Connection pool usage
   - Failed query rates

4. **Narrative Execution**
   - JSON extraction failure rates
   - Act processing times
   - Table operation success rates

### Alert Thresholds (Recommended)

- **Critical**: Bot failure rate > 10%
- **Warning**: API error rate > 5%
- **Warning**: JSON extraction failure > 20%
- **Info**: Queue depth growing over 100 posts
- **Info**: API rate limit wait time > 60s

### Log Analysis

**Find JSON extraction failures**:
```bash
grep "JSON parsing failed" logs/*.log
```

**Track bot cycle times**:
```bash
grep "Bot cycle complete" logs/*.log | grep "duration_ms"
```

**Monitor rate limiting**:
```bash
grep "rate_limit" logs/*.log | grep "waiting"
```

## Troubleshooting

### High JSON Failure Rate

**Symptoms**: `narrative.json.failures` increasing rapidly

**Solutions**:
1. Check `max_tokens` in narrative configuration
2. Review JSON schema complexity
3. Examine recent prompt changes
4. Enable trace logging for extraction module

### Bot Cycles Taking Too Long

**Symptoms**: `*.duration.seconds` histograms showing high values

**Solutions**:
1. Check API latency metrics
2. Review narrative complexity (act count)
3. Check database query performance
4. Monitor rate limit wait times

### API Rate Limiting

**Symptoms**: Frequent `api.rate_limit.wait` events

**Solutions**:
1. Reduce bot cycle frequency
2. Lower batch sizes in narratives
3. Adjust budget multipliers in `botticelli.toml`
4. Consider upgrading API tier

## Future Enhancements

- [ ] OpenTelemetry export (traces, metrics, logs)
- [ ] Grafana/Prometheus integration
- [ ] Built-in dashboard UI
- [ ] Alert rule configuration
- [ ] Historical metric persistence
- [ ] Distributed tracing support

## See Also

- [Bot Server Deployment](BOT_SERVER_OBSERVABILITY_STRATEGY.md)
- [Testing Patterns](TESTING_PATTERNS.md)
- [Usage Tiers](USAGE_TIERS.md)
