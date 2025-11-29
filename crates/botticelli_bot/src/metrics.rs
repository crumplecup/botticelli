//! Metrics collection for bot operations.

use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

/// Metrics collector for bot operations.
#[derive(Debug, Clone)]
pub struct BotMetrics {
    inner: Arc<BotMetricsInner>,
}

#[derive(Debug)]
struct BotMetricsInner {
    // Bot execution counts
    generation_executions: std::sync::atomic::AtomicU64,
    curation_executions: std::sync::atomic::AtomicU64,
    posting_executions: std::sync::atomic::AtomicU64,

    // Bot failure counts
    generation_failures: std::sync::atomic::AtomicU64,
    curation_failures: std::sync::atomic::AtomicU64,
    posting_failures: std::sync::atomic::AtomicU64,

    // Last execution timestamps
    generation_last_success: parking_lot::Mutex<Option<Instant>>,
    curation_last_success: parking_lot::Mutex<Option<Instant>>,
    posting_last_success: parking_lot::Mutex<Option<Instant>>,
}

impl Default for BotMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl BotMetrics {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(BotMetricsInner {
                generation_executions: std::sync::atomic::AtomicU64::new(0),
                curation_executions: std::sync::atomic::AtomicU64::new(0),
                posting_executions: std::sync::atomic::AtomicU64::new(0),
                generation_failures: std::sync::atomic::AtomicU64::new(0),
                curation_failures: std::sync::atomic::AtomicU64::new(0),
                posting_failures: std::sync::atomic::AtomicU64::new(0),
                generation_last_success: parking_lot::Mutex::new(None),
                curation_last_success: parking_lot::Mutex::new(None),
                posting_last_success: parking_lot::Mutex::new(None),
            }),
        }
    }

    /// Records a generation execution.
    pub fn record_generation_execution(&self) {
        self.inner
            .generation_executions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Records a generation success.
    pub fn record_generation_success(&self) {
        *self.inner.generation_last_success.lock() = Some(Instant::now());
    }

    /// Records a generation failure.
    pub fn record_generation_failure(&self) {
        self.inner
            .generation_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Records a curation execution.
    pub fn record_curation_execution(&self) {
        self.inner
            .curation_executions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Records a curation success.
    pub fn record_curation_success(&self) {
        *self.inner.curation_last_success.lock() = Some(Instant::now());
    }

    /// Records a curation failure.
    pub fn record_curation_failure(&self) {
        self.inner
            .curation_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Records a posting execution.
    pub fn record_posting_execution(&self) {
        self.inner
            .posting_executions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Records a posting success.
    pub fn record_posting_success(&self) {
        *self.inner.posting_last_success.lock() = Some(Instant::now());
    }

    /// Records a posting failure.
    pub fn record_posting_failure(&self) {
        self.inner
            .posting_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Gets generation execution count.
    pub fn generation_executions(&self) -> u64 {
        self.inner
            .generation_executions
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets generation failure count.
    pub fn generation_failures(&self) -> u64 {
        self.inner
            .generation_failures
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets time since last generation success.
    pub fn generation_time_since_success(&self) -> Option<std::time::Duration> {
        self.inner
            .generation_last_success
            .lock()
            .map(|instant| instant.elapsed())
    }

    /// Gets curation execution count.
    pub fn curation_executions(&self) -> u64 {
        self.inner
            .curation_executions
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets curation failure count.
    pub fn curation_failures(&self) -> u64 {
        self.inner
            .curation_failures
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets time since last curation success.
    pub fn curation_time_since_success(&self) -> Option<std::time::Duration> {
        self.inner
            .curation_last_success
            .lock()
            .map(|instant| instant.elapsed())
    }

    /// Gets posting execution count.
    pub fn posting_executions(&self) -> u64 {
        self.inner
            .posting_executions
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets posting failure count.
    pub fn posting_failures(&self) -> u64 {
        self.inner
            .posting_failures
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets time since last posting success.
    pub fn posting_time_since_success(&self) -> Option<std::time::Duration> {
        self.inner
            .posting_last_success
            .lock()
            .map(|instant| instant.elapsed())
    }

    /// Gets overall success rate (0.0 - 1.0).
    pub fn overall_success_rate(&self) -> f64 {
        let total_executions = self.generation_executions()
            + self.curation_executions()
            + self.posting_executions();
        let total_failures =
            self.generation_failures() + self.curation_failures() + self.posting_failures();

        if total_executions == 0 {
            return 1.0;
        }

        let successes = total_executions.saturating_sub(total_failures);
        successes as f64 / total_executions as f64
    }

    /// Creates a serializable snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            generation: BotMetricSnapshot {
                executions: self.generation_executions(),
                failures: self.generation_failures(),
                seconds_since_success: self
                    .generation_time_since_success()
                    .map(|d| d.as_secs()),
            },
            curation: BotMetricSnapshot {
                executions: self.curation_executions(),
                failures: self.curation_failures(),
                seconds_since_success: self.curation_time_since_success().map(|d| d.as_secs()),
            },
            posting: BotMetricSnapshot {
                executions: self.posting_executions(),
                failures: self.posting_failures(),
                seconds_since_success: self.posting_time_since_success().map(|d| d.as_secs()),
            },
            overall_success_rate: self.overall_success_rate(),
        }
    }
}

/// Serializable snapshot of bot metrics.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    /// Generation bot metrics
    pub generation: BotMetricSnapshot,
    /// Curation bot metrics
    pub curation: BotMetricSnapshot,
    /// Posting bot metrics
    pub posting: BotMetricSnapshot,
    /// Overall success rate across all bots
    pub overall_success_rate: f64,
}

/// Serializable snapshot of individual bot metrics.
#[derive(Debug, Clone, Serialize)]
pub struct BotMetricSnapshot {
    /// Number of executions
    pub executions: u64,
    /// Number of failures
    pub failures: u64,
    /// Seconds since last success
    pub seconds_since_success: Option<u64>,
}
