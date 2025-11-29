# OpenTelemetry Integration Plan

**Status:** Planning  
**Last Updated:** 2025-11-28  
**Target:** Botticelli Bot Server v0.3.0

## Executive Summary

Implement production-grade observability by bridging our existing `tracing` instrumentation to OpenTelemetry, enabling export to industry-standard backends (SigNoz, Jaeger, Grafana Cloud, etc.) with **minimal application code changes**.

## Key Insight: We're 80% Done

**Current State:**
- ✅ All public functions have `#[instrument]`
- ✅ Structured logging with span fields
- ✅ Error tracking with locations
- ✅ Comprehensive tracing coverage

**What's Missing:**
- ❌ Export to persistent backend (logs go to stdout only)
- ❌ Metrics collection (counters, gauges, histograms)
- ❌ Distributed context propagation
- ❌ Visualization/querying UI

**Solution:** Add OpenTelemetry layer to bridge existing instrumentation to OTLP backends.

## Architecture

### Three-Layer Design

```
┌──────────────────────────────────────────────────────────┐
│  Application Layer (Botticelli)                          │
│  - Existing: info!(), debug!(), #[instrument]            │
│  - New: Metrics macros (counter!, histogram!)            │
│  - No changes to business logic                          │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│  OpenTelemetry SDK Layer                                 │
│  - opentelemetry-appender-tracing: Logs bridge           │
│  - opentelemetry-sdk: Traces + Metrics                   │
│  - Resource attributes (service.name, version, etc.)     │
│  - Batching, sampling, filtering                         │
└────────────────────────┬─────────────────────────────────┘
                         │ OTLP (gRPC/HTTP)
┌────────────────────────▼─────────────────────────────────┐
│  Backend Layer (User Choice)                             │
│  - Development: Jaeger/stdout                            │
│  - Production: SigNoz, Grafana Cloud, Honeycomb, etc.    │
│  - Storage: ClickHouse, Tempo, S3                        │
│  - UI: Query, visualize, alert                           │
└──────────────────────────────────────────────────────────┘
```

### Why This Approach

1. **Leverage Existing Work** - Don't rewrite instrumentation
2. **Vendor Neutral** - Switch backends without code changes
3. **Industry Standard** - OTLP is universally supported
4. **Rust Idiomatic** - `tracing` is the Rust standard
5. **Future Proof** - OpenTelemetry is CNCF standard

## Implementation Plan

### Phase 1: Basic OTLP Export (Week 1)

**Goal:** Export logs and traces to Jaeger for development.

**Dependencies:**
```toml
[dependencies]
opentelemetry = "0.31"
opentelemetry-sdk = "0.31"
opentelemetry-stdout = "0.31"  # For development/testing
opentelemetry-otlp = { version = "0.31", features = ["tonic", "logs"] }  # For production
opentelemetry-appender-tracing = "0.31"
opentelemetry-semantic-conventions = "0.31"
tracing-opentelemetry = "0.22"  # Note: version mismatch with OTel is normal
```

**Code Changes:**

1. **Add telemetry init function** (`crates/botticelli/src/telemetry.rs`):

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{logs, trace, Resource, runtime::Tokio};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct TelemetryGuard {
    tracer_provider: trace::TracerProvider,
    logger_provider: logs::LoggerProvider,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Graceful shutdown
        let _ = self.tracer_provider.shutdown();
        let _ = self.logger_provider.shutdown();
    }
}

pub fn init_telemetry(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<TelemetryGuard, Box<dyn std::error::Error>> {
    // Create resource with service metadata
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // Setup trace provider
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint)
        )
        .with_trace_config(
            trace::config()
                .with_resource(resource.clone())
                .with_sampler(trace::Sampler::AlwaysOn)  // TODO: Make configurable
        )
        .install_batch(Tokio)?;

    // Setup logger provider
    let logger_provider = opentelemetry_otlp::new_pipeline()
        .logging()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint)
        )
        .with_resource(resource)
        .install_batch(Tokio)?;

    // Create tracer for spans
    let tracer = tracer_provider.tracer("botticelli");
    
    // Create tracing-opentelemetry layer for spans
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    
    // Create log bridge layer
    let otel_log_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    // Initialize subscriber (keep console output for dev)
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)          // Spans to OTel
        .with(otel_log_layer)            // Logs to OTel
        .init();

    Ok(TelemetryGuard {
        tracer_provider,
        logger_provider,
    })
}
```

2. **Update main.rs** (`crates/botticelli/src/main.rs`):

```rust
use crate::telemetry::init_telemetry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry early
    let config = load_config()?;
    let _guard = init_telemetry(
        "botticelli-server",
        &config.otlp_endpoint.unwrap_or("http://localhost:4317".to_string()),
    )?;

    // Rest of application...
    run_server(config).await?;

    // Guard ensures graceful shutdown on drop
    Ok(())
}
```

3. **Add config field** (`botticelli.toml`):

```toml
[telemetry]
enabled = true
otlp_endpoint = "http://localhost:4317"  # Jaeger default
service_name = "botticelli-server"
```

**Testing:**

```bash
# Run Jaeger in Docker
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest

# Run bot server
just bot-server

# View traces at http://localhost:16686
```

**Success Criteria:**
- ✅ Bot server starts without errors
- ✅ Logs appear in Jaeger UI
- ✅ Spans show narrative execution hierarchy
- ✅ No performance degradation

### Phase 2: Metrics Collection (Week 2)

**Goal:** Add custom metrics for bot operations.

**Additional Dependencies:**
```toml
opentelemetry-prometheus = "0.28"
```

**Metrics to Track:**

1. **Bot Metrics:**
   - `bot.generation.runs.total` (counter) - Total generation runs
   - `bot.generation.duration` (histogram) - Generation time
   - `bot.generation.tokens.used` (counter) - API tokens consumed
   - `bot.curation.approved.total` (counter) - Posts approved
   - `bot.curation.rejected.total` (counter) - Posts rejected
   - `bot.posting.published.total` (counter) - Posts published
   - `bot.posting.failed.total` (counter) - Post failures

2. **Narrative Metrics:**
   - `narrative.execution.duration` (histogram) - Per-narrative timing
   - `narrative.act.duration` (histogram) - Per-act timing
   - `narrative.json.parse.failures` (counter) - JSON parse errors
   - `narrative.json.parse.success` (counter) - Successful parses

3. **System Metrics:**
   - `database.query.duration` (histogram) - DB query times
   - `database.connection.pool.size` (gauge) - Connection pool
   - `api.requests.total` (counter) - LLM API calls
   - `api.requests.duration` (histogram) - API latency

**Implementation:**

```rust
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::KeyValue;

pub struct BotMetrics {
    generation_runs: Counter<u64>,
    generation_duration: Histogram<f64>,
    tokens_used: Counter<u64>,
    curation_approved: Counter<u64>,
    curation_rejected: Counter<u64>,
    posting_published: Counter<u64>,
    posting_failed: Counter<u64>,
}

impl BotMetrics {
    pub fn new(meter: &Meter) -> Self {
        Self {
            generation_runs: meter
                .u64_counter("bot.generation.runs")
                .with_description("Total content generation runs")
                .init(),
            generation_duration: meter
                .f64_histogram("bot.generation.duration")
                .with_description("Content generation duration in seconds")
                .with_unit("s")
                .init(),
            // ... rest
        }
    }

    pub fn record_generation(&self, duration_secs: f64, tokens: u64, topic: &str) {
        let attrs = &[KeyValue::new("topic", topic.to_string())];
        self.generation_runs.add(1, attrs);
        self.generation_duration.record(duration_secs, attrs);
        self.tokens_used.add(tokens, attrs);
    }
}
```

**Integration in Bots:**

```rust
impl ContentGenerationBot {
    async fn generate_content(&self) -> Result<(), BotticelliError> {
        let start = std::time::Instant::now();
        
        let result = self.narrative_provider.execute(...).await;
        
        let duration = start.elapsed().as_secs_f64();
        match result {
            Ok(output) => {
                self.metrics.record_generation(
                    duration,
                    output.tokens_used,
                    &self.config.topic,
                );
                Ok(())
            }
            Err(e) => {
                self.metrics.record_generation_failure(duration, &self.config.topic);
                Err(e)
            }
        }
    }
}
```

### Phase 3: Production Backend (Week 3)

**Goal:** Deploy with SigNoz for production observability.

**SigNoz Setup:**

```bash
# Docker Compose deployment
git clone https://github.com/SigNoz/signoz.git
cd signoz/deploy
docker-compose up -d
```

**Configuration:**

```toml
[telemetry]
enabled = true
otlp_endpoint = "http://signoz:4317"
service_name = "botticelli-production"
sampling_rate = 0.1  # Sample 10% of traces in production
```

**Dashboard Setup:**

1. Navigate to SigNoz UI (http://localhost:3301)
2. Create dashboards for:
   - Bot operation overview
   - Content pipeline flow
   - Error rates and patterns
   - API cost tracking
   - Performance trends

3. Setup alerts:
   - Generation failures > 10% in 5 minutes
   - Posting failures > 5 in 10 minutes
   - API token usage > 80% of daily quota
   - Database query duration > 1 second

### Phase 4: Advanced Features (Week 4+)

**Features:**

1. **Sampling Strategies:**
   - Always sample errors
   - Sample successful operations based on load
   - Custom sampling per narrative type

2. **Context Propagation:**
   - Correlate generation → curation → posting
   - Track content through entire pipeline
   - Link failures to original generation

3. **Custom Exporters:**
   - Cost tracking export to CSV
   - Failure report generation
   - Performance regression detection

4. **Integration:**
   - Prometheus metrics endpoint for Kubernetes
   - Health check endpoint using telemetry data
   - Status page generation

## Configuration

### Development
```toml
[telemetry]
enabled = true
backend = "jaeger"
otlp_endpoint = "http://localhost:4317"
sampling_rate = 1.0  # Sample everything
export_console = true  # Also log to console
```

### Production
```toml
[telemetry]
enabled = true
backend = "signoz"
otlp_endpoint = "http://signoz-collector:4317"
sampling_rate = 0.1  # 10% sampling
export_console = false
batch_size = 512
batch_timeout_ms = 5000
```

## Backend Comparison

| Backend | Use Case | Pros | Cons |
|---------|----------|------|------|
| **Jaeger** | Development | Easy setup, mature, free | Traces only, basic UI |
| **SigNoz** | Production | All signals, good UI, self-hosted | Requires infra |
| **Grafana Cloud** | SaaS | Managed, integrated, reliable | Cost, vendor lock-in |
| **Honeycomb** | Advanced | Powerful querying, great UX | Expensive |
| **Stdout** | CI/Testing | No infra needed | No querying |

## Migration Path

### Phase 1: Development (Now)
- Jaeger locally
- Console output
- Learn OTel patterns

### Phase 2: Staging (Next Month)
- Self-hosted SigNoz
- Basic dashboards
- Alert testing

### Phase 3: Production (3 Months)
- Production SigNoz or SaaS
- Full dashboards
- Automated alerts
- Cost optimization

## Cost Considerations

**SigNoz (Self-hosted):**
- Infrastructure: $50-200/month (depending on scale)
- Storage: ~$20/month for 30 days retention
- No per-event charges

**Grafana Cloud:**
- Free tier: 50GB logs, 10k series
- Paid: ~$0.50/GB ingested
- Estimated: $100-300/month for moderate use

**Honeycomb:**
- Free tier: 20M events/month
- Paid: $0.001/event
- Estimated: $200-500/month

**Recommendation:** Start with self-hosted SigNoz, evaluate cost vs. effort before moving to SaaS.

## Success Metrics

After full implementation:

- ✅ 100% of bot operations traced
- ✅ P50/P95/P99 latencies tracked
- ✅ < 5 minute time to identify failures
- ✅ < 1% overhead from telemetry
- ✅ Alerts fire before users notice issues
- ✅ Cost per 1000 posts tracked
- ✅ Deployment confidence through metrics

## Open Questions

1. **Sampling:** What percentage is acceptable for production?
2. **Retention:** How long should we keep trace data?
3. **Cardinality:** Which attributes should we include in metrics?
4. **Alerting:** Who gets paged for what failures?
5. **Cost:** What's the budget for observability infrastructure?

## Implementation Status

### Phase 0: Foundation (COMPLETED)
- ✅ Added `observability` feature flag
- ✅ Added OpenTelemetry dependencies (v0.31)
- ✅ Created `telemetry` module placeholder
- ⚠️ **Blocker**: OpenTelemetry v0.31 has breaking API changes from v0.28 docs
  - Need to research current v0.31+ API patterns
  - Pipeline builders have different signatures
  - Runtime and provider APIs changed
  - See: https://github.com/open-telemetry/opentelemetry-rust/releases

### Next Steps

1. [ ] Research OpenTelemetry Rust v0.31 API changes
2. [ ] Find working examples for v0.31 OTLP export
3. [ ] Update telemetry.rs with correct v0.31 implementation
4. [ ] Test locally with Jaeger
5. [ ] Add metrics collection (Phase 2)
6. [ ] Deploy SigNoz staging environment
7. [ ] Create dashboards and alerts
8. [ ] Production rollout
9. [ ] Document runbooks using telemetry

## References

- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Tracing Bridge Pattern](https://opentelemetry.io/docs/specs/otel/logs/bridge/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [SigNoz Docs](https://signoz.io/docs/)
- [OpenTelemetry Best Practices](https://opentelemetry.io/docs/concepts/best-practices/)
