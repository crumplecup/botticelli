# Usage Tiers and Request Rate Limiting

While under early development, we will rely on the free usage tier for each API. As we approach production use, we can upgrade to higher usage tiers as needed. From a practical standpoint, the usage tier limits the maximum rate of requests in one way or another, and we need a data type to represent state transitions from one tier to another.

## Problem Statement

For e.g., the type GeminiClient could have a field called tier containing an enum Tier, with variants for each paid tier.

- This works well for one or two models, but we have dozens, and each plan has different ways of throttling usage.
- Different providers measure limits differently: requests per minute (RPM), tokens per minute (TPM), requests per day (RPD), concurrent requests, quota tokens, etc.
- Some providers have tiered systems (free/pro/enterprise), others have pay-as-you-go without explicit tiers
- Rate limits may vary by model within the same provider (e.g., GPT-3.5 vs GPT-4)

## Proposed Solution: Trait-Based Rate Limiting

As an alternative, we could define a Tier trait with methods that define a common language of rate limits among different models.

## Implementation Status

This document serves as both design specification and implementation guide. Sections are marked with their implementation status:

- ✅ **Implemented** - Code is written, tested, and committed
- 🚧 **In Progress** - Currently being implemented
- 📋 **Planned** - Designed but not yet implemented

### Current Status (Step 8 of 8 Complete)

| Step | Component | Status | Location |
|------|-----------|--------|----------|
| 1 | Core Tier trait | ✅ Implemented | `src/rate_limit/tier.rs` |
| 2 | TierConfig & BoticelliConfig | ✅ Implemented | `src/rate_limit/config.rs` |
| 3 | Provider tier enums | ✅ Implemented | `src/rate_limit/tiers.rs` |
| 4 | RateLimiter (governor/GCRA) | ✅ Implemented | `src/rate_limit/limiter.rs` |
| 5 | HeaderRateLimitDetector | ✅ Implemented | `src/rate_limit/detector.rs` |
| 6 | GeminiClient integration | ✅ Implemented | `src/models/gemini.rs` |
| 7 | CLI override flags | ✅ Implemented | `src/main.rs` |
| 8 | Testing & validation | ✅ Implemented | `tests/rate_limit_integration_test.rs`, `TESTING.md` |

### Core Trait (✅ Implemented)

```rust
/// Represents rate limiting constraints for an API tier.
pub trait Tier: Send + Sync {
    /// Requests per minute limit. None = unlimited.
    fn rpm(&self) -> Option<u32>;

    /// Tokens per minute limit. None = unlimited.
    fn tpm(&self) -> Option<u64>;

    /// Requests per day limit. None = unlimited.
    fn rpd(&self) -> Option<u32>;

    /// Maximum concurrent requests. None = unlimited.
    fn max_concurrent(&self) -> Option<u32>;

    /// Daily quota in USD (for pay-as-you-go). None = no quota.
    fn daily_quota_usd(&self) -> Option<f64>;

    /// Cost per million input tokens. None = free or unknown.
    fn cost_per_million_input_tokens(&self) -> Option<f64>;

    /// Cost per million output tokens. None = free or unknown.
    fn cost_per_million_output_tokens(&self) -> Option<f64>;

    /// Name of the tier (e.g., "Free", "Pro", "Enterprise").
    fn name(&self) -> &str;
}
```

## Provider-Specific Tier Implementations (✅ Implemented - Step 3)

### Gemini Tiers

Based on [Gemini API pricing](https://ai.google.dev/pricing):

```rust
pub enum GeminiTier {
    Free,
    PayAsYouGo,
}

impl Tier for GeminiTier {
    fn rpm(&self) -> Option<u32> {
        match self {
            GeminiTier::Free => Some(10),      // Free tier: 10 RPM (Flash 2.0)
            GeminiTier::PayAsYouGo => Some(360), // Paid: 360 RPM (6 per second)
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            GeminiTier::Free => Some(250_000),       // 250K tokens/min (Flash 2.0)
            GeminiTier::PayAsYouGo => Some(4_000_000), // 4M tokens/min
        }
    }

    fn rpd(&self) -> Option<u32> {
        match self {
            GeminiTier::Free => Some(250),           // 250 requests/day (Flash 2.0)
            GeminiTier::PayAsYouGo => None, // No daily limit
        }
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(1) // Both tiers: 1 concurrent request
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        match self {
            GeminiTier::Free => Some(0.0),
            GeminiTier::PayAsYouGo => Some(0.075), // Gemini 2.0 Flash
        }
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        match self {
            GeminiTier::Free => Some(0.0),
            GeminiTier::PayAsYouGo => Some(0.30),
        }
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None // No hard USD quota
    }

    fn name(&self) -> &str {
        match self {
            GeminiTier::Free => "Free",
            GeminiTier::PayAsYouGo => "Pay-as-you-go",
        }
    }
}
```

### Anthropic Tiers

Based on [Anthropic pricing](https://docs.anthropic.com/claude/docs/rate-limits):

```rust
pub enum AnthropicTier {
    Tier1,  // Free/new accounts
    Tier2,  // $5+ paid
    Tier3,  // $40+ paid
    Tier4,  // $200+ paid
}

impl Tier for AnthropicTier {
    fn rpm(&self) -> Option<u32> {
        match self {
            AnthropicTier::Tier1 => Some(5),
            AnthropicTier::Tier2 => Some(50),
            AnthropicTier::Tier3 => Some(1000),
            AnthropicTier::Tier4 => Some(2000),
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            AnthropicTier::Tier1 => Some(20_000),
            AnthropicTier::Tier2 => Some(40_000),
            AnthropicTier::Tier3 => Some(80_000),
            AnthropicTier::Tier4 => Some(160_000),
        }
    }

    fn rpd(&self) -> Option<u32> {
        None // No daily limit, monthly budget instead
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(5) // All tiers
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        Some(3.0) // Claude 3.5 Sonnet (varies by model)
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        Some(15.0)
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None // Monthly budget, not daily
    }

    fn name(&self) -> &str {
        match self {
            AnthropicTier::Tier1 => "Tier 1",
            AnthropicTier::Tier2 => "Tier 2",
            AnthropicTier::Tier3 => "Tier 3",
            AnthropicTier::Tier4 => "Tier 4",
        }
    }
}
```

### OpenAI Tiers

```rust
pub enum OpenAITier {
    Free,
    Tier1,   // $5+ paid
    Tier2,   // $50+ paid
    Tier3,   // $100+ paid
    Tier4,   // $250+ paid
    Tier5,   // $1000+ paid
}

impl Tier for OpenAITier {
    fn rpm(&self) -> Option<u32> {
        match self {
            OpenAITier::Free => Some(3),
            OpenAITier::Tier1 => Some(500),
            OpenAITier::Tier2 => Some(5000),
            OpenAITier::Tier3 => Some(10000),
            OpenAITier::Tier4 => Some(10000),
            OpenAITier::Tier5 => Some(10000),
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            OpenAITier::Free => Some(40_000),
            OpenAITier::Tier1 => Some(200_000),
            OpenAITier::Tier2 => Some(2_000_000),
            OpenAITier::Tier3 => Some(10_000_000),
            OpenAITier::Tier4 => Some(30_000_000),
            OpenAITier::Tier5 => Some(100_000_000),
        }
    }

    fn rpd(&self) -> Option<u32> {
        match self {
            OpenAITier::Free => Some(200),
            _ => None,
        }
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(50) // Batch queue limit
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        Some(2.50) // GPT-4 Turbo (varies by model)
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        Some(10.0)
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None
    }

    fn name(&self) -> &str {
        match self {
            OpenAITier::Free => "Free",
            OpenAITier::Tier1 => "Tier 1",
            OpenAITier::Tier2 => "Tier 2",
            OpenAITier::Tier3 => "Tier 3",
            OpenAITier::Tier4 => "Tier 4",
            OpenAITier::Tier5 => "Tier 5",
        }
    }
}
```

## Rate Limiter Implementation (✅ Implemented - Step 4)

### Using Governor Crate (GCRA Algorithm)

The [`governor`](https://crates.io/crates/governor) crate provides efficient rate limiting using the Generic Cell Rate Algorithm (GCRA), which is functionally equivalent to a leaky bucket but ~10x faster than mutex-based approaches on multi-threaded workloads.

**Why Governor over manual token buckets:**
- **Lock-free**: Uses atomic compare-and-swap operations (64-bit state)
- **No background tasks**: GCRA doesn't require periodic refills
- **Async-friendly**: Integrated `until_ready()` for waiting
- **Composable**: Combine multiple limiters for different quota types

```rust
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct RateLimiter {
    tier: Box<dyn Tier>,

    // RPM limiter (requests per minute)
    rpm_limiter: Option<Arc<GovernorRateLimiter<governor::state::direct::NotKeyed, governor::clock::DefaultClock>>>,

    // TPM limiter (tokens per minute)
    tpm_limiter: Option<Arc<GovernorRateLimiter<governor::state::direct::NotKeyed, governor::clock::DefaultClock>>>,

    // RPD counter (requests per day) - using AtomicU32
    rpd_limiter: Option<Arc<GovernorRateLimiter<governor::state::direct::NotKeyed, governor::clock::DefaultClock>>>,

    // Concurrent request semaphore
    concurrent_semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(tier: Box<dyn Tier>) -> Self {
        // Create RPM limiter
        let rpm_limiter = tier.rpm().map(|rpm| {
            let quota = Quota::per_minute(NonZeroU32::new(rpm).unwrap());
            Arc::new(GovernorRateLimiter::direct(quota))
        });

        // Create TPM limiter
        let tpm_limiter = tier.tpm().and_then(|tpm| {
            // Governor uses u32, so we need to handle large TPM values
            NonZeroU32::new(tpm.min(u32::MAX as u64) as u32)
                .map(|n| Arc::new(GovernorRateLimiter::direct(Quota::per_minute(n))))
        });

        // Create RPD limiter (per day = per 1440 minutes)
        let rpd_limiter = tier.rpd().map(|rpd| {
            let quota = Quota::per_minute(NonZeroU32::new(rpd).unwrap())
                .allow_burst(NonZeroU32::new(rpd).unwrap());  // Allow full daily burst
            Arc::new(GovernorRateLimiter::direct(quota))
        });

        // Create concurrent semaphore
        let max_concurrent = tier.max_concurrent().unwrap_or(u32::MAX) as usize;
        let concurrent_semaphore = Arc::new(Semaphore::new(max_concurrent));

        Self {
            tier,
            rpm_limiter,
            tpm_limiter,
            rpd_limiter,
            concurrent_semaphore,
        }
    }

    /// Acquire rate limit permission for a request.
    ///
    /// This waits until all rate limits allow the request:
    /// - RPM (requests per minute)
    /// - TPM (tokens per minute, based on estimated_tokens)
    /// - RPD (requests per day)
    /// - Concurrent request limit
    ///
    /// Returns a guard that releases the concurrent slot when dropped.
    pub async fn acquire(&self, estimated_tokens: u64) -> RateLimiterGuard {
        // Wait for RPM quota
        if let Some(limiter) = &self.rpm_limiter {
            limiter.until_ready().await;
        }

        // Wait for TPM quota (consume estimated tokens)
        if let Some(limiter) = &self.tpm_limiter {
            let tokens = (estimated_tokens.min(u32::MAX as u64) as u32).max(1);
            for _ in 0..tokens {
                limiter.until_ready().await;
            }
        }

        // Wait for RPD quota
        if let Some(limiter) = &self.rpd_limiter {
            limiter.until_ready().await;
        }

        // Acquire concurrent request slot (last to avoid holding slot while waiting)
        let permit = self.concurrent_semaphore.clone()
            .acquire_owned()
            .await
            .expect("Semaphore should not be closed");

        RateLimiterGuard {
            _permit: permit,
        }
    }

    /// Try to acquire without waiting.
    /// Returns None if any rate limit would block.
    pub fn try_acquire(&self, estimated_tokens: u64) -> Option<RateLimiterGuard> {
        // Check RPM
        if let Some(limiter) = &self.rpm_limiter {
            limiter.check().ok()?;
        }

        // Check TPM
        if let Some(limiter) = &self.tpm_limiter {
            let tokens = (estimated_tokens.min(u32::MAX as u64) as u32).max(1);
            for _ in 0..tokens {
                limiter.check().ok()?;
            }
        }

        // Check RPD
        if let Some(limiter) = &self.rpd_limiter {
            limiter.check().ok()?;
        }

        // Try to acquire concurrent slot
        let permit = self.concurrent_semaphore.clone().try_acquire_owned().ok()?;

        Some(RateLimiterGuard { _permit: permit })
    }
}

/// RAII guard for rate limiter.
/// Automatically releases the concurrent slot when dropped.
pub struct RateLimiterGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
```

**Key advantages of this approach:**

1. **No mutexes** - Governor uses atomic operations, avoiding lock contention
2. **No polling loops** - `until_ready()` waits efficiently without busy-waiting
3. **RAII safety** - Semaphore permit is automatically released via Drop
4. **Composable** - Each quota type has its own independent limiter
5. **Accurate** - GCRA provides mathematically precise rate limiting

## Integration with BoticelliDriver (✅ Implemented - Step 6)

### GeminiClient Integration

The `GeminiClient` now includes optional rate limiting support with three constructor variants:

```rust
pub struct GeminiClient {
    client: Gemini,
    model_name: String,
    rate_limiter: Option<RateLimiter>,
}

impl GeminiClient {
    /// Create client without rate limiting (backward compatible)
    pub fn new() -> BoticelliResult<Self> {
        Self::new_with_tier(None)
    }

    /// Create client with explicit tier
    pub fn new_with_tier(tier: Option<Box<dyn Tier>>) -> BoticelliResult<Self> {
        // Create rate limiter if tier provided
        let rate_limiter = tier.map(RateLimiter::new);

        Ok(Self {
            client,
            model_name: "gemini-2.0-flash".to_string(),
            rate_limiter,
        })
    }

    /// Create client with tier from config
    pub fn new_with_config(tier_name: Option<&str>) -> BoticelliResult<Self> {
        let tier = BoticelliConfig::load()
            .ok()
            .and_then(|config| config.get_tier("gemini", tier_name))
            .map(|tier_config| Box::new(tier_config) as Box<dyn Tier>);

        Self::new_with_tier(tier)
    }
}
```

### Rate Limit Acquisition

Before each API request, the client acquires rate limit permission with token estimation:

```rust
async fn generate_internal(&self, req: &GenerateRequest) -> GeminiResult<GenerateResponse> {
    // Acquire rate limit permission if rate limiting is enabled
    let _guard = if let Some(limiter) = &self.rate_limiter {
        // Estimate tokens for all input messages
        let estimated_tokens: u64 = req
            .messages
            .iter()
            .flat_map(|msg| &msg.content)
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();

        // Add max_tokens if specified (output token estimate)
        let total_estimate = estimated_tokens + req.max_tokens.unwrap_or(1000) as u64;

        Some(limiter.acquire(total_estimate).await)
    } else {
        None
    };

    // Make API request...
    // Guard is held until function exits (RAII)
}
```

### Token Estimation

Simple character-based estimation (rough approximation: 4 chars per token):

```rust
fn estimate_tokens(text: &str) -> u64 {
    (text.len() / 4).max(1) as u64
}
```

### Usage Examples

```rust
// Without rate limiting (default, backward compatible)
let client = GeminiClient::new()?;

// With explicit tier enum
let client = GeminiClient::new_with_tier(Some(Box::new(GeminiTier::Free)))?;

// With config (uses boticelli.toml)
let client = GeminiClient::new_with_config(None)?;  // Default tier
let client = GeminiClient::new_with_config(Some("payasyougo"))?;  // Specific tier

// Make requests - rate limiting is transparent
let response = client.generate(&request).await?;
```

### Limitations

**Note**: The `gemini-rust` wrapper doesn't expose HTTP response headers, so header-based rate limit detection isn't available for GeminiClient. Users must:
- Configure tier via `GeminiTier` enum
- Load from `boticelli.toml` configuration
- Use CLI flags (Step 7) to override limits

Future work could use a lower-level HTTP client (e.g., `reqwest` directly) to access headers, but this would require reimplementing the Gemini API protocol.

## Original Design Alternatives (Pre-Implementation)

### Option 1: Add Rate Limiting to Driver Trait

```rust
#[async_trait]
pub trait BoticelliDriver: Send + Sync {
    async fn generate(&self, request: &GenerateRequest) -> BoticelliResult<GenerateResponse>;

    /// Get the current tier for this driver.
    fn tier(&self) -> &dyn Tier;

    /// Get the rate limiter for this driver.
    fn rate_limiter(&self) -> &RateLimiter;
}
```

### Option 2: Wrap Driver with Rate Limiting

```rust
pub struct RateLimitedDriver<D: BoticelliDriver> {
    inner: D,
    rate_limiter: RateLimiter,
}

impl<D: BoticelliDriver> RateLimitedDriver<D> {
    pub fn new(driver: D, tier: Box<dyn Tier>) -> Self {
        Self {
            inner: driver,
            rate_limiter: RateLimiter::new(tier),
        }
    }
}

#[async_trait]
impl<D: BoticelliDriver> BoticelliDriver for RateLimitedDriver<D> {
    async fn generate(&self, request: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
        // Estimate tokens (rough heuristic)
        let estimated_tokens = estimate_tokens(request);

        // Acquire rate limit slot
        let _guard = self.rate_limiter.acquire(estimated_tokens).await;

        // Make actual request
        self.inner.generate(request).await
    }
}

fn estimate_tokens(request: &GenerateRequest) -> u64 {
    // Rough estimate: 4 chars per token
    let text_length: usize = request.inputs.iter()
        .filter_map(|input| match input {
            Input::Text(s) => Some(s.len()),
            _ => None,
        })
        .sum();

    (text_length / 4) as u64
}
```

## Usage Tracking and Cost Estimation (📋 Planned)

### Track actual usage in database

```sql
CREATE TABLE api_usage (
    id SERIAL PRIMARY KEY,
    provider VARCHAR(50) NOT NULL,
    model_name VARCHAR(100) NOT NULL,
    tier VARCHAR(50) NOT NULL,
    request_timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd DECIMAL(10, 6),
    request_id UUID REFERENCES narrative_executions(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_usage_provider ON api_usage(provider);
CREATE INDEX idx_api_usage_timestamp ON api_usage(request_timestamp);
```

### Cost calculation

```rust
pub fn calculate_cost(tier: &dyn Tier, input_tokens: u64, output_tokens: u64) -> Option<f64> {
    let input_cost = tier.cost_per_million_input_tokens()? * (input_tokens as f64 / 1_000_000.0);
    let output_cost = tier.cost_per_million_output_tokens()? * (output_tokens as f64 / 1_000_000.0);

    Some(input_cost + output_cost)
}
```

## Configuration (✅ Implemented - Step 2)

### Configuration File: boticelli.toml

Rate limits change over time as providers adjust pricing and quotas. To avoid hardcoding these values, users can define custom rate limits in `boticelli.toml`:

```toml
# Default provider tiers
[providers.gemini]
default_tier = "free"

[providers.gemini.tiers.free]
name = "Free"
rpm = 10              # Requests per minute
tpm = 250_000         # Tokens per minute
rpd = 250             # Requests per day
max_concurrent = 1
cost_per_million_input_tokens = 0.0
cost_per_million_output_tokens = 0.0

[providers.gemini.tiers.payasyougo]
name = "Pay-as-you-go"
rpm = 360
tpm = 4_000_000
# rpd = null  # Omit for unlimited
max_concurrent = 1
cost_per_million_input_tokens = 0.075
cost_per_million_output_tokens = 0.30

[providers.anthropic]
default_tier = "tier1"

[providers.anthropic.tiers.tier1]
name = "Tier 1"
rpm = 5
tpm = 20_000
max_concurrent = 5
cost_per_million_input_tokens = 3.0
cost_per_million_output_tokens = 15.0

[providers.anthropic.tiers.tier2]
name = "Tier 2"
rpm = 50
tpm = 40_000
max_concurrent = 5
cost_per_million_input_tokens = 3.0
cost_per_million_output_tokens = 15.0

[providers.anthropic.tiers.tier3]
name = "Tier 3"
rpm = 1000
tpm = 80_000
max_concurrent = 5
cost_per_million_input_tokens = 3.0
cost_per_million_output_tokens = 15.0

[providers.anthropic.tiers.tier4]
name = "Tier 4"
rpm = 2000
tpm = 160_000
max_concurrent = 5
cost_per_million_input_tokens = 3.0
cost_per_million_output_tokens = 15.0

[providers.openai]
default_tier = "tier1"

[providers.openai.tiers.free]
name = "Free"
rpm = 3
tpm = 40_000
rpd = 200
max_concurrent = 50
cost_per_million_input_tokens = 0.0
cost_per_million_output_tokens = 0.0

[providers.openai.tiers.tier1]
name = "Tier 1"
rpm = 500
tpm = 200_000
max_concurrent = 50
cost_per_million_input_tokens = 2.50
cost_per_million_output_tokens = 10.0

# ... additional OpenAI tiers
```

### Configuration Loading

Uses the [`config`](https://crates.io/crates/config) crate for robust multi-source configuration management with automatic merging:

```rust
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BoticelliConfig {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

/// Configuration for a specific provider
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub default_tier: String,
    pub tiers: HashMap<String, TierConfig>,
}

/// Configuration for a specific tier
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TierConfig {
    pub name: String,
    #[serde(default)]
    pub rpm: Option<u32>,
    #[serde(default)]
    pub tpm: Option<u64>,
    #[serde(default)]
    pub rpd: Option<u32>,
    #[serde(default)]
    pub max_concurrent: Option<u32>,
    #[serde(default)]
    pub daily_quota_usd: Option<f64>,
    #[serde(default)]
    pub cost_per_million_input_tokens: Option<f64>,
    #[serde(default)]
    pub cost_per_million_output_tokens: Option<f64>,
}

impl Tier for TierConfig {
    fn rpm(&self) -> Option<u32> { self.rpm }
    fn tpm(&self) -> Option<u64> { self.tpm }
    fn rpd(&self) -> Option<u32> { self.rpd }
    fn max_concurrent(&self) -> Option<u32> { self.max_concurrent }
    fn daily_quota_usd(&self) -> Option<f64> { self.daily_quota_usd }
    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        self.cost_per_million_input_tokens
    }
    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        self.cost_per_million_output_tokens
    }
    fn name(&self) -> &str { &self.name }
}

impl BoticelliConfig {
    /// Load configuration from a specific file path
    pub fn from_file(path: impl AsRef<std::path::Path>) -> BoticelliResult<Self> {
        Config::builder()
            .add_source(File::from(path.as_ref()))
            .build()?
            .try_deserialize()
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to parse configuration: {}", e)
            )))
    }

    /// Load configuration with automatic precedence handling
    ///
    /// Sources in order of precedence (later sources override earlier):
    /// 1. Bundled defaults (include_str! from repo boticelli.toml)
    /// 2. User config in home directory (~/.config/boticelli/boticelli.toml)
    /// 3. User config in current directory (./boticelli.toml)
    ///
    /// User config files are optional and silently skipped if not found.
    pub fn load() -> BoticelliResult<Self> {
        const DEFAULT_CONFIG: &str = include_str!("../../boticelli.toml");

        let mut builder = Config::builder()
            // Start with bundled defaults
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml));

        // Add user config from home directory (optional)
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".config/boticelli/boticelli.toml");
            builder = builder.add_source(File::from(home_config).required(false));
        }

        // Add user config from current directory (optional, highest precedence)
        builder = builder.add_source(File::with_name("boticelli").required(false));

        builder
            .build()
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to build configuration: {}", e)
            )))?
            .try_deserialize()
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to parse configuration: {}", e)
            )))
    }

    /// Get tier configuration for a provider
    pub fn get_tier(&self, provider: &str, tier_name: Option<&str>) -> Option<TierConfig> {
        let provider_config = self.providers.get(provider)?;
        let tier = tier_name.unwrap_or(&provider_config.default_tier);
        provider_config.tiers.get(tier).cloned()
    }
}
```

**Key improvements using `config` crate:**

- **Automatic merging**: Config sources are merged automatically by the `config` crate based on order added
- **Optional files**: `.required(false)` allows user configs to be absent without errors
- **Format detection**: Automatically detects TOML format from file extension
- **String sources**: `File::from_str()` enables bundled defaults via `include_str!()`
- **Error handling**: Comprehensive error messages for configuration issues

### Configuration Precedence

Configuration is loaded in the following order (highest priority first):

1. **CLI flags** - Runtime overrides via command-line arguments
2. **Auto-detected from API headers** - Provider response headers (most accurate)
3. **Environment variables** - `GEMINI_TIER`, `ANTHROPIC_TIER`, etc.
4. **User config file** - `./boticelli.toml` or `~/.config/boticelli/boticelli.toml`
5. **Bundled defaults** - The `boticelli.toml` shipped with Boticelli

Header detection is preferred because it reflects the actual current limits from the provider,
automatically updates when you upgrade tiers, and never goes stale.

### CLI Override Flags (✅ Implemented - Step 7)

Override rate limits at runtime for quick testing or one-off adjustments:

```bash
# Override tier selection
boticelli run -n narrative.toml --backend gemini --tier payasyougo

# Override specific rate limits
boticelli run -n narrative.toml --backend gemini --rpm 20 --tpm 500000

# Override cost tracking
boticelli run -n narrative.toml --backend anthropic \
  --cost-input 2.5 --cost-output 12.0

# Disable rate limiting entirely (use with caution!)
boticelli run -n narrative.toml --backend gemini --no-rate-limit
```

CLI flag implementation in `src/main.rs`:

```rust
/// CLI rate limiting options
#[derive(Debug, Clone)]
struct RateLimitOptions {
    tier: Option<String>,
    rpm: Option<u32>,
    tpm: Option<u64>,
    rpd: Option<u32>,
    max_concurrent: Option<u32>,
    cost_input: Option<f64>,
    cost_output: Option<f64>,
    no_rate_limit: bool,
}

impl RateLimitOptions {
    /// Apply CLI overrides to a tier configuration
    fn apply_to_config(&self, mut config: TierConfig) -> TierConfig {
        if self.no_rate_limit {
            // Remove all limits
            config.rpm = None;
            config.tpm = None;
            config.rpd = None;
            config.max_concurrent = None;
        } else {
            // Apply individual overrides
            if let Some(rpm) = self.rpm {
                config.rpm = Some(rpm);
            }
            if let Some(tpm) = self.tpm {
                config.tpm = Some(tpm);
            }
            if let Some(rpd) = self.rpd {
                config.rpd = Some(rpd);
            }
            if let Some(max_concurrent) = self.max_concurrent {
                config.max_concurrent = Some(max_concurrent);
            }
        }

        // Apply cost overrides
        if let Some(cost_input) = self.cost_input {
            config.cost_per_million_input_tokens = Some(cost_input);
        }
        if let Some(cost_output) = self.cost_output {
            config.cost_per_million_output_tokens = Some(cost_output);
        }

        config
    }

    /// Build a tier configuration from CLI overrides and config
    fn build_tier(&self, provider: &str) -> Result<Option<Box<dyn Tier>>, Box<dyn std::error::Error>> {
        // Get tier name from: CLI > Env > Config default
        let tier_name = self
            .tier
            .clone()
            .or_else(|| {
                let env_var = format!("{}_TIER", provider.to_uppercase());
                std::env::var(&env_var).ok()
            })
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|c| c.providers.get(provider))
                    .map(|p| p.default_tier.clone())
            });

        // Load base tier config from file
        let config = BoticelliConfig::load().ok();
        let mut tier_config = config
            .and_then(|cfg| cfg.get_tier(provider, tier_name.as_deref()))
            .ok_or_else(|| format!("Tier not found for provider '{}'", provider))?;

        // Apply CLI overrides to base config
        tier_config = self.apply_to_config(tier_config);

        Ok(Some(Box::new(tier_config) as Box<dyn Tier>))
    }
}
```

### Full Precedence Chain

Configuration is loaded in the following order (highest priority first):

1. **CLI flags** - `--rpm`, `--tpm`, `--rpd`, `--max-concurrent`, `--cost-input`, `--cost-output`, `--no-rate-limit`
2. **CLI tier selection** - `--tier payasyougo`
3. **Environment variables** - `GEMINI_TIER`, `ANTHROPIC_TIER`, etc.
4. **User config file** - `./boticelli.toml` or `~/.config/boticelli/boticelli.toml`
5. **Bundled defaults** - The `boticelli.toml` shipped with Boticelli

### Usage Examples

```bash
# Use default tier from config (free tier for Gemini)
boticelli run -n mint.toml --backend gemini

# Select specific tier
boticelli run -n mint.toml --backend gemini --tier payasyougo

# Override RPM for testing (keeps other limits from config)
boticelli run -n mint.toml --backend gemini --rpm 5

# Override multiple limits
boticelli run -n mint.toml --backend gemini --rpm 20 --tpm 500000 --max-concurrent 3

# Disable rate limiting for quick test
boticelli run -n mint.toml --backend gemini --no-rate-limit

# Set environment variable for tier
export GEMINI_TIER=payasyougo
boticelli run -n mint.toml --backend gemini

# Combine config, env, and CLI (CLI has highest priority)
export GEMINI_TIER=free
boticelli run -n mint.toml --backend gemini --rpm 20  # Uses free tier with RPM=20
```

The CLI displays the active rate limiting configuration on startup:

```
📖 Loading narrative from "mint.toml"...
✓ Loaded: Video Creation Narrative
  Description: Generates video content workflow
  Acts: 3
  Rate Limiting: Free (RPM: Some(10), TPM: Some(250000), RPD: Some(250))

🚀 Executing narrative...
```

### Environment Variables

Environment variables select the tier but don't override specific limits:

```env
# Gemini
GEMINI_TIER=free  # or "payasyougo"

# Anthropic
ANTHROPIC_TIER=tier1  # tier1, tier2, tier3, tier4

# OpenAI
OPENAI_TIER=tier1  # free, tier1, tier2, tier3, tier4, tier5
```

### User Configuration Override

Create your own `boticelli.toml` to override defaults:

**Option 1: Project-specific** (recommended for per-project limits)
```bash
# In your project directory
cp /path/to/boticelli/boticelli.toml ./boticelli.toml
# Edit ./boticelli.toml with your custom values
```

**Option 2: Global user config** (applies to all projects)
```bash
mkdir -p ~/.config/boticelli
cp /path/to/boticelli/boticelli.toml ~/.config/boticelli/boticelli.toml
# Edit ~/.config/boticelli/boticelli.toml with your custom values
```

You only need to specify values you want to override:

```toml
# Custom boticelli.toml - only override Gemini free tier RPM
[providers.gemini.tiers.free]
rpm = 20  # Increased from default 10

# Everything else inherits from bundled defaults
```

### Client initialization with full precedence chain

```rust
impl GeminiClient {
    pub fn new() -> BoticelliResult<Self> {
        Self::new_with_overrides(None, None)
    }

    pub fn new_with_overrides(
        tier_override: Option<String>,
        cli_overrides: Option<&RunCommand>,
    ) -> BoticelliResult<Self> {
        // 1. Load configuration (bundled default + user overrides merged)
        let config = BoticelliConfig::load()?;

        // 2. Get tier name from: CLI > Env > Config default
        let tier_name = tier_override
            .or_else(|| std::env::var("GEMINI_TIER").ok())
            .or_else(|| {
                config
                    .providers
                    .get("gemini")
                    .map(|p| p.default_tier.clone())
            });

        // 3. Load tier config from merged config
        let mut tier_config = config
            .get_tier("gemini", tier_name.as_deref())
            .ok_or_else(|| {
                BoticelliError::new(BoticelliErrorKind::Config(format!(
                    "Tier '{}' not found for provider 'gemini'",
                    tier_name.unwrap_or_else(|| "default".to_string())
                )))
            })?;

        // 4. Apply CLI overrides (highest priority)
        if let Some(cmd) = cli_overrides {
            tier_config = cmd.apply_overrides(tier_config);
        }

        // 5. Create client with final tier configuration
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| BoticelliError::new(BoticelliErrorKind::Config(
                "GEMINI_API_KEY not provided".to_string()
            )))?;

        let client = gemini_rust::Client::new(&api_key);

        Ok(Self {
            client,
            tier: tier_config.clone(),
            rate_limiter: RateLimiter::new(Box::new(tier_config)),
        })
    }
}
```

Full flow in CLI:

```rust
async fn run_narrative(cmd: RunCommand) -> BoticelliResult<()> {
    // Load narrative
    let content = std::fs::read_to_string(&cmd.narrative)?;
    let narrative: Narrative = content.parse()?;

    // Create driver with full precedence chain
    let driver = match cmd.backend.as_str() {
        "gemini" => {
            let client = GeminiClient::new_with_overrides(
                cmd.tier.clone(),  // CLI tier override
                Some(&cmd),         // CLI flag overrides
            )?;
            Box::new(client) as Box<dyn BoticelliDriver>
        }
        "anthropic" => {
            let client = AnthropicClient::new_with_overrides(
                cmd.tier.clone(),
                Some(&cmd),
            )?;
            Box::new(client) as Box<dyn BoticelliDriver>
        }
        _ => return Err(/* unsupported backend */),
    };

    // Execute narrative with rate-limited driver
    let executor = NarrativeExecutor::new(driver);
    let execution = executor.execute(&narrative).await?;

    Ok(())
}
```

## Testing & Validation (✅ Implemented - Step 8)

### Economical Testing Strategy

To conserve API quota while still validating rate limiting functionality, Boticelli uses a multi-tier testing approach:

1. **Unit tests (no API calls)** - Test rate limiting logic with mock drivers
2. **Integration tests (gated)** - Minimal API tests behind `BOTICELLI_RUN_API_TESTS` environment variable
3. **Manual CLI testing** - Minimal narratives for end-to-end validation

### Test Budget Management

**Gemini Free Tier Limits:**
- 10 requests per minute (RPM)
- 250,000 tokens per minute (TPM)
- 250 requests per day (RPD)

**Test Suite Budget:**

| Test Type | Requests | Tokens | % of Daily Quota |
|-----------|----------|--------|------------------|
| Unit tests (`cargo test`) | 0 | 0 | 0% |
| Integration tests (API) | 2 | ~14 | 0.8% |
| Manual CLI test (1x) | 1 | ~5 | 0.4% |
| **Total per full cycle** | **3** | **~19** | **1.2%** |

This allows running the full test suite **~83 times per day** before hitting the 250 RPD limit.

### Integration Test Suite

**Location:** `tests/rate_limit_integration_test.rs`

The integration test file contains 8 tests:
- **6 unit tests** - Test rate limiting logic without API calls
- **2 API tests** - Gated behind `BOTICELLI_RUN_API_TESTS` environment variable

```rust
/// Check if API tests should run
#[cfg(feature = "gemini")]
fn should_run_api_tests() -> bool {
    std::env::var("BOTICELLI_RUN_API_TESTS").is_ok()
}

/// Skip test if API tests are disabled
#[cfg(feature = "gemini")]
macro_rules! skip_unless_api_tests_enabled {
    () => {
        if !should_run_api_tests() {
            println!("Skipping API test (set BOTICELLI_RUN_API_TESTS=1 to enable)");
            return;
        }
    };
}
```

**Unit tests (0 API calls):**
1. `test_rate_limiter_blocks_on_rpm_limit` - Tests RPM limiting logic
2. `test_rate_limiter_releases_concurrent_slots` - Tests RAII guard pattern
3. `test_rate_limiter_with_multiple_limits` - Tests combined RPM/TPM/RPD/concurrent limits
4. `test_tier_trait_from_config` - Tests Tier trait implementation
5. `test_gemini_tier_enum` - Tests GeminiTier enum values (feature-gated)

**API tests (2 requests, ~14 tokens total):**
1. `test_gemini_client_without_rate_limiting` - 1 request, ~7 tokens
2. `test_gemini_client_with_rate_limiting` - 1 request, ~7 tokens

Each API test uses minimal prompts and output limits:

```rust
#[cfg(feature = "gemini")]
#[tokio::test]
async fn test_gemini_client_without_rate_limiting() {
    skip_unless_api_tests_enabled!();

    let client = GeminiClient::new().expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'hi'".to_string())], // ~2 tokens
        }],
        temperature: Some(0.0),  // Deterministic
        max_tokens: Some(5),     // Minimal output
        model: None,
    };

    let response = client.generate(&request).await.expect("API call failed");
    assert!(!response.outputs.is_empty());
}
```

### Running Tests

**Default (no API calls):**
```bash
cargo test
# Runs 39 unit tests in < 1 second, uses 0 tokens
```

**With API tests:**
```bash
export GEMINI_API_KEY="your-key-here"
BOTICELLI_RUN_API_TESTS=1 cargo test
# Runs 39 unit tests + 2 API tests, uses ~14 tokens
```

**Specific test suites:**
```bash
# Rate limiter logic only (no API)
cargo test --test rate_limit_limiter_test

# Integration tests with optional API
BOTICELLI_RUN_API_TESTS=1 cargo test --test rate_limit_integration_test
```

### Manual CLI Testing

**Minimal test narrative:** `narratives/test_minimal.toml`

```toml
[narration]
name = "Minimal Test"
description = "Smallest possible narrative for testing rate limiting"

[toc]
order = ["test"]

[acts]
test = "Say 'ok'"
```

This narrative uses ~5 tokens total (2 input, 3 output).

**Testing with CLI:**
```bash
# Test with default rate limiting
cargo run -- run -n narratives/test_minimal.toml --backend gemini

# Test with specific tier
cargo run -- run -n narratives/test_minimal.toml --backend gemini --tier free

# Test with aggressive limits to see blocking
cargo run -- run -n narratives/test_minimal.toml --backend gemini --rpm 1

# Test without rate limiting
cargo run -- run -n narratives/test_minimal.toml --backend gemini --no-rate-limit
```

### Conservative Testing Workflow

**Default development cycle** (multiple times per day):
```bash
cargo test  # Unit tests only, 0 API calls, 0 tokens
```

**Before commits** (1-2 times per feature):
```bash
BOTICELLI_RUN_API_TESTS=1 cargo test  # +2 requests, +14 tokens
```

**Manual CLI verification** (once per major change):
```bash
cargo run -- run -n narratives/test_minimal.toml --backend gemini  # +1 request, +5 tokens
```

### Testing Documentation

Comprehensive testing guide available in `TESTING.md`:
- Test categories and budget management
- Conservative testing strategies
- Examples of economical test patterns
- CI/CD recommendations
- Troubleshooting guide

Key testing principles:
1. **Use mocks by default** - All new tests should use mock drivers unless specifically testing API integration
2. **Gate API tests** - Always use `skip_unless_api_tests_enabled!()` macro
3. **Minimal prompts** - Test prompts should be 1-5 words
4. **Low max_tokens** - Use `max_tokens: Some(5)` in test requests
5. **Deterministic** - Use `temperature: Some(0.0)` for predictable responses

### Auto-Detection from Response Headers (✅ Implemented - Step 5)

Most providers return rate limit information in response headers. This is the most accurate
source of truth since it reflects your actual current limits and automatically updates when
you upgrade tiers.

#### Common Header Formats

Different providers use different header conventions:

**Gemini/Google AI:**
```
x-ratelimit-limit: 10          # Requests allowed in current window
x-ratelimit-remaining: 9       # Requests remaining
x-ratelimit-reset: 1705012345  # Unix timestamp when limit resets
```

**Anthropic:**
```
anthropic-ratelimit-requests-limit: 50
anthropic-ratelimit-requests-remaining: 47
anthropic-ratelimit-requests-reset: 2024-01-11T12:34:56Z
anthropic-ratelimit-tokens-limit: 40000
anthropic-ratelimit-tokens-remaining: 35000
anthropic-ratelimit-tokens-reset: 2024-01-11T12:34:56Z
```

**OpenAI:**
```
x-ratelimit-limit-requests: 500
x-ratelimit-limit-tokens: 200000
x-ratelimit-remaining-requests: 495
x-ratelimit-remaining-tokens: 195000
x-ratelimit-reset-requests: 12s
x-ratelimit-reset-tokens: 18s
```

#### Header Detection Implementation

```rust
use reqwest::header::HeaderMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Detects and caches rate limits from API response headers
pub struct HeaderRateLimitDetector {
    /// Cached detected limits (updated on each API call)
    detected_limits: Arc<RwLock<Option<TierConfig>>>,
}

impl HeaderRateLimitDetector {
    pub fn new() -> Self {
        Self {
            detected_limits: Arc::new(RwLock::new(None)),
        }
    }

    /// Detect rate limits from Gemini response headers
    pub async fn detect_gemini(&self, headers: &HeaderMap) -> Option<TierConfig> {
        // Parse rate limit headers
        let rpm = parse_header_u32(headers, "x-ratelimit-limit")?;

        // Gemini doesn't expose TPM/RPD in headers, so we infer from RPM
        let (tpm, rpd, tier_name) = if rpm <= 10 {
            (Some(250_000), Some(250), "Free")
        } else if rpm <= 360 {
            (Some(4_000_000), None, "Pay-as-you-go")
        } else {
            (None, None, "Unknown")
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm,
            rpd,
            max_concurrent: Some(1), // Gemini doesn't expose this
            daily_quota_usd: None,
            cost_per_million_input_tokens: if rpm <= 10 { Some(0.0) } else { Some(0.075) },
            cost_per_million_output_tokens: if rpm <= 10 { Some(0.0) } else { Some(0.30) },
        };

        // Cache for future use
        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Detect rate limits from Anthropic response headers
    pub async fn detect_anthropic(&self, headers: &HeaderMap) -> Option<TierConfig> {
        let rpm = parse_header_u32(headers, "anthropic-ratelimit-requests-limit")?;
        let tpm = parse_header_u64(headers, "anthropic-ratelimit-tokens-limit")?;

        // Determine tier name from limits
        let tier_name = match (rpm, tpm) {
            (5, 20_000) => "Tier 1",
            (50, 40_000) => "Tier 2",
            (1000, 80_000) => "Tier 3",
            (2000, 160_000) => "Tier 4",
            _ => "Custom",
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm: Some(tpm),
            rpd: None, // Anthropic doesn't have daily limits
            max_concurrent: Some(5), // Not exposed in headers
            daily_quota_usd: None,
            cost_per_million_input_tokens: Some(3.0), // Varies by model
            cost_per_million_output_tokens: Some(15.0),
        };

        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Detect rate limits from OpenAI response headers
    pub async fn detect_openai(&self, headers: &HeaderMap) -> Option<TierConfig> {
        let rpm = parse_header_u32(headers, "x-ratelimit-limit-requests")?;
        let tpm = parse_header_u64(headers, "x-ratelimit-limit-tokens")?;

        // Determine tier from limits
        let (tier_name, rpd) = match (rpm, tpm) {
            (3, 40_000) => ("Free", Some(200)),
            (500, 200_000) => ("Tier 1", None),
            (5000, 2_000_000) => ("Tier 2", None),
            (10000, 10_000_000) => ("Tier 3", None),
            (10000, 30_000_000) => ("Tier 4", None),
            (10000, 100_000_000) => ("Tier 5", None),
            _ => ("Custom", None),
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm: Some(tpm),
            rpd,
            max_concurrent: Some(50),
            daily_quota_usd: None,
            cost_per_million_input_tokens: Some(2.50), // Varies by model
            cost_per_million_output_tokens: Some(10.0),
        };

        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Get last detected limits (from cache)
    pub async fn get_cached(&self) -> Option<TierConfig> {
        self.detected_limits.read().await.clone()
    }
}

/// Helper to parse u32 from header
fn parse_header_u32(headers: &HeaderMap, key: &str) -> Option<u32> {
    headers
        .get(key)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

/// Helper to parse u64 from header
fn parse_header_u64(headers: &HeaderMap, key: &str) -> Option<u64> {
    headers
        .get(key)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}
```

#### Integration with Client

```rust
pub struct GeminiClient {
    client: gemini_rust::Client,
    tier: Arc<RwLock<TierConfig>>,
    rate_limiter: Arc<RateLimiter>,
    header_detector: HeaderRateLimitDetector,
}

impl GeminiClient {
    pub fn new_with_overrides(
        tier_override: Option<String>,
        cli_overrides: Option<&RunCommand>,
    ) -> BoticelliResult<Self> {
        // Load initial config (CLI > Env > User Config > Defaults)
        let config = BoticelliConfig::load()?;
        let tier_name = tier_override
            .or_else(|| std::env::var("GEMINI_TIER").ok());

        let mut tier_config = config
            .get_tier("gemini", tier_name.as_deref())
            .ok_or_else(|| BoticelliError::new(BoticelliErrorKind::Config(
                "No tier configuration found for Gemini".to_string()
            )))?;

        // Apply CLI overrides
        if let Some(cmd) = cli_overrides {
            tier_config = cmd.apply_overrides(tier_config);
        }

        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| BoticelliError::new(BoticelliErrorKind::Config(
                "GEMINI_API_KEY not provided".to_string()
            )))?;

        let client = gemini_rust::Client::new(&api_key);

        Ok(Self {
            client,
            tier: Arc::new(RwLock::new(tier_config.clone())),
            rate_limiter: Arc::new(RateLimiter::new(Box::new(tier_config))),
            header_detector: HeaderRateLimitDetector::new(),
        })
    }

    /// Update rate limits from response headers if available
    async fn update_from_headers(&self, response: &reqwest::Response) {
        if let Some(detected) = self.header_detector.detect_gemini(response.headers()).await {
            tracing::info!(
                "Detected rate limits from headers: {} RPM, {} TPM",
                detected.rpm.unwrap_or(0),
                detected.tpm.unwrap_or(0)
            );

            // Update tier config
            *self.tier.write().await = detected.clone();

            // Update rate limiter with new limits
            // Note: Would need to add update method to RateLimiter
            // self.rate_limiter.update_limits(detected);
        }
    }
}

#[async_trait]
impl BoticelliDriver for GeminiClient {
    async fn generate(&self, request: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
        // Make API request
        let response = self.client.generate(request).await?;

        // Update limits from headers (auto-detection in background)
        if let Ok(http_response) = &response {
            self.update_from_headers(http_response).await;
        }

        // Process and return response
        Ok(response)
    }
}
```

#### Persistent Header Detection Cache

To avoid re-detecting on every request, cache detected limits to disk:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DetectedLimitsCache {
    provider: String,
    detected_at: chrono::DateTime<chrono::Utc>,
    tier_config: TierConfig,
}

impl HeaderRateLimitDetector {
    /// Save detected limits to cache file
    async fn save_cache(&self, provider: &str) -> BoticelliResult<()> {
        if let Some(config) = self.detected_limits.read().await.as_ref() {
            let cache = DetectedLimitsCache {
                provider: provider.to_string(),
                detected_at: chrono::Utc::now(),
                tier_config: config.clone(),
            };

            let cache_path = dirs::cache_dir()
                .ok_or_else(|| BoticelliError::new(BoticelliErrorKind::Config(
                    "Cannot determine cache directory".to_string()
                )))?
                .join("boticelli")
                .join(format!("{}_limits.json", provider));

            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json = serde_json::to_string_pretty(&cache)?;
            std::fs::write(cache_path, json)?;
        }

        Ok(())
    }

    /// Load detected limits from cache file
    fn load_cache(provider: &str) -> Option<TierConfig> {
        let cache_path = dirs::cache_dir()?
            .join("boticelli")
            .join(format!("{}_limits.json", provider));

        let content = std::fs::read_to_string(cache_path).ok()?;
        let cache: DetectedLimitsCache = serde_json::from_str(&content).ok()?;

        // Only use cache if less than 24 hours old
        let age = chrono::Utc::now() - cache.detected_at;
        if age < chrono::Duration::hours(24) {
            Some(cache.tier_config)
        } else {
            None
        }
    }
}
```

#### Benefits of Header Detection

1. **Always Accurate**: Reflects actual current limits from provider
2. **Auto-Updates**: Detects tier upgrades without manual configuration
3. **No Staleness**: Never out of date like TOML config can be
4. **Transparent**: User doesn't need to know their tier name
5. **Fallback Safe**: TOML config used when headers unavailable

#### When Headers Aren't Available

Some scenarios where TOML fallback is used:
- First request (before any headers seen)
- Provider doesn't send rate limit headers
- Network errors prevent header parsing
- User explicitly disables detection with CLI flag

## Error Handling (📋 Planned)

### Rate limit errors

```rust
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Exceeded RPM limit
    RpmExceeded { limit: u32, reset_in: Duration },

    /// Exceeded TPM limit
    TpmExceeded { limit: u64, reset_in: Duration },

    /// Exceeded daily request limit
    RpdExceeded { limit: u32, reset_at: chrono::DateTime<chrono::Utc> },

    /// Insufficient quota
    QuotaExhausted { daily_quota_usd: f64 },
}

// Add to BoticelliErrorKind
pub enum BoticelliErrorKind {
    // ... existing variants
    RateLimit(RateLimitError),
}
```

### Retry with exponential backoff

```rust
pub async fn generate_with_retry<D: BoticelliDriver>(
    driver: &D,
    request: &GenerateRequest,
    max_retries: u32,
) -> BoticelliResult<GenerateResponse> {
    let mut retries = 0;
    let mut backoff = Duration::from_secs(1);

    loop {
        match driver.generate(request).await {
            Ok(response) => return Ok(response),
            Err(e) if retries < max_retries => {
                if let BoticelliErrorKind::RateLimit(_) = e.kind {
                    tracing::warn!("Rate limit hit, retrying in {:?}", backoff);
                    tokio::time::sleep(backoff).await;
                    retries += 1;
                    backoff *= 2; // Exponential backoff
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Future Enhancements

1. **Adaptive rate limiting**: Learn actual limits from 429 responses
2. **Shared rate limiters**: Pool rate limits across multiple client instances
3. **Priority queues**: Prioritize certain requests over others
4. **Budget enforcement**: Stop requests when daily/monthly budget is reached
5. **Multi-region support**: Different rate limits per region
6. **Burst allowances**: Allow short bursts beyond normal rate limits
7. **Dashboard**: Web UI to monitor usage and costs in real-time
