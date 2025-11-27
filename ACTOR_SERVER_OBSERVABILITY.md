# Actor Server: Multi-Phase Narrative Automation

**Date**: 2025-11-27
**Status**: In Progress - Migrating to Ractor

## Table of Contents

### Core Sections
- [Executive Summary](#executive-summary)
- [Phase 1: Observability & Metrics](#phase-1-observability--metrics)
- [Phase 2: HTTP API for Runtime Control](#phase-2-http-api-for-runtime-control)
- [Implementation Sequence](#implementation-sequence)
- [Success Criteria](#success-criteria)
- [Risks and Mitigations](#risks-and-mitigations)
- [Summary](#summary)

### Phase 1: Observability
- [Why Observability Matters](#why-observability-matters)
- [Implementation Plan](#implementation-plan)
  - [1.1: Prometheus Metrics Export](#11-prometheus-metrics-export-core)
  - [1.2: Health Check Endpoints](#12-health-check-endpoints)
  - [1.3: Grafana Dashboards](#13-grafana-dashboards-reference)
  - [1.4: Alerting Rules](#14-alerting-rules-reference)
- [Testing Strategy](#testing-strategy)
- [Deliverables](#deliverables)

### Phase 2: HTTP API
- [Why an HTTP API?](#why-an-http-api)
- [Implementation Plan](#implementation-plan-1)
  - [2.1: API Framework Setup](#21-api-framework-setup)
  - [2.2: API Server Implementation](#22-api-server-implementation)
  - [2.3: Authentication & Authorization](#23-authentication--authorization)
  - [2.4: Integration with Actor Server](#24-integration-with-actor-server)
  - [2.5: API Client Examples](#25-api-client-examples)
- [Testing Strategy](#testing-strategy-1)
- [Deliverables](#deliverables-1)

### Implementation Guide
- [Recommended Order](#recommended-order)
- [Alternative: Parallel Development](#alternative-parallel-development)
- [Minimal Viable Product (MVP)](#minimal-viable-product-mvp)

### Reference
- [Phase 1 Complete When](#phase-1-complete-when)
- [Phase 2 Complete When](#phase-2-complete-when)
- [Production Ready When](#production-ready-when)
- [Phase 1 Risks](#phase-1-risks)
- [Phase 2 Risks](#phase-2-risks)
- [Appendix: Database Schema Extensions](#appendix-database-schema-extensions)

---

## Executive Summary

This document outlines the architecture for a long-running actor server that hosts three independent narrative-execution actors for automated Discord content posting:

- **Generation Actor** - Runs content generation carousel, outputs to `potential_posts`
- **Curation Actor** - Reviews and approves posts, outputs to `approved_posts`  
- **Posting Actor** - Posts approved content to Discord on schedule

The system uses **Ractor** (not actix) for actor framework, **PostgreSQL** for inter-actor communication via shared tables, and **NoOpPlatform** for generation/curation actors (no Discord posting until final stage).

**Current Status**: Migrating from actix to Ractor to resolve runtime conflicts in tests.

---

## Architecture Overview

### Three-Phase Pipeline

```
┌──────────────────────────────────────────────────────────────────┐
│                   Actor Server (Long-Running)                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                    │
│  ┌─────────────────┐      ┌──────────────────┐     ┌───────────┐│
│  │ Generation      │      │ Curation         │     │ Posting   ││
│  │ Actor           │      │ Actor            │     │ Actor     ││
│  ├─────────────────┤      ├──────────────────┤     ├───────────┤│
│  │ Narrative:      │      │ Narrative:       │     │ Narrative:││
│  │ generation_     │      │ curation_        │     │ posting_  ││
│  │ carousel.toml   │      │ workflow.toml    │     │ schedule. ││
│  │                 │      │                  │     │ toml      ││
│  │ Platform:       │      │ Platform:        │     │ Platform: ││
│  │ NoOpPlatform    │      │ NoOpPlatform     │     │ Discord   ││
│  │                 │      │                  │     │           ││
│  │ Schedule:       │      │ Schedule:        │     │ Schedule: ││
│  │ Every 2 hours   │      │ Every 4 hours    │     │ Hourly    ││
│  │                 │      │                  │     │           ││
│  │ Output Table:   │      │ Input Table:     │     │ Input:    ││
│  │ potential_posts │──────▶ potential_posts  │     │ approved_ ││
│  │                 │      │ Output Table:    │     │ posts     ││
│  │                 │      │ approved_posts   │─────▶           ││
│  └─────────────────┘      └──────────────────┘     └───────────┘│
│                                                                    │
└────────────────────────────────────┬───────────────────────────────┘
                                     │
                          ┌──────────▼──────────┐
                          │ PostgreSQL Database │
                          │                     │
                          │ • potential_posts   │
                          │ • approved_posts    │
                          │ • content_metadata  │
                          └─────────────────────┘
```

### Communication Pattern

**Database as Message Bus**: Actors communicate exclusively through shared database tables.

- Generation writes → `potential_posts`
- Curation reads `potential_posts`, writes → `approved_posts`
- Posting reads `approved_posts`, posts to Discord

**No direct actor-to-actor messaging** - completely decoupled, scales independently.

### Actor Framework: Ractor

Using **Ractor** (not actix) because:
- ✅ Works with tokio runtime (no nested runtime issues)
- ✅ Supervision tree support
- ✅ Location transparency (can distribute later)
- ✅ Better test compatibility
- ✅ Active development

**Migration Status**: Currently removing actix dependencies and implementing Ractor-based narrative actors.

---

## Implementation Phases

### Phase 1: Ractor Migration ✅ (In Progress)

**Goal**: Replace actix with Ractor throughout the codebase

**Files Being Modified**:
1. `crates/botticelli_actor/Cargo.toml` - Remove actix, add ractor
2. `crates/botticelli_narrative/src/narrative_actor.rs` - Implement Ractor actor
3. `crates/botticelli_narrative/src/storage_actor.rs` - Convert to Ractor (if needed)
4. `crates/botticelli_actor/src/bin/actor-server.rs` - Use Ractor runtime

**Ractor Actor Pattern**:
```rust
use ractor::{Actor, ActorRef, ActorProcessingErr};

pub struct NarrativeActor {
    narrative_path: PathBuf,
    narrative_name: String,
    schedule: Schedule,
}

#[derive(Clone)]
pub enum NarrativeActorMessage {
    Execute,           // Scheduled execution trigger
    Pause,             // Pause execution
    Resume,            // Resume execution
    GetStatus,         // Query status
    Shutdown,          // Graceful shutdown
}

#[async_trait::async_trait]
impl Actor for NarrativeActor {
    type Msg = NarrativeActorMessage;
    type State = NarrativeActorState;
    type Arguments = NarrativeActorConfig;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Initialize narrative, load config
        // Start timer task for scheduled execution
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            NarrativeActorMessage::Execute => {
                // Load narrative, execute, save to database
            }
            NarrativeActorMessage::Pause => {
                state.paused = true;
            }
            // ... other messages
        }
    }
}
```

### Phase 2: Narrative Actor Implementation

**Goal**: Create Ractor-based actor that runs narratives on schedule

**Components**:
1. `NarrativeActor` - Main actor implementation
2. `Schedule` - Parse and calculate next execution time
3. Timer task - Sends `Execute` message on schedule
4. Narrative execution - Load TOML, run, save results

**Schedule Syntax**:
```toml
[actors.generation]
narrative = "narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"
schedule = "every 2 hours"
platform = "noop"  # NoOpPlatform
enabled = true

[actors.curation]
narrative = "narratives/discord/curation_workflow.toml"
narrative_name = "curate"
schedule = "every 4 hours"
platform = "noop"
enabled = true

[actors.posting]
narrative = "narratives/discord/posting_schedule.toml"
narrative_name = "post"
schedule = "every 1 hour"
platform = "discord"
channel_id = "1234567890"
enabled = true
```

**Schedule Parsing**:
```rust
pub enum Schedule {
    Interval(Duration),         // "every 2 hours"
    Cron(String),               // "0 9 * * *" (daily at 9am)
    Immediate,                  // Run once on startup
}

impl Schedule {
    pub fn from_str(s: &str) -> Result<Self, ScheduleError> {
        if s.starts_with("every ") {
            // Parse "every 2 hours", "every 30 minutes"
        } else if s.contains(" ") {
            // Parse cron expression
        } else if s == "immediate" {
            Ok(Schedule::Immediate)
        }
    }

    pub fn next_execution(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Schedule::Interval(duration) => from + *duration,
            Schedule::Cron(expr) => /* calculate from cron */,
            Schedule::Immediate => from,
        }
    }
}
```

### Phase 3: Actor Server Integration

**Goal**: Load multiple actors from config, manage lifecycle

**Configuration** (`actor_server.toml`):
```toml
[server]
database_url = "postgresql://localhost/botticelli"
metrics_port = 9090    # Prometheus metrics
health_port = 8080     # Health checks
api_port = 3000        # Control API (optional)

[actors.generation]
narrative = "narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"
schedule = "every 2 hours"
platform = "noop"
enabled = true

[actors.curation]
narrative = "narratives/discord/curation_workflow.toml"
narrative_name = "curate"
schedule = "every 4 hours"
platform = "noop"
enabled = true

[actors.posting]
narrative = "narratives/discord/posting_schedule.toml"
narrative_name = "post"
schedule = "every 1 hour"
platform = "discord"
channel_id = "1234567890"
enabled = true
```

**Server Implementation**:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ActorServerConfig::load("actor_server.toml")?;

    // Start Ractor runtime
    let (actor_system, handles) = ractor::Actor::spawn_system().await?;

    // Spawn narrative actors
    let mut actor_refs = Vec::new();
    for (name, actor_config) in config.actors {
        if !actor_config.enabled {
            continue;
        }

        let (actor_ref, handle) = Actor::spawn(
            Some(name.clone()),
            NarrativeActor::new(actor_config),
            actor_config.clone(),
        ).await?;

        actor_refs.push((name, actor_ref));
        handles.push(handle);
    }

    info!("Started {} actors", actor_refs.len());

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    // Graceful shutdown
    for (name, actor_ref) in actor_refs {
        actor_ref.send_message(NarrativeActorMessage::Shutdown)?;
    }

    Ok(())
}
```

### Phase 4: Observability (Optional Enhancement)

See sections below for Prometheus metrics, health checks, and HTTP API.

---

## Phase 1: Observability & Metrics (Optional)

**Goal**: Enable operators to monitor actor health, performance, and execution patterns in production.

**Estimated Time**: 1-2 days

### Why Observability Matters

Current state:
- ✅ Execution history in database (after-the-fact analysis)
- ✅ Tracing logs (text-based, requires log aggregation)
- ❌ Real-time metrics (no Prometheus/Grafana integration)
- ❌ Health endpoints (no k8s readiness/liveness probes)
- ❌ Alerting hooks (no automatic incident response)

With observability:
- 📊 Dashboards showing execution trends and failure rates
- 🚨 Automatic alerts when tasks fail or circuit breakers trip
- 🏥 Health checks for deployment orchestration
- 📈 Performance tracking for optimization decisions

### Implementation Plan

#### 1.1: Prometheus Metrics Export (Core)

**Location**: New file `crates/botticelli_actor/src/metrics.rs`

**Dependencies**:
```toml
[dependencies]
prometheus = "0.13"
```

**Metrics to Track**:

```rust
use prometheus::{Registry, IntCounter, IntCounterVec, Histogram, HistogramVec};

pub struct ActorMetrics {
    /// Total executions by actor and status
    pub executions_total: IntCounterVec,

    /// Currently active (running) actors
    pub active_actors: IntGauge,

    /// Circuit breaker trips by actor
    pub circuit_breaker_trips_total: IntCounterVec,

    /// Execution duration in seconds
    pub execution_duration_seconds: HistogramVec,

    /// Skills executed per actor run
    pub skills_executed_total: IntCounterVec,

    /// Current pause state by actor (1=paused, 0=active)
    pub actor_paused: IntGaugeVec,

    /// Time since last successful execution
    pub last_success_timestamp: GaugeVec,

    /// Consecutive failures by actor
    pub consecutive_failures: IntGaugeVec,
}

impl ActorMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let executions_total = IntCounterVec::new(
            Opts::new("actor_executions_total", "Total actor executions")
                .namespace("botticelli"),
            &["actor", "status"],  // status: success, failure, skipped
        )?;
        registry.register(Box::new(executions_total.clone()))?;

        // ... register all metrics

        Ok(Self { executions_total, /* ... */ })
    }

    /// Record execution start
    pub fn record_execution_start(&self, actor: &str) {
        self.active_actors.inc();
    }

    /// Record execution completion
    pub fn record_execution_complete(&self, actor: &str, duration: f64, result: &ExecutionResult) {
        self.active_actors.dec();

        let status = if !result.failed.is_empty() {
            "failure"
        } else {
            "success"
        };

        self.executions_total
            .with_label_values(&[actor, status])
            .inc();

        self.execution_duration_seconds
            .with_label_values(&[actor])
            .observe(duration);

        self.skills_executed_total
            .with_label_values(&[actor, "succeeded"])
            .add(result.succeeded.len() as i64);

        if status == "success" {
            self.last_success_timestamp
                .with_label_values(&[actor])
                .set(chrono::Utc::now().timestamp() as f64);
        }
    }

    /// Record circuit breaker trip
    pub fn record_circuit_breaker_trip(&self, actor: &str) {
        self.circuit_breaker_trips_total
            .with_label_values(&[actor])
            .inc();

        self.actor_paused
            .with_label_values(&[actor])
            .set(1);
    }

    /// Update consecutive failures gauge
    pub fn update_consecutive_failures(&self, actor: &str, count: i64) {
        self.consecutive_failures
            .with_label_values(&[actor])
            .set(count);
    }
}
```

**Integration Points**:

```rust
// In actor-server.rs initialization (after line 122):
let metrics = if std::env::var("ENABLE_METRICS").is_ok() {
    let registry = Registry::new();
    let metrics = ActorMetrics::new(&registry)?;

    // Start metrics HTTP server on :9090
    tokio::spawn(start_metrics_server(registry, 9090));

    Some(Arc::new(metrics))
} else {
    None
};

// In execution loop (around line 318):
if let Some(ref metrics) = metrics {
    metrics.record_execution_start(&name);
}

let start = Instant::now();
match actor.execute(&mut conn).await {
    Ok(result) => {
        if let Some(ref metrics) = metrics {
            metrics.record_execution_complete(&name, start.elapsed().as_secs_f64(), &result);
        }
    }
    Err(e) => {
        if let Some(ref metrics) = metrics {
            // Record failure metrics
        }
    }
}

// When circuit breaker trips (around line 387):
if should_pause {
    if let Some(ref metrics) = metrics {
        metrics.record_circuit_breaker_trip(&name);
    }
}
```

**Metrics Endpoint**: `http://localhost:9090/metrics`

**Example Prometheus Config**:
```yaml
scrape_configs:
  - job_name: 'botticelli-actor-server'
    static_configs:
      - targets: ['localhost:9090']
```

#### 1.2: Health Check Endpoints

**Location**: New file `crates/botticelli_actor/src/health.rs`

**Dependencies**:
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
```

**Endpoints**:

```rust
use axum::{Router, Json, http::StatusCode};
use serde_json::{json, Value};

/// Health check router
pub fn health_router() -> Router {
    Router::new()
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/health/startup", get(startup))
}

/// Liveness probe - is the server running?
async fn liveness() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Readiness probe - can the server accept traffic?
async fn readiness(
    State(state): State<Arc<HealthState>>,
) -> (StatusCode, Json<Value>) {
    // Check database connection
    let db_ok = state.persistence.as_ref()
        .and_then(|p| p.pool.get().ok())
        .is_some();

    // Check if any actors are loaded
    let actors_loaded = state.actor_count.load(Ordering::Relaxed) > 0;

    let ready = db_ok && actors_loaded;

    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(json!({
        "status": if ready { "ready" } else { "not_ready" },
        "checks": {
            "database": db_ok,
            "actors_loaded": actors_loaded,
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Startup probe - has initialization completed?
async fn startup(
    State(state): State<Arc<HealthState>>,
) -> (StatusCode, Json<Value>) {
    let initialized = state.initialized.load(Ordering::Relaxed);

    let status_code = if initialized {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(json!({
        "status": if initialized { "started" } else { "starting" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

pub struct HealthState {
    persistence: Option<Arc<DatabaseStatePersistence>>,
    actor_count: AtomicUsize,
    initialized: AtomicBool,
}
```

**Integration**:
```rust
// In actor-server.rs (after line 122):
let health_state = Arc::new(HealthState {
    persistence: persistence.clone(),
    actor_count: AtomicUsize::new(0),
    initialized: AtomicBool::new(false),
});

// Start health check server on :8080
tokio::spawn(start_health_server(health_state.clone(), 8080));

// After actor loading (around line 212):
health_state.actor_count.store(actors.len(), Ordering::Relaxed);
health_state.initialized.store(true, Ordering::Relaxed);
```

**Health Endpoints**:
- `http://localhost:8080/health/live` - Liveness (is process alive?)
- `http://localhost:8080/health/ready` - Readiness (can accept requests?)
- `http://localhost:8080/health/startup` - Startup (has initialization completed?)

**Kubernetes Integration**:
```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10

startupProbe:
  httpGet:
    path: /health/startup
    port: 8080
  failureThreshold: 30
  periodSeconds: 10
```

#### 1.3: Grafana Dashboards (Reference)

**Example Dashboard Panels**:

1. **Execution Overview**
   - Total executions (counter)
   - Success rate (%)
   - Active executions (gauge)

2. **Circuit Breaker Status**
   - Paused actors (table)
   - Circuit breaker trips over time (graph)
   - Consecutive failures by actor (gauge)

3. **Performance**
   - Execution duration p50/p95/p99 (histogram)
   - Skills per execution (avg)
   - Time since last success (staleness)

4. **Alerts**
   - Actor paused > 1 hour
   - Execution failure rate > 50%
   - No successful executions in 24h

**Example Grafana Query**:
```promql
# Success rate per actor
rate(botticelli_actor_executions_total{status="success"}[5m])
/
rate(botticelli_actor_executions_total[5m])

# P95 execution duration
histogram_quantile(0.95,
  rate(botticelli_execution_duration_seconds_bucket[5m])
)

# Circuit breaker alert
botticelli_actor_paused == 1
```

#### 1.4: Alerting Rules (Reference)

**Prometheus Alerting Rules** (`alerts.yml`):

```yaml
groups:
  - name: botticelli_actor_alerts
    interval: 30s
    rules:
      - alert: ActorCircuitBreakerOpen
        expr: botticelli_actor_paused == 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Actor {{ $labels.actor }} circuit breaker open"
          description: "Circuit breaker has paused actor {{ $labels.actor }} due to repeated failures"

      - alert: ActorExecutionStalled
        expr: (time() - botticelli_last_success_timestamp) > 86400
        for: 1h
        labels:
          severity: critical
        annotations:
          summary: "Actor {{ $labels.actor }} has not succeeded in 24h"
          description: "No successful executions for {{ $labels.actor }} in the last 24 hours"

      - alert: HighFailureRate
        expr: |
          (
            rate(botticelli_actor_executions_total{status="failure"}[10m])
            /
            rate(botticelli_actor_executions_total[10m])
          ) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High failure rate for actor {{ $labels.actor }}"
          description: "Actor {{ $labels.actor }} failure rate is {{ $value | humanizePercentage }}"

      - alert: DatabaseConnectionFailed
        expr: up{job="botticelli-actor-server"} == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Botticelli actor server is down"
          description: "Cannot scrape metrics from actor server"
```

### Testing Strategy

**Manual Testing**:
```bash
# Start server with metrics enabled
ENABLE_METRICS=1 cargo run --bin actor-server --features discord

# Check metrics endpoint
curl http://localhost:9090/metrics | grep botticelli

# Check health endpoints
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready
```

**Integration Test**:
```rust
// tests/metrics_integration_test.rs
#[tokio::test]
async fn test_metrics_recorded() {
    let registry = Registry::new();
    let metrics = ActorMetrics::new(&registry).unwrap();

    metrics.record_execution_start("test_actor");

    let result = ExecutionResultBuilder::default()
        .succeeded(vec![/* ... */])
        .build()
        .unwrap();

    metrics.record_execution_complete("test_actor", 1.5, &result);

    // Verify metrics
    let families = registry.gather();
    let executions = families.iter()
        .find(|f| f.get_name() == "botticelli_actor_executions_total")
        .unwrap();

    assert_eq!(executions.get_metric()[0].get_counter().get_value(), 1.0);
}
```

### Deliverables

1. ✅ `src/metrics.rs` - Prometheus metrics implementation
2. ✅ `src/health.rs` - Health check endpoints
3. ✅ Integration into `actor-server.rs`
4. ✅ Example Grafana dashboard JSON
5. ✅ Example Prometheus alerting rules
6. ✅ Documentation on metrics interpretation
7. ✅ Tests for metrics recording

---

## Phase 2: HTTP API for Runtime Control

**Goal**: Enable programmatic control and introspection of running actors without SSH/database access.

**Estimated Time**: 2-3 days

### Why an HTTP API?

Current state:
- ✅ Actor execution happens automatically on schedule
- ✅ Circuit breaker auto-pauses failing actors
- ❌ No way to manually pause/resume actors
- ❌ No way to trigger immediate execution
- ❌ No way to query execution history via HTTP
- ❌ No way to update configuration without restart

With HTTP API:
- 🎮 Pause actors for maintenance windows
- ⚡ Trigger immediate execution for testing
- 📜 Query execution history and logs
- 🔄 Reload configuration without restart
- 🔐 Secure access via authentication

### Implementation Plan

#### 2.1: API Framework Setup

**Dependencies**:
```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
jsonwebtoken = "9"  # For API authentication
```

**API Structure**:
```
/api/v1
  /actors
    GET    /              - List all actors
    GET    /{id}          - Get actor details
    POST   /{id}/pause    - Pause an actor
    POST   /{id}/resume   - Resume an actor
    POST   /{id}/trigger  - Trigger immediate execution
    GET    /{id}/status   - Get current status

  /executions
    GET    /              - List recent executions (paginated)
    GET    /{id}          - Get execution details
    GET    /actor/{id}    - List executions for specific actor

  /config
    GET    /              - Get current configuration
    POST   /reload        - Reload configuration from file

  /health
    GET    /live          - Liveness probe (shared with Phase 1)
    GET    /ready         - Readiness probe (shared with Phase 1)
```

#### 2.2: API Server Implementation

**Location**: New file `crates/botticelli_actor/src/api_server.rs`

**Core Router**:
```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

pub struct ApiState {
    persistence: Arc<DatabaseStatePersistence>,
    actor_control: Arc<Mutex<ActorControl>>,
}

pub struct ActorControl {
    /// Signal to trigger immediate execution
    trigger_tx: mpsc::Sender<String>,
    /// Current actor states
    actors: HashMap<String, ActorState>,
}

#[derive(Serialize)]
pub struct ActorInfo {
    name: String,
    enabled: bool,
    is_paused: bool,
    consecutive_failures: i32,
    last_run: Option<DateTime<Utc>>,
    next_run: Option<DateTime<Utc>>,
    schedule_type: String,
}

#[derive(Serialize)]
pub struct ExecutionInfo {
    id: i64,
    task_id: String,
    actor_name: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    success: Option<bool>,
    skills_succeeded: Option<i32>,
    skills_failed: Option<i32>,
    error_message: Option<String>,
}

pub fn api_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/actors", get(list_actors))
        .route("/api/v1/actors/:id", get(get_actor))
        .route("/api/v1/actors/:id/pause", post(pause_actor))
        .route("/api/v1/actors/:id/resume", post(resume_actor))
        .route("/api/v1/actors/:id/trigger", post(trigger_actor))
        .route("/api/v1/actors/:id/status", get(actor_status))
        .route("/api/v1/executions", get(list_executions))
        .route("/api/v1/executions/:id", get(get_execution))
        .route("/api/v1/executions/actor/:id", get(list_actor_executions))
        .route("/api/v1/config/reload", post(reload_config))
        .with_state(state)
}

/// List all actors
async fn list_actors(
    State(state): State<ApiState>,
) -> Result<Json<Vec<ActorInfo>>, (StatusCode, String)> {
    let tasks = state.persistence
        .list_all_tasks()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let actors = tasks.into_iter().map(|task| ActorInfo {
        name: task.task_id,
        enabled: true,  // From config
        is_paused: task.is_paused.unwrap_or(false),
        consecutive_failures: task.consecutive_failures.unwrap_or(0),
        last_run: task.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        next_run: Some(DateTime::from_naive_utc_and_offset(task.next_run, Utc)),
        schedule_type: "Interval".to_string(),  // From config
    }).collect();

    Ok(Json(actors))
}

/// Get specific actor details
async fn get_actor(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ActorInfo>, (StatusCode, String)> {
    let task = state.persistence
        .load_task_state(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Actor '{}' not found", id)))?;

    Ok(Json(ActorInfo {
        name: task.task_id,
        enabled: true,
        is_paused: task.is_paused.unwrap_or(false),
        consecutive_failures: task.consecutive_failures.unwrap_or(0),
        last_run: task.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        next_run: Some(DateTime::from_naive_utc_and_offset(task.next_run, Utc)),
        schedule_type: "Interval".to_string(),
    }))
}

/// Pause an actor
async fn pause_actor(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.persistence
        .pause_task(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Resume a paused actor
async fn resume_actor(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.persistence
        .resume_task(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Trigger immediate execution
async fn trigger_actor(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Send trigger signal to main execution loop
    state.actor_control
        .lock()
        .await
        .trigger_tx
        .send(id.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "message": format!("Triggered execution for actor '{}'", id),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Get actor status
async fn actor_status(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let task = state.persistence
        .load_task_state(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Actor '{}' not found", id)))?;

    let history = state.persistence
        .get_execution_history(&id, 5)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recent_success_rate = if !history.is_empty() {
        let successes = history.iter().filter(|h| h.success == Some(true)).count();
        (successes as f64 / history.len() as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(json!({
        "actor": id,
        "is_paused": task.is_paused.unwrap_or(false),
        "consecutive_failures": task.consecutive_failures.unwrap_or(0),
        "last_run": task.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc).to_rfc3339()),
        "next_run": DateTime::from_naive_utc_and_offset(task.next_run, Utc).to_rfc3339(),
        "recent_executions": history.len(),
        "recent_success_rate": format!("{:.1}%", recent_success_rate),
    })))
}

#[derive(Deserialize)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// List recent executions (paginated)
async fn list_executions(
    State(state): State<ApiState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<ExecutionInfo>>, (StatusCode, String)> {
    // Note: Would need to add a method to list all executions
    // For now, return empty array
    Ok(Json(vec![]))
}

/// Get specific execution details
async fn get_execution(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<ExecutionInfo>, (StatusCode, String)> {
    // Note: Would need to add method to get execution by ID
    Err((StatusCode::NOT_IMPLEMENTED, "Not yet implemented".to_string()))
}

/// List executions for specific actor
async fn list_actor_executions(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<ExecutionInfo>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50);

    let history = state.persistence
        .get_execution_history(&id, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let executions = history.into_iter().map(|exec| ExecutionInfo {
        id: exec.id,
        task_id: exec.task_id,
        actor_name: exec.actor_name,
        started_at: DateTime::from_naive_utc_and_offset(exec.started_at, Utc),
        completed_at: exec.completed_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        success: exec.success,
        skills_succeeded: exec.skills_succeeded,
        skills_failed: exec.skills_failed,
        error_message: exec.error_message,
    }).collect();

    Ok(Json(executions))
}

/// Reload configuration
async fn reload_config(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Signal to reload configuration
    // Implementation would need coordination with main loop
    Ok(Json(json!({
        "message": "Configuration reload triggered",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}
```

#### 2.3: Authentication & Authorization

**JWT-based Authentication**:

```rust
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // subject (user id)
    exp: usize,   // expiration
    role: String, // "admin", "operator", "readonly"
}

/// Auth middleware
async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];
    let secret = std::env::var("API_JWT_SECRET")
        .unwrap_or_else(|_| "default-secret".to_string());

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add claims to request extensions for downstream use
    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}

/// Apply auth to protected routes
pub fn api_router_with_auth(state: ApiState) -> Router {
    let protected = Router::new()
        .route("/api/v1/actors/:id/pause", post(pause_actor))
        .route("/api/v1/actors/:id/resume", post(resume_actor))
        .route("/api/v1/actors/:id/trigger", post(trigger_actor))
        .route("/api/v1/config/reload", post(reload_config))
        .layer(middleware::from_fn(auth_middleware));

    let public = Router::new()
        .route("/api/v1/actors", get(list_actors))
        .route("/api/v1/actors/:id", get(get_actor))
        .route("/api/v1/actors/:id/status", get(actor_status))
        .route("/api/v1/executions", get(list_executions))
        .route("/api/v1/executions/:id", get(get_execution))
        .route("/api/v1/executions/actor/:id", get(list_actor_executions));

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}
```

**Environment Variables**:
```bash
# Generate a secure secret
API_JWT_SECRET="your-very-secure-secret-key-here"

# Generate tokens (would need separate tool/script)
# Example payload: {"sub": "admin", "role": "admin", "exp": 1735689600}
```

#### 2.4: Integration with Actor Server

**Main Changes to `actor-server.rs`**:

```rust
// After line 122 (after persistence setup):
let (trigger_tx, mut trigger_rx) = mpsc::channel::<String>(100);

let actor_control = Arc::new(Mutex::new(ActorControl {
    trigger_tx: trigger_tx.clone(),
    actors: HashMap::new(),
}));

let api_state = ApiState {
    persistence: persistence.clone().unwrap(),  // Assume persistence required for API
    actor_control: actor_control.clone(),
};

// Start API server on :3000
if std::env::var("ENABLE_API").is_ok() {
    let api_router = api_router_with_auth(api_state);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await
            .expect("Failed to bind API server");

        axum::serve(listener, api_router)
            .await
            .expect("API server failed");
    });
    info!("API server listening on :3000");
}

// In main execution loop (around line 289):
loop {
    tokio::select! {
        _ = interval.tick() => {
            // Normal scheduled execution
        }

        Some(actor_name) = trigger_rx.recv() => {
            // Handle triggered execution
            if let Some((actor, schedule, last_run, tracker)) = actors.get_mut(&actor_name) {
                info!(actor = %actor_name, "Manual execution triggered via API");

                // Execute immediately (same logic as scheduled execution)
                // ...
            } else {
                warn!(actor = %actor_name, "Triggered actor not found");
            }
        }

        _ = shutdown_flag.notified() => {
            break;
        }
    }
}
```

#### 2.5: API Client Examples

**cURL Examples**:

```bash
# List all actors
curl http://localhost:3000/api/v1/actors

# Get specific actor
curl http://localhost:3000/api/v1/actors/daily_poster

# Get actor status
curl http://localhost:3000/api/v1/actors/daily_poster/status

# Pause actor (requires auth)
curl -X POST \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:3000/api/v1/actors/daily_poster/pause

# Resume actor (requires auth)
curl -X POST \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:3000/api/v1/actors/daily_poster/resume

# Trigger immediate execution (requires auth)
curl -X POST \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:3000/api/v1/actors/daily_poster/trigger

# List recent executions for actor
curl http://localhost:3000/api/v1/executions/actor/daily_poster?limit=10
```

**Rust Client Library**:

```rust
use reqwest::Client;
use serde_json::Value;

pub struct ActorServerClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

impl ActorServerClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub async fn list_actors(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/actors", self.base_url);
        let response = self.client.get(&url).send().await?;
        let actors = response.json().await?;
        Ok(actors)
    }

    pub async fn pause_actor(&self, actor: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/actors/{}/pause", self.base_url, actor);
        let mut req = self.client.post(&url);

        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        req.send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn trigger_actor(&self, actor: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/actors/{}/trigger", self.base_url, actor);
        let mut req = self.client.post(&url);

        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
    }
}

// Usage:
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ActorServerClient::new("http://localhost:3000")
        .with_token("your-jwt-token");

    // List actors
    let actors = client.list_actors().await?;
    println!("Actors: {}", actors);

    // Pause an actor
    client.pause_actor("daily_poster").await?;
    println!("Actor paused");

    Ok(())
}
```

**Python Client Example**:

```python
import requests

class ActorServerClient:
    def __init__(self, base_url, token=None):
        self.base_url = base_url
        self.token = token
        self.session = requests.Session()
        if token:
            self.session.headers.update({"Authorization": f"Bearer {token}"})

    def list_actors(self):
        response = self.session.get(f"{self.base_url}/api/v1/actors")
        response.raise_for_status()
        return response.json()

    def get_actor(self, actor_name):
        response = self.session.get(f"{self.base_url}/api/v1/actors/{actor_name}")
        response.raise_for_status()
        return response.json()

    def pause_actor(self, actor_name):
        response = self.session.post(f"{self.base_url}/api/v1/actors/{actor_name}/pause")
        response.raise_for_status()

    def resume_actor(self, actor_name):
        response = self.session.post(f"{self.base_url}/api/v1/actors/{actor_name}/resume")
        response.raise_for_status()

    def trigger_actor(self, actor_name):
        response = self.session.post(f"{self.base_url}/api/v1/actors/{actor_name}/trigger")
        response.raise_for_status()
        return response.json()

    def get_executions(self, actor_name, limit=10):
        response = self.session.get(
            f"{self.base_url}/api/v1/executions/actor/{actor_name}",
            params={"limit": limit}
        )
        response.raise_for_status()
        return response.json()

# Usage
if __name__ == "__main__":
    client = ActorServerClient("http://localhost:3000", token="your-jwt-token")

    # List actors
    actors = client.list_actors()
    print(f"Found {len(actors)} actors")

    # Get specific actor
    actor = client.get_actor("daily_poster")
    print(f"Actor: {actor['name']}, Paused: {actor['is_paused']}")

    # Trigger execution
    result = client.trigger_actor("daily_poster")
    print(f"Triggered: {result['message']}")
```

### Testing Strategy

**API Integration Tests**:

```rust
// tests/api_integration_test.rs
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn test_list_actors() {
    let server = create_test_server().await;

    let response = server.get("/api/v1/actors").await;

    response.assert_status(StatusCode::OK);
    response.assert_json_contains(&json!([
        {"name": "test_actor"},
    ]));
}

#[tokio::test]
async fn test_pause_actor_requires_auth() {
    let server = create_test_server().await;

    let response = server
        .post("/api/v1/actors/test_actor/pause")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_pause_actor_with_auth() {
    let server = create_test_server().await;
    let token = generate_test_token();

    let response = server
        .post("/api/v1/actors/test_actor/pause")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status(StatusCode::OK);

    // Verify actor is actually paused
    let state = server
        .get("/api/v1/actors/test_actor")
        .await;

    state.assert_json_contains(&json!({"is_paused": true}));
}

#[tokio::test]
async fn test_trigger_actor() {
    let server = create_test_server().await;
    let token = generate_test_token();

    let response = server
        .post("/api/v1/actors/test_actor/trigger")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status(StatusCode::OK);
    response.assert_json_contains(&json!({"message": "Triggered execution"}));
}
```

### Deliverables

1. ✅ `src/api_server.rs` - API router and handlers
2. ✅ `src/auth.rs` - JWT authentication middleware
3. ✅ Integration into `actor-server.rs` with trigger channel
4. ✅ OpenAPI/Swagger specification
5. ✅ Client library (Rust)
6. ✅ Example clients (cURL, Python)
7. ✅ API integration tests
8. ✅ Documentation on authentication and endpoints

---

## Implementation Sequence

### Recommended Order

**Week 1: Phase 1 - Observability**
1. Day 1: Prometheus metrics implementation and integration
2. Day 2: Health check endpoints and Grafana dashboard examples
3. Testing and documentation

**Week 2: Phase 2 - HTTP API**
1. Day 1: API framework, basic routes (list, get)
2. Day 2: Control routes (pause, resume, trigger)
3. Day 3: Authentication, integration, testing

### Alternative: Parallel Development

If multiple developers available:
- **Developer A**: Phase 1 (Observability)
- **Developer B**: Phase 2 (HTTP API foundation)
- **Integration**: Merge health checks from Phase 1 into API from Phase 2

### Minimal Viable Product (MVP)

If time-constrained, prioritize:

**Phase 1 MVP** (4-6 hours):
- ✅ Basic Prometheus metrics (executions, failures)
- ✅ Simple health check endpoint
- ❌ Skip: Grafana dashboards (can create later)
- ❌ Skip: Alerting rules (can configure later)

**Phase 2 MVP** (8-12 hours):
- ✅ Read-only endpoints (list actors, executions)
- ✅ Basic control (pause/resume)
- ❌ Skip: Authentication (add later)
- ❌ Skip: Trigger execution (complex integration)
- ❌ Skip: Config reload (complex implementation)

---

## Success Criteria

### Phase 1 Complete When:
- ✅ Prometheus metrics endpoint responding on :9090
- ✅ Key metrics visible: executions, failures, circuit breakers, duration
- ✅ Health endpoints responding on :8080 (live, ready, startup)
- ✅ Example Grafana dashboard JSON provided
- ✅ Example alert rules documented
- ✅ Tests verify metrics are recorded correctly

### Phase 2 Complete When:
- ✅ API server responding on :3000
- ✅ All CRUD endpoints functional (list, get, pause, resume, trigger)
- ✅ JWT authentication protecting control endpoints
- ✅ Trigger execution works and integrates with main loop
- ✅ Client examples work (cURL, Rust, Python)
- ✅ OpenAPI spec generated
- ✅ Integration tests passing

### Production Ready When:
- ✅ Both phases complete
- ✅ Deployed with metrics scraping configured
- ✅ Grafana dashboards showing real data
- ✅ Alerts firing when issues occur
- ✅ API accessible and documented
- ✅ Authentication configured with real secrets

---

## Risks and Mitigations

### Phase 1 Risks

**Risk**: Metrics overhead impacts execution performance
- **Mitigation**: Metrics are async, minimal overhead (<1ms)
- **Mitigation**: Can disable with environment variable
- **Testing**: Load test with metrics enabled

**Risk**: Prometheus scraping fails
- **Mitigation**: Metrics endpoint is pull-based, no data loss
- **Mitigation**: Health checks independent of metrics

### Phase 2 Risks

**Risk**: API changes break main execution loop
- **Mitigation**: Minimal coupling via channels
- **Mitigation**: Comprehensive integration tests
- **Mitigation**: Feature flag to disable API

**Risk**: Authentication misconfiguration
- **Mitigation**: Default to secure (no default token)
- **Mitigation**: Require explicit environment variable
- **Mitigation**: Document token generation clearly

**Risk**: Trigger execution causes race conditions
- **Mitigation**: Use tokio channels (thread-safe)
- **Mitigation**: Actors already in HashMap<Mutex>
- **Mitigation**: Test concurrent triggers

---

## Appendix: Database Schema Extensions

### Execution History Queries

Current schema supports both phases with existing tables:
- `actor_server_state` - Task state (pause, failures)
- `actor_server_executions` - Execution history

No schema changes required, but may want indexes:

```sql
-- Optimize execution history queries
CREATE INDEX IF NOT EXISTS idx_executions_task_started
ON actor_server_executions(task_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_executions_started
ON actor_server_executions(started_at DESC);

-- Optimize state queries by actor
CREATE INDEX IF NOT EXISTS idx_state_actor
ON actor_server_state(actor_name);
```

---

## Summary

**Phase 1: Observability** enables production monitoring through Prometheus metrics, health checks, and alerting integration. This provides operational visibility without changing actor behavior.

**Phase 2: HTTP API** enables programmatic control through REST endpoints for pausing, resuming, triggering, and querying actors. This provides operational flexibility without requiring database or SSH access.

Both phases are **optional enhancements** that add operational capabilities to the production-ready actor-server. They can be implemented independently or together based on operational needs.

**Estimated Total Time**: 3-5 days (or 12-18 hours for MVP versions)

**Key Benefits**:
- 📊 Real-time dashboards and alerting
- 🎮 Runtime control without restarts
- 🔍 Execution history via HTTP
- 🏥 Kubernetes-ready health checks
- 🔐 Secure API with authentication

**Next Steps**: Choose Phase 1, Phase 2, or both based on immediate operational needs.
