//! Metrics for LLM API calls.
//!
//! Provides OpenTelemetry-based metrics for tracking LLM API performance,
//! errors, and token usage across all provider implementations.

use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter},
    KeyValue,
};
use std::sync::OnceLock;

static METRICS: OnceLock<LlmMetrics> = OnceLock::new();

/// Metrics for LLM API interactions.
///
/// Tracks requests, errors, latency, and token usage for all LLM providers.
/// Metrics are labeled with provider (gemini, anthropic, etc.) and model name.
#[derive(Clone)]
pub struct LlmMetrics {
    /// Meter handle kept alive for metric instruments
    _meter: Meter,
    /// Total LLM API requests
    pub requests: Counter<u64>,
    /// Failed LLM API requests
    pub errors: Counter<u64>,
    /// LLM API call duration in seconds
    pub duration: Histogram<f64>,
    /// Total tokens used (prompt + completion)
    pub tokens_used: Counter<u64>,
    /// Prompt tokens used
    pub prompt_tokens: Counter<u64>,
    /// Completion tokens used
    pub completion_tokens: Counter<u64>,
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
                .with_description("Total tokens used (prompt + completion)")
                .build(),
            prompt_tokens: meter
                .u64_counter("llm.tokens.prompt")
                .with_description("Prompt tokens used")
                .build(),
            completion_tokens: meter
                .u64_counter("llm.tokens.completion")
                .with_description("Completion tokens used")
                .build(),
        }
    }

    /// Get the global LLM metrics instance.
    pub fn get() -> &'static Self {
        METRICS.get_or_init(Self::init)
    }

    /// Record a successful LLM API request.
    pub fn record_request(&self, provider: &str, model: &str, duration_secs: f64) {
        let labels = &[
            KeyValue::new("provider", provider.to_string()),
            KeyValue::new("model", model.to_string()),
        ];
        self.requests.add(1, labels);
        self.duration.record(duration_secs, labels);
    }

    /// Record a failed LLM API request.
    pub fn record_error(&self, provider: &str, model: &str, error_type: &str) {
        let labels = &[
            KeyValue::new("provider", provider.to_string()),
            KeyValue::new("model", model.to_string()),
            KeyValue::new("error_type", error_type.to_string()),
        ];
        self.errors.add(1, labels);
    }

    /// Record token usage from an LLM response.
    pub fn record_tokens(
        &self,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        total_tokens: u64,
    ) {
        let labels = &[KeyValue::new("model", model.to_string())];
        self.tokens_used.add(total_tokens, labels);
        self.prompt_tokens.add(prompt_tokens, labels);
        self.completion_tokens.add(completion_tokens, labels);
    }
}

impl Default for LlmMetrics {
    fn default() -> Self {
        Self::get().clone()
    }
}

/// Classify error type for metrics labeling.
///
/// Returns one of: "rate_limit", "auth", "network", "timeout", "invalid_request", "unknown"
pub fn classify_error(error: &dyn std::error::Error) -> &'static str {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("rate limit") || error_str.contains("429") {
        "rate_limit"
    } else if error_str.contains("auth") || error_str.contains("401") || error_str.contains("403")
    {
        "auth"
    } else if error_str.contains("network")
        || error_str.contains("connection")
        || error_str.contains("dns")
    {
        "network"
    } else if error_str.contains("timeout") {
        "timeout"
    } else if error_str.contains("400") || error_str.contains("invalid") {
        "invalid_request"
    } else {
        "unknown"
    }
}
