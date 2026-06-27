use elicitation::Generator;
use elicitation_rand::{SeedableRng, StdRng, UniformGenerator};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::instrument;

/// User-configurable timing jitter for scheduled intervals.
///
/// Derives [`elicitation::Elicit`] so the user can configure jitter
/// interactively. Call [`PostingJitter::make_generator`] to obtain a
/// [`PostingJitterGen`] that produces varied [`Duration`] offsets.
#[derive(Debug, Clone, Serialize, Deserialize, elicitation::Elicit, JsonSchema)]
#[prompt("Configure timing jitter for scheduled intervals:")]
pub struct PostingJitter {
    /// Maximum jitter offset in minutes, applied to the base interval
    #[prompt("Maximum jitter in minutes (applied to the base interval; 0 for no jitter):")]
    pub max_minutes: u64,
    /// Seed for the jitter sequence — same seed reproduces the same cadence
    #[prompt("Seed for the jitter sequence (same seed = reproducible cadence):")]
    pub seed: u64,
}

impl PostingJitter {
    /// Create a jitter config.
    pub fn new(max_minutes: u64, seed: u64) -> Self {
        Self { max_minutes, seed }
    }

    /// Build a stateful generator that produces varied [`Duration`] offsets.
    ///
    /// The returned [`PostingJitterGen`] is `Send + Sync` and may move into
    /// `tokio::spawn`.
    pub fn make_generator(&self) -> PostingJitterGen {
        PostingJitterGen::new(self.max_minutes * 60, self.seed)
    }
}

/// Stateful generator that produces random [`Duration`] offsets within a bound.
///
/// Each [`Generator::generate`] call increments an atomic counter and mixes it
/// with the base seed via a multiplicative hash to derive a fresh child seed for
/// [`UniformGenerator`], giving varied durations without holding mutex-guarded
/// RNG state.
pub struct PostingJitterGen {
    base_seed: u64,
    call_count: AtomicU64,
    max_secs: u64,
}

impl PostingJitterGen {
    /// Create a new generator seeded from the given values.
    #[instrument]
    pub fn new(max_secs: u64, base_seed: u64) -> Self {
        // Validate seed is acceptable to StdRng.
        let _ = StdRng::seed_from_u64(base_seed);
        Self {
            base_seed,
            call_count: AtomicU64::new(0),
            max_secs,
        }
    }
}

impl Generator for PostingJitterGen {
    type Target = Duration;

    #[instrument(skip(self), fields(max_secs = self.max_secs))]
    fn generate(&self) -> Duration {
        if self.max_secs == 0 {
            return Duration::ZERO;
        }
        let n = self.call_count.fetch_add(1, Ordering::Relaxed);
        // LCG-style mix so consecutive calls produce uncorrelated seeds.
        let child_seed = self
            .base_seed
            .wrapping_add(n.wrapping_mul(6364136223846793005));
        let offset =
            UniformGenerator::<u64>::with_seed(child_seed, 0, self.max_secs + 1).generate();
        Duration::from_secs(offset)
    }
}
