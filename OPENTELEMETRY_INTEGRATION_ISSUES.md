# OpenTelemetry Integration Issues and Resolution Strategy

**Status**: Research and Planning  
**Created**: 2025-11-29  
**Last Updated**: 2025-11-29

## Current State

We have **two different OpenTelemetry implementations** in the codebase:

### 1. `botticelli/src/observability.rs` (Stdout Development)
- Uses `opentelemetry_stdout::SpanExporter`
- Simple stdout exporter for local development
- Minimal setup with `simple_exporter`
- No metrics support
- **Purpose**: Quick development feedback

### 2. `botticelli_server/src/observability.rs` (Production OTLP)
- Uses `opentelemetry_otlp` with Tonic
- Full OTLP pipeline with traces + metrics
- Configurable endpoint, service metadata
- Batch export with Tokio runtime
- **Purpose**: Production deployment with external collectors

## The Problem

We have **architectural confusion**:

1. **Duplication**: Two initialization paths doing similar things differently
2. **Inconsistency**: Main binary uses stdout, server uses OTLP
3. **Feature Gaps**: Stdout path has no metrics, OTLP path unused
4. **Integration Unclear**: Which one should bots/narratives use?
5. **Testing Difficulty**: No clear dev vs prod separation strategy

## Root Cause Analysis

### Why Two Implementations?

Looking at the code history:
- `botticelli/observability.rs` created first for quick development
- `botticelli_server/observability.rs` created later for "production-ready" server
- Neither was completed or integrated into actual execution paths
- No decision made on unifying vs separating concerns

### What's Missing?

1. **No actual integration**: Neither is called from main execution paths
2. **No configuration**: Can't choose exporter at runtime
3. **No metrics instrumentation**: Code exists but no actual metrics collected
4. **No logs bridging**: OpenTelemetry logs not connected to `tracing` events
5. **No graceful shutdown**: Observability cleanup not in shutdown paths

## Research: Industry Best Practices

### Standard Approach (Rust Ecosystem)

The Rust OpenTelemetry ecosystem follows this pattern:

```rust
// Application code: Use tracing macros
#[instrument]
fn my_function() {
    info!("Processing request");
    // ... business logic
}

// Initialization: Bridge tracing → OpenTelemetry → Exporters
fn init_telemetry() {
    // 1. Create exporters (stdout, OTLP, Jaeger, etc)
    // 2. Build tracer/meter providers
    // 3. Set as global providers
    // 4. Create tracing-opentelemetry layer
    // 5. Initialize tracing subscriber with layer
}
```

**Key principle**: Application code stays exporter-agnostic. Configuration determines where telemetry goes.

### Configuration Strategy

Best practice uses **builder pattern** + **environment variables**:

```rust
ObservabilityBuilder::new()
    .with_service_name("botticelli")
    .with_traces(|traces| {
        traces
            .with_exporter(ExporterKind::from_env()) // OTLP vs Stdout
            .with_endpoint(env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
    })
    .with_metrics(|metrics| {
        metrics.enable_runtime_metrics()
               .enable_custom_metrics()
    })
    .with_logs(|logs| {
        logs.bridge_tracing_events()
    })
    .init()?
```

### Multi-Environment Pattern

Industry standard is **one implementation, multiple backends**:

- **Development**: `OTEL_EXPORTER=stdout` → Human-readable console output
- **Staging**: `OTEL_EXPORTER=otlp` + `OTEL_ENDPOINT=localhost:4317` → Local collector
- **Production**: `OTEL_EXPORTER=otlp` + `OTEL_ENDPOINT=otel-collector:4317` → Remote collector

This avoids code duplication while supporting all environments.

## Proposed Solution

### Option A: Unified Configurable Implementation (RECOMMENDED)

**Single implementation in `botticelli_core`** with runtime exporter selection:

```rust
// crates/botticelli_core/src/observability.rs
pub enum ExporterBackend {
    Stdout,
    Otlp { endpoint: String },
    Jaeger { endpoint: String },
}

pub struct ObservabilityConfig {
    pub service_name: String,
    pub traces: Option<TraceConfig>,
    pub metrics: Option<MetricsConfig>,
    pub logs: Option<LogsConfig>,
}

pub fn init_observability(config: ObservabilityConfig) -> Result<...> {
    // Unified initialization supporting all exporters
}
```

**Pros**:
- Single source of truth
- Easy to test (mock exporters)
- Supports all environments with same code
- Follows industry standards

**Cons**:
- More complex initial implementation
- Requires feature flags for optional exporters

### Option B: Keep Separate, Clarify Roles

Keep both but **clearly separate**:
- `botticelli::observability` → CLI tools, development, testing
- `botticelli_server::observability` → Production server deployments

**Pros**:
- Simpler, less refactoring
- Clear separation of concerns

**Cons**:
- Code duplication
- Inconsistent telemetry between CLI and server
- Harder to maintain

### Option C: Gradual Migration

1. **Phase 1**: Keep stdout for all current use cases
2. **Phase 2**: Add OTLP support as opt-in feature flag
3. **Phase 3**: Make exporters pluggable
4. **Phase 4**: Deprecate old implementations

**Pros**:
- Low risk, incremental
- Can validate each step

**Cons**:
- Takes longer
- Temporary complexity during migration

## Recommendation: Option A (Unified)

### Implementation Plan

#### Phase 1: Foundation (Week 1)
- [ ] Create `botticelli_core::observability` module
- [ ] Define `ObservabilityConfig` with builder
- [ ] Implement stdout exporter support
- [ ] Add environment variable parsing
- [ ] Write unit tests for config builder

#### Phase 2: Exporter Support (Week 1-2)
- [ ] Add OTLP exporter behind feature flag
- [ ] Implement exporter selection logic
- [ ] Add graceful shutdown handling
- [ ] Test with local OTLP collector (docker)

#### Phase 3: Metrics Integration (Week 2)
- [ ] Define standard metrics for narratives
- [ ] Define standard metrics for bots
- [ ] Implement metrics collection points
- [ ] Test metrics export

#### Phase 4: Logs Bridge (Week 2-3)
- [ ] Add `opentelemetry-appender-tracing`
- [ ] Bridge tracing events → OpenTelemetry logs
- [ ] Configure log levels and filtering
- [ ] Validate log correlation with traces

#### Phase 5: Integration (Week 3)
- [ ] Update CLI binary to use new observability
- [ ] Update bot server to use new observability
- [ ] Update narrative executor integration
- [ ] Add observability config to TOML files

#### Phase 6: Documentation (Week 3-4)
- [ ] Document configuration options
- [ ] Write deployment guide
- [ ] Create troubleshooting guide
- [ ] Add examples for common setups

#### Phase 7: Cleanup (Week 4)
- [ ] Remove old `botticelli/src/observability.rs`
- [ ] Remove old `botticelli_server/src/observability.rs`
- [ ] Update all imports
- [ ] Final testing and validation

## Key Design Decisions

### 1. Where Should Observability Code Live?

**Decision**: `botticelli_core/src/observability/`

**Rationale**:
- Core infrastructure, not business logic
- Needed by both CLI and server
- No circular dependencies (core depends on nothing)
- Aligns with existing pattern (error types in core)

### 2. How to Handle Feature Flags?

```toml
[features]
default = ["otel-stdout"]
otel-stdout = ["opentelemetry-stdout"]
otel-otlp = ["opentelemetry-otlp", "tonic"]
otel-jaeger = ["opentelemetry-jaeger"]
otel-all = ["otel-stdout", "otel-otlp", "otel-jaeger"]
```

**Rationale**:
- Stdout is zero-dep default for development
- OTLP is opt-in for production (requires network deps)
- Users can choose what they need

### 3. Configuration Source Priority?

1. Explicit config (programmatic)
2. Environment variables (`OTEL_*` standard)
3. TOML file (`botticelli.toml`)
4. Defaults (stdout, info level)

**Rationale**:
- Follows 12-factor app principles
- Standard OpenTelemetry env vars
- Easy override for different environments

### 4. What Metrics Should We Collect?

**Narrative Execution**:
- `narrative.executions.total` (counter)
- `narrative.execution.duration` (histogram)
- `narrative.acts.processed` (counter)
- `narrative.errors.total` (counter by error type)

**Bot Operations**:
- `bot.tasks.queued` (gauge)
- `bot.tasks.processed` (counter)
- `bot.task.duration` (histogram)
- `bot.api.calls` (counter by provider)
- `bot.api.tokens` (counter by provider)

**System**:
- `process.runtime.memory` (gauge)
- `process.runtime.cpu` (gauge)
- `db.connections.active` (gauge)
- `db.query.duration` (histogram)

### 5. Trace Span Strategy?

**Span Hierarchy**:
```
bot_server.run
├── bot.generation.tick
│   ├── narrative.execute (name=generation_carousel)
│   │   ├── narrative.act (name=generate)
│   │   │   ├── api.call (provider=gemini)
│   │   │   └── db.insert (table=potential_posts)
│   │   └── narrative.act (name=format_json)
│   └── bot.schedule_next
├── bot.curation.tick
│   └── ...
└── bot.posting.tick
    └── ...
```

**Rationale**:
- Clear hierarchy shows execution flow
- Each level adds context attributes
- Easy to filter and visualize
- Matches existing `#[instrument]` usage

## Testing Strategy

### Unit Tests
- Config builder logic
- Exporter selection
- Environment parsing
- Graceful shutdown

### Integration Tests
- Stdout exporter output validation
- OTLP export with test collector
- Metrics collection accuracy
- Trace propagation through async

### Local Development Testing
```bash
# Terminal 1: Start OTLP collector
docker run -p 4317:4317 -p 16686:16686 jaegertracing/all-in-one:latest

# Terminal 2: Run bot server with OTLP
OTEL_EXPORTER=otlp just bot-server

# Terminal 3: View traces
open http://localhost:16686
```

## Migration Risks and Mitigation

### Risk 1: Breaking Existing Tracing
**Impact**: High  
**Mitigation**:
- Keep existing `tracing` macros unchanged
- Layer-based approach is additive
- Test extensively before replacing old code

### Risk 2: Performance Overhead
**Impact**: Medium  
**Mitigation**:
- Use batch exporters (not synchronous)
- Sample traces in production (e.g., 10%)
- Measure overhead with benchmarks

### Risk 3: Configuration Complexity
**Impact**: Low  
**Mitigation**:
- Sensible defaults (stdout, info level)
- Clear documentation
- Validation with helpful error messages

### Risk 4: External Dependency Failures
**Impact**: Medium  
**Mitigation**:
- Graceful degradation if exporter fails
- Continue execution even if telemetry breaks
- Log telemetry errors separately

## Success Criteria

- [ ] Single observability implementation used everywhere
- [ ] Zero code changes needed for dev → prod deployment
- [ ] All narrative executions automatically traced
- [ ] All bot operations emit metrics
- [ ] Traces visible in Jaeger/SigNoz
- [ ] Metrics visible in Prometheus/dashboards
- [ ] Documentation complete
- [ ] Zero test failures
- [ ] Performance overhead < 5%

## Next Steps

1. **Decision**: Choose Option A, B, or C (recommend A)
2. **Prototype**: Build minimal unified config + stdout exporter
3. **Validate**: Test with existing bot server
4. **Iterate**: Add OTLP support
5. **Deploy**: Replace old implementations
6. **Document**: Write guides and examples

## References

- [OpenTelemetry Rust Docs](https://docs.rs/opentelemetry/)
- [Tracing-OpenTelemetry Integration](https://docs.rs/tracing-opentelemetry/)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/otel/)
- [OTLP Protocol](https://opentelemetry.io/docs/specs/otlp/)
- [SigNoz Rust Guide](https://signoz.io/docs/instrumentation/rust/)

## Appendix: Example Configurations

### Development (Stdout)
```toml
# botticelli.toml
[observability]
enabled = true
exporter = "stdout"
log_level = "debug"
```

### Staging (Local Collector)
```toml
[observability]
enabled = true
exporter = "otlp"
otlp_endpoint = "http://localhost:4317"
log_level = "info"
service_name = "botticelli-staging"
```

### Production (Remote Collector)
```toml
[observability]
enabled = true
exporter = "otlp"
otlp_endpoint = "https://otel-collector.example.com:4317"
log_level = "warn"
service_name = "botticelli-prod"
json_logs = true
trace_sampling_rate = 0.1  # Sample 10% of traces
```
