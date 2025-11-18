# botticelli_rate_limit

Rate limiting and retry logic for the Botticelli ecosystem.

## Overview

This crate provides configurable rate limiting and automatic retry logic for LLM API calls. It prevents exceeding provider rate limits and handles transient failures gracefully.

## Features

- **Multi-dimensional rate limiting**: RPM (requests per minute), TPM (tokens per minute), RPD (requests per day), concurrent requests
- **Provider-specific tiers**: Pre-configured limits for Gemini, Anthropic, OpenAI tiers
- **TOML configuration**: Load rate limits from configuration files
- **HTTP header detection**: Automatically detect rate limits from API responses
- **Exponential backoff**: Intelligent retry with jitter
- **Thread-safe**: Safe to use across multiple async tasks

## Rate Limiter

The core `RateLimiter<T>` type wraps any value with rate limiting:

```rust
use botticelli_rate_limit::RateLimiter;
use std::sync::Arc;

// Create a rate limiter with custom limits
let limiter = RateLimiter::new(
    my_client,
    60,     // requests per minute
    10_000, // tokens per minute  
    1440,   // requests per day
    5       // concurrent requests
);

// Access the wrapped value
limiter.get()?; // Returns Arc<T>
```

## Provider Tiers

Pre-configured rate limits for popular providers:

```rust
use botticelli_rate_limit::{GeminiTier, OpenAITier, AnthropicTier};

// Gemini Free tier
let tier = GeminiTier::free();
// RPM: 15, TPM: 1,000,000, RPD: 1,500, Concurrent: 1

// Gemini Pro tier  
let tier = GeminiTier::pro();
// RPM: 360, TPM: 4,000,000, RPD: 10,000, Concurrent: 10

// OpenAI Tier 1
let tier = OpenAITier::tier1();
// RPM: 500, TPM: 30,000, RPD: unlimited, Concurrent: 100

// Anthropic Tier 1
let tier = AnthropicTier::tier1();
// RPM: 50, TPM: 40,000, RPD: unlimited, Concurrent: 5
```

## TOML Configuration

Load rate limits from configuration files:

```toml
# config.toml
[rate_limits]
requests_per_minute = 60
tokens_per_minute = 10000
requests_per_day = 1440
max_concurrent_requests = 5

# Model-specific overrides
[rate_limits.models."gemini-1.5-flash"]
requests_per_minute = 15
tokens_per_minute = 1000000

[rate_limits.models."gemini-1.5-pro"]
requests_per_minute = 360
tokens_per_minute = 4000000
```

```rust
use botticelli_rate_limit::RateLimitConfig;

// Load from TOML
let config = RateLimitConfig::from_file("config.toml")?;

// Get limits for a specific model
let limits = config.get_limits("gemini-1.5-flash");
```

## Retry Logic

Automatic retry with exponential backoff for transient failures:

```rust
use botticelli_rate_limit::RetryableError;

// Errors that implement RetryableError will be retried
impl RetryableError for MyError {
    fn is_retryable(&self) -> bool {
        matches!(self, MyError::Timeout | MyError::RateLimited)
    }
}
```

The rate limiter automatically retries:
- Rate limit errors (429)
- Timeout errors
- Transient network errors

With exponential backoff:
- Initial delay: 1 second
- Multiplier: 2x
- Max delay: 60 seconds
- Max attempts: 3

## Header-Based Detection

Automatically detect rate limits from HTTP response headers:

```rust
use botticelli_rate_limit::detect_rate_limits;
use reqwest::Response;

let response: Response = client.get(url).send().await?;

// Extract rate limits from headers
if let Some(limits) = detect_rate_limits(&response) {
    println!("RPM: {:?}", limits.requests_per_minute);
    println!("TPM: {:?}", limits.tokens_per_minute);
    println!("RPD: {:?}", limits.requests_per_day);
}
```

Supported headers:
- `x-ratelimit-limit-requests`
- `x-ratelimit-limit-tokens`
- `x-ratelimit-remaining-requests`
- `x-ratelimit-remaining-tokens`

## Usage Example

Complete example with rate limiting:

```rust
use botticelli_rate_limit::{RateLimiter, GeminiTier};
use std::sync::Arc;

// Create client with rate limiting
let client = MyClient::new(api_key);
let tier = GeminiTier::free();

let rate_limited_client = RateLimiter::new(
    client,
    tier.rpm,
    tier.tpm,
    tier.rpd,
    tier.concurrent,
);

// Use the rate-limited client
let client = rate_limited_client.get()?;
let response = client.generate(request).await?;
```

## Design Philosophy

### Token-Bucket Algorithm

Uses the token bucket algorithm for smooth rate limiting:
- Requests consume tokens
- Tokens refill at a constant rate
- Burst capacity for occasional spikes

### Graceful Degradation

When limits are reached:
1. Wait for tokens to refill
2. Queue requests automatically
3. No explicit backpressure needed

### Provider-Agnostic

Works with any API that:
- Has rate limits
- Returns errors for limit violations
- Optionally provides rate limit headers

## Dependencies

- `governor` - Token bucket rate limiting
- `tokio-retry2` - Retry logic with exponential backoff
- `tokio` - Async runtime
- `serde` - Configuration serialization
- `toml` - TOML parsing

## Version

Current version: 0.2.0

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
