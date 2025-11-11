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

### Core Trait

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

## Provider-Specific Tier Implementations

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

## Rate Limiter Implementation

### Token Bucket Algorithm

```rust
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    tier: Box<dyn Tier>,

    // Token bucket for RPM
    rpm_tokens: Mutex<f64>,
    rpm_last_refill: Mutex<Instant>,

    // Token bucket for TPM
    tpm_tokens: Mutex<f64>,
    tpm_last_refill: Mutex<Instant>,

    // Daily request counter
    rpd_count: Mutex<u32>,
    rpd_reset_time: Mutex<Instant>,

    // Concurrent request semaphore
    concurrent_semaphore: tokio::sync::Semaphore,
}

impl RateLimiter {
    pub fn new(tier: Box<dyn Tier>) -> Self {
        let max_concurrent = tier.max_concurrent().unwrap_or(u32::MAX) as usize;

        Self {
            rpm_tokens: Mutex::new(tier.rpm().unwrap_or(u32::MAX) as f64),
            tpm_tokens: Mutex::new(tier.tpm().unwrap_or(u64::MAX) as f64),
            rpm_last_refill: Mutex::new(Instant::now()),
            tpm_last_refill: Mutex::new(Instant::now()),
            rpd_count: Mutex::new(0),
            rpd_reset_time: Mutex::new(Instant::now() + Duration::from_secs(86400)),
            concurrent_semaphore: tokio::sync::Semaphore::new(max_concurrent),
            tier,
        }
    }

    /// Wait until we can make a request with the given token count.
    pub async fn acquire(&self, estimated_tokens: u64) -> RateLimiterGuard {
        // Acquire concurrent request slot
        let permit = self.concurrent_semaphore.acquire().await.unwrap();

        // Refill RPM tokens
        self.refill_rpm().await;

        // Refill TPM tokens
        self.refill_tpm().await;

        // Check RPD
        self.check_rpd().await;

        // Wait for RPM token
        while !self.try_acquire_rpm().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.refill_rpm().await;
        }

        // Wait for TPM tokens
        while !self.try_acquire_tpm(estimated_tokens).await {
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.refill_tpm().await;
        }

        RateLimiterGuard { _permit: permit }
    }

    async fn refill_rpm(&self) {
        if let Some(rpm) = self.tier.rpm() {
            let mut tokens = self.rpm_tokens.lock().await;
            let mut last_refill = self.rpm_last_refill.lock().await;

            let elapsed = last_refill.elapsed();
            let refill_amount = (rpm as f64 / 60.0) * elapsed.as_secs_f64();

            *tokens = (*tokens + refill_amount).min(rpm as f64);
            *last_refill = Instant::now();
        }
    }

    async fn refill_tpm(&self) {
        if let Some(tpm) = self.tier.tpm() {
            let mut tokens = self.tpm_tokens.lock().await;
            let mut last_refill = self.tpm_last_refill.lock().await;

            let elapsed = last_refill.elapsed();
            let refill_amount = (tpm as f64 / 60.0) * elapsed.as_secs_f64();

            *tokens = (*tokens + refill_amount).min(tpm as f64);
            *last_refill = Instant::now();
        }
    }

    async fn check_rpd(&self) {
        if let Some(_rpd) = self.tier.rpd() {
            let mut reset_time = self.rpd_reset_time.lock().await;

            // Reset daily counter if needed
            if Instant::now() >= *reset_time {
                let mut count = self.rpd_count.lock().await;
                *count = 0;
                *reset_time = Instant::now() + Duration::from_secs(86400);
            }
        }
    }

    async fn try_acquire_rpm(&self) -> bool {
        if let Some(_rpm) = self.tier.rpm() {
            let mut tokens = self.rpm_tokens.lock().await;
            if *tokens >= 1.0 {
                *tokens -= 1.0;
                return true;
            }
            false
        } else {
            true // No RPM limit
        }
    }

    async fn try_acquire_tpm(&self, estimated_tokens: u64) -> bool {
        if let Some(_tpm) = self.tier.tpm() {
            let mut tokens = self.tpm_tokens.lock().await;
            if *tokens >= estimated_tokens as f64 {
                *tokens -= estimated_tokens as f64;
                return true;
            }
            false
        } else {
            true // No TPM limit
        }
    }
}

pub struct RateLimiterGuard {
    _permit: tokio::sync::SemaphorePermit<'static>,
}
```

## Integration with BoticelliDriver

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

## Usage Tracking and Cost Estimation

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

## Configuration

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

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    fn rpm(&self) -> Option<u32> {
        self.rpm
    }

    fn tpm(&self) -> Option<u64> {
        self.tpm
    }

    fn rpd(&self) -> Option<u32> {
        self.rpd
    }

    fn max_concurrent(&self) -> Option<u32> {
        self.max_concurrent
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        self.daily_quota_usd
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        self.cost_per_million_input_tokens
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        self.cost_per_million_output_tokens
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl BoticelliConfig {
    /// Load configuration from file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> BoticelliResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to read boticelli.toml: {}", e)
            )))?;

        toml::from_str(&content)
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to parse boticelli.toml: {}", e)
            )))
    }

    /// Load bundled default configuration
    fn load_defaults() -> BoticelliResult<Self> {
        const DEFAULT_CONFIG: &str = include_str!("../boticelli.toml");

        toml::from_str(DEFAULT_CONFIG)
            .map_err(|e| BoticelliError::new(BoticelliErrorKind::Config(
                format!("Failed to parse bundled boticelli.toml: {}", e)
            )))
    }

    /// Load configuration with precedence: user override > bundled default
    pub fn load() -> BoticelliResult<Self> {
        // Start with bundled defaults
        let mut config = Self::load_defaults()?;

        // Try to load user override from current directory
        if let Ok(user_config) = Self::from_file("boticelli.toml") {
            config.merge(user_config);
            return Ok(config);
        }

        // Try to load user override from home directory
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".config/boticelli/boticelli.toml");
            if let Ok(user_config) = Self::from_file(&path) {
                config.merge(user_config);
                return Ok(config);
            }
        }

        // No user override found, return defaults
        Ok(config)
    }

    /// Merge another config into this one, with the other config taking precedence
    pub fn merge(&mut self, other: BoticelliConfig) {
        for (provider_name, provider_config) in other.providers {
            self.providers
                .entry(provider_name)
                .and_modify(|existing| {
                    // Override default tier if specified
                    existing.default_tier = provider_config.default_tier.clone();

                    // Merge tiers (other's tiers override ours)
                    for (tier_name, tier_config) in &provider_config.tiers {
                        existing.tiers.insert(tier_name.clone(), tier_config.clone());
                    }
                })
                .or_insert(provider_config);
        }
    }

    /// Get tier configuration for a provider
    pub fn get_tier(&self, provider: &str, tier_name: Option<&str>) -> Option<TierConfig> {
        let provider_config = self.providers.get(provider)?;

        let tier = tier_name.unwrap_or(&provider_config.default_tier);

        provider_config.tiers.get(tier).cloned()
    }
}

impl Default for BoticelliConfig {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
}
```

### Configuration Precedence

Configuration is loaded in the following order (highest priority first):

1. **CLI flags** - Runtime overrides via command-line arguments
2. **Environment variables** - `GEMINI_TIER`, `ANTHROPIC_TIER`, etc.
3. **User config file** - `./boticelli.toml` or `~/.config/boticelli/boticelli.toml`
4. **Bundled defaults** - The `boticelli.toml` shipped with Boticelli

### CLI Override Flags

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

CLI flag implementation:

```rust
#[derive(Parser)]
struct RunCommand {
    /// Path to narrative TOML file
    #[arg(short, long)]
    narrative: PathBuf,

    /// LLM backend to use
    #[arg(short, long, default_value = "gemini")]
    backend: String,

    /// API tier to use (overrides config and env)
    #[arg(long)]
    tier: Option<String>,

    /// Override requests per minute limit
    #[arg(long)]
    rpm: Option<u32>,

    /// Override tokens per minute limit
    #[arg(long)]
    tpm: Option<u64>,

    /// Override requests per day limit
    #[arg(long)]
    rpd: Option<u32>,

    /// Override max concurrent requests
    #[arg(long)]
    max_concurrent: Option<u32>,

    /// Override input token cost (per million)
    #[arg(long)]
    cost_input: Option<f64>,

    /// Override output token cost (per million)
    #[arg(long)]
    cost_output: Option<f64>,

    /// Disable rate limiting
    #[arg(long)]
    no_rate_limit: bool,
}

impl RunCommand {
    /// Apply CLI overrides to tier config
    fn apply_overrides(&self, mut tier_config: TierConfig) -> TierConfig {
        if self.no_rate_limit {
            // Remove all limits
            tier_config.rpm = None;
            tier_config.tpm = None;
            tier_config.rpd = None;
            tier_config.max_concurrent = None;
        } else {
            // Apply individual overrides
            if let Some(rpm) = self.rpm {
                tier_config.rpm = Some(rpm);
            }
            if let Some(tpm) = self.tpm {
                tier_config.tpm = Some(tpm);
            }
            if let Some(rpd) = self.rpd {
                tier_config.rpd = Some(rpd);
            }
            if let Some(max_concurrent) = self.max_concurrent {
                tier_config.max_concurrent = Some(max_concurrent);
            }
        }

        if let Some(cost_input) = self.cost_input {
            tier_config.cost_per_million_input_tokens = Some(cost_input);
        }
        if let Some(cost_output) = self.cost_output {
            tier_config.cost_per_million_output_tokens = Some(cost_output);
        }

        tier_config
    }
}
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

### Programmatic tier detection

Some providers return rate limit headers that can be used to auto-detect tier:

```rust
impl GeminiClient {
    /// Detect tier from API response headers
    fn detect_tier_from_headers(&self, headers: &reqwest::header::HeaderMap) -> GeminiTier {
        if let Some(rpm) = headers.get("x-ratelimit-requests-per-minute") {
            if rpm.to_str().unwrap_or("0").parse::<u32>().unwrap_or(0) > 100 {
                return GeminiTier::PayAsYouGo;
            }
        }
        GeminiTier::Free
    }
}
```

## Error Handling

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
