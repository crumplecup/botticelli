# Bot Server Observability & Metrics Strategy

## Overview

This document outlines a comprehensive strategy for implementing production-grade observability and metrics for the Botticelli bot server deployment. The goal is to provide deep visibility into narrative execution, bot behavior, content generation pipeline, and system health.

## Current State

### What We Have

**Tracing Infrastructure:**
- `tracing` crate with `#[instrument]` on most public functions
- Structured logging with span context
- Console subscriber for development

**Bot Server:**
- Three bot types: ContentGenerationBot, ContentCurationBot, ContentPostingBot
- Ractor-based actor system
- Scheduled task execution
- Database-backed content pipeline

**Narrative Execution:**
- Multi-act narrative composition
- Table-based state management
- JSON extraction with retry logic
- Carousel execution for batch processing

### What's Missing

1. **Metrics Collection** - No quantitative performance data
2. **Health Checks** - No automated health monitoring
3. **Alerting** - No proactive failure detection
4. **Dashboards** - No visualization of system state
5. **Distributed Tracing** - No request correlation across components
6. **Error Aggregation** - Manual log searching for errors
7. **Performance Profiling** - No bottleneck identification
8. **Cost Tracking** - No API usage/token consumption metrics

## Goals

### Primary Objectives

1. **Visibility** - Know what's happening in production at any time
2. **Debuggability** - Quickly identify and diagnose issues
3. **Performance** - Track and optimize execution times
4. **Reliability** - Detect and alert on failures
5. **Cost Control** - Monitor and optimize API token usage

### Success Metrics

- < 5 minutes to identify root cause of failures
- < 1% overhead from observability infrastructure
- 100% of critical paths instrumented
- Real-time visibility into content pipeline status
- Automated alerting for all critical failures

## Architecture

### Three Pillars of Observability

#### 1. Metrics (Quantitative Data)

**What to Measure:**

**Bot-Level Metrics:**
- Bot uptime and restart counts
- Task execution frequency and duration
- Queue depths (pending content counts)
- Success/failure rates per bot type
- Time since last successful execution

**Narrative-Level Metrics:**
- Narrative execution count and duration
- Act execution times (by act type)
- Processor success/failure rates
- JSON extraction success rates
- Table operation latencies

**Content Pipeline Metrics:**
- Posts generated per hour/day
- Posts curated per hour/day
- Posts published per hour/day
- Content quality scores (if available)
- Pipeline throughput and latency

**API/LLM Metrics:**
- Request counts (by model)
- Token consumption (input/output)
- Request latencies (by model)
- Rate limit approaches/violations
- API error rates
- Cost per request/day/month

**Database Metrics:**
- Query execution times
- Table sizes (potential/approved posts)
- Connection pool utilization
- Transaction success/failure rates

**System Metrics:**
- CPU and memory usage
- Actor mailbox depths
- Thread pool utilization
- Disk I/O for state persistence

#### 2. Logs (Contextual Data)

**Structured Logging Standards:**
- Use `tracing` spans for request correlation
- Include bot_id, narrative_name, act_name in all spans
- Emit events at key decision points
- Log all errors with full context
- Include timing information in completion events

**Log Levels:**
- `ERROR` - Failures requiring immediate attention
- `WARN` - Degraded operation or approaching limits
- `INFO` - Major lifecycle events (bot start/stop, narrative execution)
- `DEBUG` - Detailed execution flow
- `TRACE` - Fine-grained operation details

**Key Events to Log:**
- Bot lifecycle: started, stopped, restarted, failed
- Narrative execution: started, completed, failed
- Content operations: generated, curated, published
- API calls: request sent, response received, rate limited
- Database operations: query executed, transaction committed
- Schedule events: next execution scheduled, execution triggered

#### 3. Traces (Request Flow)

**Distributed Tracing:**
- Trace entire content generation → curation → posting flow
- Correlate narrative executions across bot instances
- Track timing of each pipeline stage
- Identify bottlenecks in multi-step workflows

**Trace Context Propagation:**
- Generate trace ID when content enters pipeline
- Propagate through: generation → potential_posts → curation → approved_posts → posting
- Include in all logs and metrics
- Store in database for post-hoc analysis

## Technology Stack: Rust-Native OpenTelemetry

### Why OpenTelemetry?

**Rust-Idiomatic:**
- We already use `tracing`/`tracing-subscriber` extensively throughout the codebase
- OpenTelemetry integrates seamlessly via `tracing-opentelemetry` bridge
- All existing `#[instrument]` annotations become distributed traces automatically
- No additional HTTP servers or scraping infrastructure needed

**Unified Observability:**
- Single protocol for metrics, traces, and logs (OTLP)
- Vendor-neutral (CNCF standard)
- Export to any compatible backend
- Future-proof for ecosystem evolution

**Lightweight & Native:**
- Push-based model (no Prometheus scraper overhead)
- Native async/tokio integration
- Minimal dependencies
- Rust OpenTelemetry SDK is production-ready

### Backend Options

**Recommended: SigNoz (Open Source)**
- Unified metrics, traces, and logs UI
- Easy self-hosting (Docker Compose)
- Built-in dashboards and alerts
- ClickHouse backend (fast queries)
- No vendor lock-in

**Alternative: Uptrace**
- Lightweight and Rust-friendly
- Good for smaller deployments
- Open source with hosted option

**Development: Jaeger**
- Simple traces-only backend
- Perfect for local development
- Widely supported

## Implementation Progress

### ✅ Phase 1: Tracing Foundation - COMPLETE
- Added OpenTelemetry v0.31 dependencies
- Implemented observability module with tracer/meter initialization
- Created shutdown handler for graceful cleanup
- Integrated with existing tracing infrastructure

### ✅ Phase 2: Bot Metrics - COMPLETE
- Created BotMetrics, NarrativeMetrics, PipelineMetrics
- Integrated OpenTelemetry counters, histograms, gauges
- Added structured metric recording methods
- Exported ServerMetrics aggregate

### ✅ Phase 3: HTTP Metrics API - COMPLETE
- Added axum for HTTP server
- Created REST API with /health, /metrics endpoints
- Implemented MetricsCollector for JSON snapshots
- Ready for integration with bot server

### ✅ Phase 4: Bot Server Integration - COMPLETE  
- ✅ Bot server (`botticelli_bot`) already has metrics integrated
- ✅ Metrics collected via shared `Arc<BotMetrics>`
- ✅ Each bot records its own metrics during execution
- ✅ HTTP metrics server on port 9090 (`/health`, `/metrics` endpoints)
- ✅ Automatic startup alongside bots
- ✅ JSON snapshots for easy consumption

### 📋 Phase 5: Dashboard and Alerts - NOT STARTED
- TODO: Set up SigNoz for visualization
- TODO: Configure alert rules
- TODO: Create operational dashboards

## Implementation Strategy

### Phase 1: OpenTelemetry Foundation (Complete)

**Step 1: Add OpenTelemetry Dependencies**

```toml
# Cargo.toml
[dependencies]
opentelemetry = "0.24"
opentelemetry-otlp = { version = "0.17", features = ["tokio"] }
opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
tracing-opentelemetry = "0.25"
```

**Step 2: Initialize OpenTelemetry Pipeline**

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    trace::{self, Tracer, TracerProvider},
    metrics::{self, MeterProvider},
};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_observability(config: &ObservabilityConfig) -> Result<()> {
    // Create resource (identifies this service)
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "botticelli-bot-server"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // Initialize tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint)
        )
        .with_trace_config(trace::config().with_resource(resource.clone()))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Initialize metrics
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint)
        )
        .with_resource(resource)
        .build()?;

    global::set_meter_provider(meter_provider);

    // Configure tracing subscriber
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    Ok(())
}
```

**Step 3: Instrument with Existing Tracing**

All our existing `#[instrument]` spans automatically become distributed traces!

```rust
// This already works - no changes needed!
#[instrument(skip(self), fields(
    bot_type = %self.bot_type(),
    bot_id = %self.id(),
))]
async fn run(&mut self) -> Result<()> {
    // Bot execution
}
```

**Step 4: Add Metrics via OpenTelemetry**

```rust
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram, Gauge};

pub struct BotMetrics {
    pub executions: Counter<u64>,
    pub failures: Counter<u64>,
    pub duration: Histogram<f64>,
    pub queue_depth: Gauge<u64>,
}

impl BotMetrics {
    pub fn new() -> Self {
        let meter = global::meter("botticelli_bots");
        
        Self {
            executions: meter.u64_counter("bot.executions")
                .with_description("Total bot executions")
                .init(),
            failures: meter.u64_counter("bot.failures")
                .with_description("Total bot failures")
                .init(),
            duration: meter.f64_histogram("bot.duration")
                .with_unit("seconds")
                .with_description("Bot execution duration")
                .init(),
            queue_depth: meter.u64_gauge("bot.queue_depth")
                .with_description("Pending content count")
                .init(),
        }
    }
}

// Usage
metrics.executions.add(1, &[KeyValue::new("bot_type", "generation")]);
metrics.duration.record(elapsed.as_secs_f64(), &[KeyValue::new("bot_type", "generation")]);
```

### Phase 2: Enhanced Logging

**Step 1: Standardize Span Context**

```rust
// Create consistent span hierarchy
#[instrument(skip(self), fields(
    bot_type = %self.bot_type(),
    bot_id = %self.id(),
    iteration = self.iteration_count(),
))]
async fn run(&mut self) -> Result<()> {
    // Bot execution
}

#[instrument(skip(narrative), fields(
    narrative_name = %narrative.name(),
    act_count = narrative.toc().len(),
    has_carousel = narrative.carousel().is_some(),
))]
async fn execute_narrative(narrative: &Narrative) -> Result<Output> {
    // Narrative execution
}

#[instrument(skip(processor), fields(
    act_name = %act.name(),
    processor_type = %processor.type_name(),
    table_name = %table.name(),
))]
async fn process_act(act: &Act, processor: &Processor) -> Result<ProcessorOutput> {
    // Act processing
}
```

**Step 2: Add Key Event Logging**

```rust
// Bot lifecycle
info!(bot_type = %self.bot_type(), "Bot starting");
info!(next_run = %next_schedule, "Scheduled next execution");
warn!(error = ?e, "Bot execution failed, will retry");
info!(uptime_secs = elapsed, "Bot stopped gracefully");

// Narrative execution
info!(narrative = %name, "Starting narrative execution");
debug!(act = %act_name, input_count = inputs.len(), "Processing act");
info!(
    narrative = %name,
    duration_ms = duration.as_millis(),
    acts_completed = act_count,
    "Narrative execution completed"
);

// Content pipeline
info!(
    table = "potential_posts",
    count = posts.len(),
    "Generated new content"
);
info!(
    selected = selected.len(),
    evaluated = total.len(),
    "Curated content for approval"
);
info!(
    post_id = %id,
    channel_id = %channel,
    "Published content to Discord"
);

// API calls
debug!(
    model = %model_name,
    input_tokens = input_count,
    max_tokens = max_output,
    "Sending API request"
);
warn!(
    model = %model_name,
    retry_after = retry_secs,
    "Rate limited, backing off"
);
```

**Step 3: Configure Log Output**

```rust
// Support multiple output formats
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_observability(config: &ObservabilityConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Add JSON formatting for production
    let json_layer = if config.json_logs {
        Some(tracing_subscriber::fmt::layer().json())
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(json_layer)
        .init();

    Ok(())
}
```

### Phase 3: Distributed Tracing

**Step 1: Add OpenTelemetry**

```rust
use opentelemetry::{
    global,
    sdk::{trace, Resource},
    trace::{Tracer, TracerProvider},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_tracing(config: &TracingConfig) -> Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint)
        )
        .with_trace_config(
            trace::config()
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "botticelli-bot-server"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(/* existing layers */)
        .init();

    Ok(())
}
```

**Step 2: Propagate Trace Context**

```rust
// Add trace_id to content records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialPost {
    pub text_content: String,
    pub generated_at: DateTime<Utc>,
    pub trace_id: Option<String>, // NEW: Track through pipeline
}

// Capture at generation time
let trace_id = span::current()
    .context()
    .span()
    .span_context()
    .trace_id()
    .to_string();

// Propagate through pipeline
#[instrument(skip(post), fields(trace_id = %post.trace_id.as_deref().unwrap_or("unknown")))]
async fn curate_post(post: PotentialPost) -> Result<ApprovedPost> {
    // Curation logic
}
```

### Phase 4: Health Checks & Alerting

**Step 1: Implement Health Endpoint**

```rust
use axum::{Json, http::StatusCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: &'static str, // "healthy", "degraded", "unhealthy"
    pub bots: Vec<BotHealth>,
    pub database: DatabaseHealth,
    pub api: ApiHealth,
}

#[derive(Serialize)]
pub struct BotHealth {
    pub name: String,
    pub status: &'static str,
    pub last_success: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
}

pub async fn health_check(server: &BotServer) -> (StatusCode, Json<HealthStatus>) {
    let status = server.get_health_status().await;
    
    let code = match status.status {
        "healthy" => StatusCode::OK,
        "degraded" => StatusCode::OK, // 200 but with warning
        "unhealthy" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    
    (code, Json(status))
}
```

**Step 2: Add Liveness & Readiness Probes**

```rust
// Liveness: Is the server process alive?
pub async fn liveness() -> StatusCode {
    StatusCode::OK // If we can respond, we're alive
}

// Readiness: Can we handle requests?
pub async fn readiness(server: &BotServer) -> StatusCode {
    if server.is_ready().await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

// Check readiness criteria
impl BotServer {
    async fn is_ready(&self) -> bool {
        // Database connection healthy
        let db_ok = self.database.ping().await.is_ok();
        
        // At least one bot is healthy
        let bots_ok = self.bots.iter()
            .any(|b| b.is_healthy().await);
        
        // API rate limiter not exhausted
        let api_ok = !self.rate_limiter.is_exhausted().await;
        
        db_ok && bots_ok && api_ok
    }
}
```

**Step 3: Configure Alerting Rules**

```yaml
# prometheus_alerts.yml
groups:
  - name: botticelli_bots
    interval: 30s
    rules:
      # Bot failures
      - alert: BotConsecutiveFailures
        expr: botticelli_bot_consecutive_failures > 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Bot {{ $labels.bot_type }} failing repeatedly"
          
      - alert: BotDown
        expr: time() - botticelli_bot_last_success_timestamp > 3600
        labels:
          severity: critical
        annotations:
          summary: "Bot {{ $labels.bot_type }} hasn't succeeded in 1 hour"
      
      # Content pipeline
      - alert: ContentPipelineStalled
        expr: rate(botticelli_content_published_total[1h]) == 0
        for: 2h
        labels:
          severity: warning
        annotations:
          summary: "No content published in 2 hours"
          
      - alert: ContentQueueGrowing
        expr: botticelli_potential_posts_size > 1000
        labels:
          severity: warning
        annotations:
          summary: "Potential posts queue very large ({{ $value }})"
      
      # API issues
      - alert: HighApiErrorRate
        expr: rate(botticelli_api_errors_total[5m]) > 0.1
        labels:
          severity: warning
        annotations:
          summary: "High API error rate for {{ $labels.model }}"
          
      - alert: ApproachingRateLimit
        expr: botticelli_rate_limit_remaining_ratio < 0.1
        labels:
          severity: warning
        annotations:
          summary: "Approaching rate limit for {{ $labels.model }}"
      
      # System health
      - alert: HighMemoryUsage
        expr: process_resident_memory_bytes > 2e9  # 2GB
        labels:
          severity: warning
        annotations:
          summary: "Bot server using {{ $value | humanize }}B memory"
```

### Phase 5: Dashboards

**Step 1: Configure Backend Dashboards**

**Dashboard 1: Bot Server Overview**
- Server uptime and restart count
- Total bots running and their status
- Overall content pipeline throughput (posts/hour)
- API request rate and token consumption
- Error rate across all components

**Dashboard 2: Content Pipeline**
- Posts in potential_posts (gauge)
- Posts in approved_posts (gauge)
- Generation rate (posts/hour)
- Curation rate (posts/hour)
- Publishing rate (posts/hour)
- Average time in each pipeline stage
- Pipeline end-to-end latency

**Dashboard 3: Bot Details**
- Per-bot execution frequency
- Per-bot success/failure rates
- Per-bot execution duration (histogram)
- Per-bot consecutive failures
- Time since last successful execution

**Dashboard 4: Narrative Execution**
- Narrative execution counts (by name)
- Narrative execution duration (by name)
- Act execution times (by type)
- JSON extraction success rates
- Carousel iteration counts and timing

**Dashboard 5: API & Costs**
- Request counts by model
- Token consumption by model (input/output)
- Request latencies by model (p50, p95, p99)
- Rate limit utilization by model
- Estimated costs (tokens × pricing)
- API errors by model and type

**Step 2: Dashboard Configuration Example**

```json
{
  "dashboard": {
    "title": "Botticelli Content Pipeline",
    "panels": [
      {
        "title": "Content Generation Rate",
        "targets": [
          {
            "expr": "rate(botticelli_content_generated_total[5m])"
          }
        ]
      },
      {
        "title": "Queue Sizes",
        "targets": [
          {
            "expr": "botticelli_potential_posts_size",
            "legendFormat": "Potential"
          },
          {
            "expr": "botticelli_approved_posts_size",
            "legendFormat": "Approved"
          }
        ]
      },
      {
        "title": "Pipeline Latency (p95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, botticelli_pipeline_latency_seconds_bucket)"
          }
        ]
      }
    ]
  }
}
```

### Phase 6: Cost Tracking

**Step 1: Token Accounting**

```rust
pub struct TokenUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub timestamp: DateTime<Utc>,
    pub narrative_name: Option<String>,
    pub bot_type: Option<String>,
    pub trace_id: Option<String>,
}

impl TokenUsage {
    pub fn estimated_cost(&self) -> f64 {
        // Pricing per 1M tokens (example rates)
        let (input_rate, output_rate) = match self.model.as_str() {
            "gemini-2.0-flash-exp" => (0.0, 0.0), // Free tier
            "gemini-1.5-flash" => (0.075, 0.30),
            "gemini-1.5-pro" => (1.25, 5.00),
            _ => (0.0, 0.0),
        };
        
        let input_cost = (self.input_tokens as f64 / 1_000_000.0) * input_rate;
        let output_cost = (self.output_tokens as f64 / 1_000_000.0) * output_rate;
        
        input_cost + output_cost
    }
}
```

**Step 2: Cost Metrics**

```rust
pub struct CostMetrics {
    pub tokens_by_model: Counter,
    pub cost_by_model: Counter,
    pub cost_by_bot: Counter,
    pub cost_by_narrative: Counter,
    pub daily_cost: Gauge,
    pub monthly_cost: Gauge,
}

// Update after each API call
cost_metrics.tokens_by_model
    .with_label_values(&[model_name, "input"])
    .inc_by(usage.input_tokens);
    
cost_metrics.cost_by_model
    .with_label_values(&[model_name])
    .inc_by(usage.estimated_cost());
```

**Step 3: Budget Alerts**

```yaml
- alert: DailyCostExceeded
  expr: botticelli_daily_cost_dollars > 10
  labels:
    severity: warning
  annotations:
    summary: "Daily API cost exceeded $10: ${{ $value }}"

- alert: MonthlyBudgetAlert
  expr: botticelli_monthly_cost_dollars > 250
  labels:
    severity: critical
  annotations:
    summary: "Monthly API cost exceeded $250: ${{ $value }}"
```

## Configuration

### Observability Config Structure

```toml
# bot_server.toml
[observability]
# Logging
log_level = "info"
json_logs = true  # Use JSON formatting in production

# Metrics
metrics_enabled = true
metrics_port = 9090
metrics_path = "/metrics"

# Tracing
tracing_enabled = true
otlp_endpoint = "http://localhost:4317"
sample_rate = 1.0  # 100% sampling for now

# Health checks
health_enabled = true
health_port = 9091

# Cost tracking
cost_tracking_enabled = true
daily_budget_usd = 10.0
monthly_budget_usd = 250.0
```

## Deployment

### Local Development

```bash
# Start Prometheus
docker run -p 9090:9090 -v ./prometheus.yml:/etc/prometheus/prometheus.yml prom/prometheus

# Start SigNoz (includes integrated dashboard)
docker run -p 3301:3301 signoz/signoz

# Start Jaeger (distributed tracing)
docker run -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one

# Run bot server with observability
just bot-server
```

### Production Deployment

**Option 1: Self-Hosted Stack**
- Prometheus for metrics
- Integrated dashboards (built into backend)
- Jaeger or Tempo for traces
- Alertmanager for notifications

**Option 2: Cloud-Native Observability**
- SigNoz Cloud (metrics + traces + logs + integrated dashboard + alerts)
- Honeycomb (traces + events)
- Datadog (all-in-one, expensive)

**Option 3: Open Source Stack**
- OpenTelemetry Collector (unified ingestion)
- Prometheus (metrics)
- Loki (logs)
- Tempo (traces)
- Backend built-in dashboards (SigNoz/Uptrace/Jaeger)

## Testing Strategy

### Metrics Testing

```rust
#[tokio::test]
async fn test_bot_execution_metrics() {
    let registry = Registry::new();
    let metrics = BotMetrics::new(&registry);
    
    let bot = ContentGenerationBot::new(config, metrics.clone());
    bot.run().await.unwrap();
    
    // Verify metrics were recorded
    let executions = metrics.executions.get();
    assert!(executions > 0);
    
    let samples = metrics.duration.collect();
    assert!(!samples.is_empty());
}
```

### Health Check Testing

```rust
#[tokio::test]
async fn test_health_endpoint() {
    let server = test_server().await;
    
    let response = server.get("/health").await;
    assert_eq!(response.status(), 200);
    
    let health: HealthStatus = response.json().await;
    assert_eq!(health.status, "healthy");
}
```

### Trace Propagation Testing

```rust
#[tokio::test]
async fn test_trace_propagation() {
    let tracer = test_tracer();
    
    let span = tracer.start("test_content_pipeline");
    let trace_id = span.span_context().trace_id();
    
    // Generate content with trace context
    let post = generate_content().await.unwrap();
    assert_eq!(post.trace_id, Some(trace_id.to_string()));
    
    // Verify trace appears in curation
    let approved = curate_content(post).await.unwrap();
    assert_eq!(approved.trace_id, Some(trace_id.to_string()));
}
```

## Rollout Plan

### Phase 1: Foundation (Completed)
- [x] Custom metrics module with atomic counters
- [x] Execution, failure, and success tracking
- [x] Time-since-last-success tracking
- [x] Overall success rate calculation

### Phase 2: Bot & Narrative Metrics (Completed)
- [x] Add metrics to ContentGenerationBot
- [x] Add metrics to ContentCurationBot
- [x] Add metrics to ContentPostingBot
- [x] Track execution duration for all bots
- [x] Emit structured logs with timing data
- [ ] Add metrics to narrative executor
- [ ] Configure backend dashboard views

### Week 3: API & Cost Tracking
- [ ] Add API request metrics
- [ ] Implement token usage tracking
- [ ] Add cost estimation
- [ ] Create cost tracking dashboard
- [ ] Set up budget alerts

### Week 4: Enhanced Logging
- [ ] Standardize span contexts
- [ ] Add key event logging
- [ ] Configure JSON log output
- [ ] Test log aggregation

### Week 5: Health & Alerting
- [ ] Implement health check endpoint
- [ ] Add liveness/readiness probes
- [ ] Define Prometheus alert rules
- [ ] Test alert firing
- [ ] Configure notification channels

### Week 6: Distributed Tracing
- [ ] Add OpenTelemetry dependencies
- [ ] Implement trace context propagation
- [ ] Add trace IDs to database records
- [ ] Set up Jaeger instance
- [ ] Create trace visualization dashboards

### Week 7: Documentation & Polish
- [ ] Document all metrics and their meaning
- [ ] Create runbook for common alerts
- [ ] Write deployment guide
- [ ] Record demo videos
- [ ] Update README with observability info

## Success Criteria

- [ ] All bots emit lifecycle metrics
- [ ] All narrative executions are traced end-to-end
- [ ] API token usage tracked per model
- [ ] Estimated costs visible in dashboard
- [ ] Health endpoint returns accurate status
- [ ] Alerts fire for critical failures
- [ ] < 1% performance overhead from observability
- [ ] Can diagnose any failure in < 5 minutes using dashboards

## Future Enhancements

### Advanced Features
- Automatic anomaly detection (ML-based)
- Predictive alerting (failure prediction)
- Cost optimization suggestions
- Performance regression detection
- Chaos engineering tests

### Integration Opportunities
- Slack notifications for alerts
- PagerDuty for on-call rotation
- GitHub integration for deploy tracking
- Discord bot for status updates
- Cost reports emailed weekly

### Scaling Considerations
- Metrics aggregation across multiple server instances
- Centralized log storage (Loki/Elasticsearch)
- Trace sampling for high-volume systems
- Metrics downsampling for long-term storage

## References

- [Prometheus Documentation](https://prometheus.io/docs/)
- [SigNoz Dashboards](https://signoz.io/docs/userguide/dashboards/)
- [Uptrace Dashboards](https://uptrace.dev/get/dashboards.html)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Tracing Crate](https://docs.rs/tracing/)
- [The Three Pillars of Observability](https://www.oreilly.com/library/view/distributed-systems-observability/9781492033431/ch04.html)
