# Botticelli Observability Dashboards

Complete guide to monitoring Botticelli with traces, metrics, and dashboards.

## Quick Start

### 1. Start the Full Observability Stack

```bash
# With Podman
podman-compose -f docker-compose.observability.yml up -d

# With Docker
docker-compose -f docker-compose.observability.yml up -d
```

This starts:
- **Jaeger** (traces): http://localhost:16686
- **Prometheus** (metrics): http://localhost:9090
- **Grafana** (dashboards): http://localhost:3000
- **PostgreSQL** (database): localhost:5433

### 2. Access Grafana

1. Open http://localhost:3000
2. Login: `admin` / `admin` (change on first login)
3. Navigate to **Dashboards** → **Botticelli Overview**

### 3. Run Your Bot with Tracing

```bash
# Bot server
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
cargo run --release -p botticelli_server --bin bot-server --features otel-otlp

# Actor server
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
cargo run --release -p botticelli_actor --bin actor-server --features otel-otlp,discord
```

Traces will flow to Jaeger, and you'll see them in Grafana!

## What You Get Out of the Box

### Trace Visualization (Jaeger)
- Request traces with timing breakdowns
- Service dependency maps
- Error traces highlighted

### Basic Metrics (Jaeger-derived)
- Trace ingestion rate
- Error rate (from trace spans)
- Service health

### Starter Dashboard (Grafana)
- Trace error rate gauge
- Traces received rate graph
- Recent traces panel

## Adding Application Metrics

The starter setup uses **trace-derived metrics** from Jaeger. For the specific metrics you want (LLM API failures, JSON parsing failures), you need to add **custom metrics** to your code.

### Good News: You Already Have Metrics Infrastructure! ✅

Your codebase already has:
- ✅ OpenTelemetry v0.31 configured in `crates/botticelli/src/observability.rs`
- ✅ Metrics collection in `crates/botticelli_server/src/metrics.rs`
- ✅ Automatic OTLP export when `OTEL_EXPORTER=otlp`

**Current metrics you already collect**:
- `narrative.json.success` - JSON extraction successes
- `narrative.json.failures` - JSON extraction failures
- `narrative.executions` - Narrative execution count
- `narrative.duration` - Narrative execution duration
- `bot.executions` - Bot execution count
- `bot.failures` - Bot failure count

### Step 1: Verify Metrics Export is Enabled

Your observability is already initialized! Just make sure metrics are enabled:

```rust
// In your main.rs or server initialization:
use botticelli::observability::{init_observability_with_config, ObservabilityConfig, ExporterBackend};

// Initialize with OTLP and metrics enabled
let config = ObservabilityConfig::new("bot-server")
    .with_exporter(ExporterBackend::Otlp {
        endpoint: "http://localhost:4317".to_string(),
    })
    .with_metrics(true);  // This is already the default!

init_observability_with_config(config)?;
```

Or just use environment variables (easier):

```bash
export OTEL_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --features otel-otlp
```

### Step 2: Add LLM API Metrics

The one missing piece is LLM API call tracking. Add to `botticelli_models`:

Create `crates/botticelli_models/src/metrics.rs`:

```rust
//! Metrics for LLM API calls.

use opentelemetry::{global, metrics::{Counter, Histogram, Meter}, KeyValue};
use std::sync::OnceLock;

static METRICS: OnceLock<LlmMetrics> = OnceLock::new();

/// Metrics for LLM API interactions.
pub struct LlmMetrics {
    _meter: Meter,
    pub requests: Counter<u64>,
    pub errors: Counter<u64>,
    pub duration: Histogram<f64>,
    pub tokens_used: Counter<u64>,
}

impl LlmMetrics {
    fn init() -> Self {
        let meter = global::meter("botticelli_llm");
        
        Self {
            _meter: meter.clone(),
            requests: meter
                .u64_counter("llm.requests")
                .with_description("Total LLM API requests")
                .build(),
            errors: meter
                .u64_counter("llm.errors")
                .with_description("Failed LLM API requests")
                .build(),
            duration: meter
                .f64_histogram("llm.duration")
                .with_unit("seconds")
                .with_description("LLM API call duration")
                .build(),
            tokens_used: meter
                .u64_counter("llm.tokens")
                .with_description("Total tokens used")
                .build(),
        }
    }
    
    pub fn get() -> &'static Self {
        METRICS.get_or_init(Self::init)
    }
}
```

### Step 3: Instrument LLM Calls

Add to `crates/botticelli_models/src/lib.rs`:

```rust
mod metrics;
pub use metrics::LlmMetrics;
```

In your Gemini client `generate()` method:

```rust
use crate::LlmMetrics;

pub async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse> {
    let metrics = LlmMetrics::get();
    let start = std::time::Instant::now();
    
    let labels = &[
        KeyValue::new("model", request.model().to_string()),
        KeyValue::new("provider", "gemini"),
    ];
    
    metrics.requests.add(1, labels);
    
    let result = self.generate_impl(request).await;
    
    let duration = start.elapsed().as_secs_f64();
    metrics.duration.record(duration, labels);
    
    match &result {
        Ok(response) => {
            // Track token usage if available
            if let Some(usage) = response.usage() {
                metrics.tokens_used.add(
                    usage.total_tokens as u64,
                    &[KeyValue::new("model", request.model().to_string())]
                );
            }
        }
        Err(e) => {
            let error_labels = &[
                KeyValue::new("model", request.model().to_string()),
                KeyValue::new("provider", "gemini"),
                KeyValue::new("error_type", classify_error(e)),
            ];
            metrics.errors.add(1, error_labels);
        }
    }
    
    result
}

fn classify_error(e: &Error) -> String {
    // Classify error: "rate_limit", "auth", "network", "unknown"
    // Based on error type
    "unknown".to_string()  // Implement based on your error types
}
```

### Step 4: JSON Parsing Metrics (Already Done!)

Good news: JSON parsing metrics are **already implemented** in `botticelli_server/src/metrics.rs`:

```rust
// Already exists!
pub fn record_json_extraction(&self, narrative_name: &str, success: bool) {
    let labels = &[KeyValue::new("narrative_name", narrative_name.to_string())];
    if success {
        self.json_success.add(1, labels);
    } else {
        self.json_failures.add(1, labels);
    }
}
```

Just make sure you're calling it in your extraction code!

### Step 5: Verify Metrics are Flowing

Your observability setup **already handles metrics export**! The code in `observability.rs` does this:

```rust
// This already exists in your codebase!
fn init_metrics(resource: &Resource, exporter: &ExporterBackend) -> Result<...> {
    match exporter {
        ExporterBackend::Otlp { endpoint } => {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.clone())
                .build()?;

            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
                .build();

            let meter_provider = SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource.clone())
                .build();

            global::set_meter_provider(meter_provider);
        }
        // ... stdout exporter for dev
    }
}
```

**To verify metrics are working:**

1. Start the observability stack:
   ```bash
   podman-compose -f docker-compose.observability.yml up -d
   ```

2. Run your bot with OTLP export:
   ```bash
   OTEL_EXPORTER=otlp cargo run --release --features otel-otlp
   ```

3. Check Prometheus targets:
   - Visit http://localhost:9090/targets
   - Should see `jaeger` target UP

4. Query metrics in Prometheus:
   - Visit http://localhost:9090/graph
   - Try query: `narrative_json_failures`
   - Or: `bot_executions`

### Step 6: Create Dashboards with Your Actual Metrics

Once metrics are flowing, create dashboards in Grafana using **your actual metric names**:

**LLM API Health Dashboard**:
```promql
# Error rate (after you add LlmMetrics)
rate(llm_errors[5m]) / rate(llm_requests[5m]) * 100

# Request rate by model
sum(rate(llm_requests[1m])) by (model)

# P95 latency
histogram_quantile(0.95, rate(llm_duration_bucket[5m]))

# Token usage by model
sum(rate(llm_tokens[1m])) by (model)
```

**JSON Parsing Dashboard** (using existing metrics):
```promql
# Failure rate
rate(narrative_json_failures[5m]) / (rate(narrative_json_success[5m]) + rate(narrative_json_failures[5m])) * 100

# Failure count by narrative
sum(rate(narrative_json_failures[1m])) by (narrative_name)

# Success count by narrative
sum(rate(narrative_json_success[1m])) by (narrative_name)
```

**Narrative Performance Dashboard** (using existing metrics):
```promql
# Execution time P95 by narrative
histogram_quantile(0.95, rate(narrative_duration_bucket[5m]))

# Success rate
sum(rate(narrative_executions{success="true"}[5m])) / sum(rate(narrative_executions[5m])) * 100

# Act duration P95
histogram_quantile(0.95, rate(narrative_act_duration_bucket[5m]))
```

**Bot Health Dashboard** (using existing metrics):
```promql
# Bot failure rate
rate(bot_failures[5m]) / rate(bot_executions[5m]) * 100

# Bot execution rate by type
sum(rate(bot_executions[1m])) by (bot_type)

# Queue depth
bot_queue_depth

# Time since last success
bot_time_since_success
```

**Pipeline Dashboard** (using existing metrics):
```promql
# Content generation rate
rate(pipeline_generated[5m])

# Content publication rate
rate(pipeline_published[5m])

# Pipeline stage latency P95
histogram_quantile(0.95, rate(pipeline_stage_latency_bucket[5m]))
```

## Dashboard Examples

### Creating a Custom Dashboard

1. Go to http://localhost:3000
2. Click **Dashboards** → **New** → **New Dashboard**
3. Click **Add visualization**
4. Select **Prometheus** data source
5. Enter PromQL query (see examples above)
6. Configure visualization type (Graph, Gauge, Stat, etc.)
7. Click **Save**

### Example: LLM Error Rate Panel

```json
{
  "title": "LLM API Error Rate",
  "targets": [
    {
      "expr": "(rate(llm_errors_total[5m]) / rate(llm_requests_total[5m])) * 100",
      "legendFormat": "{{model}}"
    }
  ],
  "type": "timeseries",
  "fieldConfig": {
    "defaults": {
      "unit": "percent",
      "color": { "mode": "thresholds" },
      "thresholds": {
        "steps": [
          { "value": 0, "color": "green" },
          { "value": 5, "color": "yellow" },
          { "value": 10, "color": "red" }
        ]
      }
    }
  }
}
```

## Alerting

### Set Up Alerts in Grafana

1. Create alert rules based on metrics
2. Example: Alert when LLM error rate > 10%

```yaml
# Alert condition
expr: (rate(llm_errors_total[5m]) / rate(llm_requests_total[5m])) * 100 > 10

# For: 5m (sustained for 5 minutes)
```

3. Configure notification channels (email, Slack, Discord)

## What You Already Have vs What's Missing

**✅ Already Implemented** (in your codebase):
- ✅ OpenTelemetry v0.31 observability infrastructure
- ✅ Automatic OTLP export (traces + metrics)
- ✅ JSON parsing metrics (`narrative.json.success/failures`)
- ✅ Narrative execution metrics (`narrative.executions/duration`)
- ✅ Bot execution metrics (`bot.executions/failures/duration`)
- ✅ Pipeline metrics (`pipeline.generated/curated/published`)
- ✅ Queue depth tracking (`bot.queue_depth`)

**❌ Missing** (need to add):
- ❌ LLM API call metrics (`llm.requests/errors/duration`)
- ❌ LLM token usage tracking (`llm.tokens`)
- ❌ Error classification by type
- ❌ Grafana dashboards (but infrastructure is ready!)

**Your metrics are already being collected!** You just need to:
1. Start the observability stack
2. Run your bot with `OTEL_EXPORTER=otlp`
3. Create Grafana dashboards with the PromQL queries above

## Next Steps (Revised Based on Your Code)

### Immediate (5 minutes):
```bash
# Start the full stack
podman-compose -f docker-compose.observability.yml up -d

# Run bot with OTLP export
OTEL_EXPORTER=otlp OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
cargo run --release --features otel-otlp -p botticelli_server --bin bot-server

# Check metrics are flowing
open http://localhost:9090  # Query: narrative_json_failures
open http://localhost:3000  # Grafana dashboards
```

### Short Term (1-2 hours):
1. Add `LlmMetrics` to `botticelli_models` (code provided above)
2. Instrument Gemini client `generate()` method
3. Create custom Grafana dashboards using your actual metric names

### Medium Term (this week):
1. Build dashboards for:
   - LLM API health (error rates, latency, token usage)
   - JSON parsing success/failure rates
   - Narrative execution performance
   - Bot health and queue depth
2. Set up alerting rules (LLM error rate > 10%, JSON parse failures spike, etc.)
3. Add error classification (rate limit, auth, network, unknown)

## Troubleshooting

### No traces showing up in Grafana
- Check OTEL_EXPORTER_OTLP_ENDPOINT is set correctly
- Verify Jaeger is running: `podman logs botticelli-jaeger`
- Check network: `curl http://localhost:4317`

### No metrics in Prometheus
- Metrics export requires custom instrumentation (see Step 2-4 above)
- Check Prometheus targets: http://localhost:9090/targets
- Verify Jaeger metrics endpoint: `curl http://localhost:14269/metrics`

### Grafana can't connect to datasources
- Check containers are on same network
- Verify datasource URLs use container names (`http://prometheus:9090`)
- Check health: `podman-compose ps`

## Resources

- [OpenTelemetry Rust SDK](https://docs.rs/opentelemetry/)
- [Prometheus PromQL](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Grafana Dashboards](https://grafana.com/docs/grafana/latest/dashboards/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)

---

**Pro Tip**: Start with trace-only observability (what you have), then gradually add metrics as you identify bottlenecks. Don't try to instrument everything at once!
