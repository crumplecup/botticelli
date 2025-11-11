//! Tier trait for representing API rate limit constraints.

/// Represents rate limiting constraints for an API tier.
///
/// Different LLM providers have different rate limiting schemes (RPM, TPM, RPD,
/// concurrent requests, etc.). This trait provides a common interface for querying
/// these limits.
///
/// All methods return `Option<T>` where `None` indicates unlimited/not applicable.
///
/// # Example
///
/// ```
/// use boticelli::Tier;
///
/// struct FreeTier;
///
/// impl Tier for FreeTier {
///     fn rpm(&self) -> Option<u32> { Some(10) }
///     fn tpm(&self) -> Option<u64> { Some(250_000) }
///     fn rpd(&self) -> Option<u32> { Some(250) }
///     fn max_concurrent(&self) -> Option<u32> { Some(1) }
///     fn daily_quota_usd(&self) -> Option<f64> { None }
///     fn cost_per_million_input_tokens(&self) -> Option<f64> { Some(0.0) }
///     fn cost_per_million_output_tokens(&self) -> Option<f64> { Some(0.0) }
///     fn name(&self) -> &str { "Free" }
/// }
/// ```
pub trait Tier: Send + Sync {
    /// Requests per minute limit.
    ///
    /// Returns `None` if there is no per-minute request limit.
    fn rpm(&self) -> Option<u32>;

    /// Tokens per minute limit.
    ///
    /// Returns `None` if there is no per-minute token limit.
    fn tpm(&self) -> Option<u64>;

    /// Requests per day limit.
    ///
    /// Returns `None` if there is no daily request limit.
    fn rpd(&self) -> Option<u32>;

    /// Maximum concurrent requests.
    ///
    /// Returns `None` if there is no concurrent request limit.
    fn max_concurrent(&self) -> Option<u32>;

    /// Daily quota in USD (for pay-as-you-go models).
    ///
    /// Returns `None` if there is no daily spending quota.
    fn daily_quota_usd(&self) -> Option<f64>;

    /// Cost per million input tokens in USD.
    ///
    /// Returns `None` if cost information is not available or the tier is free.
    fn cost_per_million_input_tokens(&self) -> Option<f64>;

    /// Cost per million output tokens in USD.
    ///
    /// Returns `None` if cost information is not available or the tier is free.
    fn cost_per_million_output_tokens(&self) -> Option<f64>;

    /// Name of the tier (e.g., "Free", "Pro", "Enterprise", "Tier 1").
    fn name(&self) -> &str;
}
