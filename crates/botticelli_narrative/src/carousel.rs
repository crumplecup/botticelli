//! Carousel - Budget-aware iterative execution.

use botticelli_core::BudgetConfig;
use botticelli_error::{NarrativeError, NarrativeErrorKind};
use botticelli_rate_limit::Budget;
use derive_getters::Getters;
use derive_setters::Setters;
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Carousel configuration for iterative execution with budget constraints.
///
/// A carousel allows an act or entire narrative to execute multiple times
/// while respecting rate limit budgets. The carousel will execute as many
/// iterations as the budget allows, stopping when limits are approached.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Getters, Setters, elicitation::Elicit)]
#[setters(prefix = "with_")]
pub struct CarouselConfig {
    /// Maximum number of iterations to attempt
    iterations: u32,

    /// Estimated tokens per iteration
    /// Used to pre-check budget before starting an iteration
    #[serde(default = "default_estimated_tokens")]
    estimated_tokens_per_iteration: u64,

    /// Whether to stop on first error or continue
    #[serde(default)]
    continue_on_error: bool,

    /// Optional budget multipliers to throttle API usage.
    ///
    /// Available with the`budget`feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    budget: Option<BudgetConfig>,
}

fn default_estimated_tokens() -> u64 {
    1000 // Conservative default estimate
}

impl CarouselConfig {
    /// Creates a new carousel configuration.
    #[tool]
    #[tracing::instrument(fields(iterations, estimated_tokens_per_iteration))]
    pub fn new(iterations: u32, estimated_tokens_per_iteration: u64) -> Self {
        tracing::debug!(
            iterations,
            estimated_tokens_per_iteration,
            "Creating carousel configuration"
        );
        Self {
            iterations,
            estimated_tokens_per_iteration,
            continue_on_error: false,
            budget: None,
        }
    }
}

/// Carousel execution state.
///
/// Tracks progress through carousel iterations and manages the budget.
#[derive(Debug, Getters)]
pub struct CarouselState<T: botticelli_interface::Tier + std::fmt::Debug> {
    /// Carousel configuration
    config: CarouselConfig,

    /// Budget tracker
    budget: Budget<T>,

    /// Current iteration number (1-indexed)
    current_iteration: u32,

    /// Number of successful iterations
    successful_iterations: u32,

    /// Number of failed iterations
    failed_iterations: u32,

    /// Whether carousel completed all iterations
    completed: bool,

    /// Whether carousel was stopped due to budget constraints
    budget_exhausted: bool,
}

impl<T: botticelli_interface::Tier + std::fmt::Debug> CarouselState<T> {
    /// Creates a new carousel state with the given configuration and budget.
    #[tracing::instrument(skip(rate_limits), fields(iterations = config.iterations, estimated_tokens = config.estimated_tokens_per_iteration))]
    #[tool]
    pub fn new(config: CarouselConfig, rate_limits: T) -> Self {
        tracing::debug!(
            iterations = config.iterations,
            estimated_tokens = config.estimated_tokens_per_iteration,
            continue_on_error = config.continue_on_error,
            "Creating carousel state"
        );
        Self {
            config,
            budget: Budget::new(rate_limits),
            current_iteration: 0,
            successful_iterations: 0,
            failed_iterations: 0,
            completed: false,
            budget_exhausted: false,
        }
    }

    /// Gets mutable access to the budget.
    #[tool]
    pub fn budget_mut(&mut self) -> &mut Budget<T> {
        &mut self.budget
    }

    /// Checks if another iteration can be started.
    ///
    /// Returns true if:
    /// - We haven't reached max iterations
    /// - Budget can afford the estimated tokens for next iteration
    #[tracing::instrument(skip(self))]
    #[tool]
    pub fn can_continue(&mut self) -> bool {
        if self.current_iteration >= self.config.iterations {
            tracing::debug!("Max iterations reached");
            return false;
        }

        if !self
            .budget
            .can_afford(self.config.estimated_tokens_per_iteration)
        {
            tracing::warn!("Budget exhausted, cannot continue carousel");
            self.budget_exhausted = true;
            return false;
        }

        true
    }

    /// Starts the next iteration.
    ///
    /// # Errors
    ///
    /// Returns an error if max iterations reached or budget exhausted.
    #[tracing::instrument(skip(self))]
    #[tool]
    pub fn start_iteration(&mut self) -> Result<u32, NarrativeError> {
        if !self.can_continue() {
            return Err(NarrativeError::new(
                NarrativeErrorKind::CarouselBudgetExhausted {
                    completed_iterations: self.successful_iterations,
                    max_iterations: self.config.iterations,
                },
            ));
        }

        self.current_iteration += 1;
        tracing::info!(
            iteration = self.current_iteration,
            max_iterations = self.config.iterations,
            "Starting carousel iteration"
        );

        Ok(self.current_iteration)
    }

    /// Records a successful iteration.
    #[tracing::instrument(skip(self), fields(iteration = self.current_iteration))]
    #[tool]
    pub fn record_success(&mut self) {
        self.successful_iterations += 1;
        tracing::debug!(
            iteration = self.current_iteration,
            total_successful = self.successful_iterations,
            "Carousel iteration succeeded"
        );
    }

    /// Records a failed iteration.
    #[tracing::instrument(skip(self), fields(iteration = self.current_iteration))]
    #[tool]
    pub fn record_failure(&mut self) {
        self.failed_iterations += 1;
        tracing::warn!(
            iteration = self.current_iteration,
            total_failed = self.failed_iterations,
            "Carousel iteration failed"
        );
    }

    /// Marks the carousel as completed.
    #[tracing::instrument(skip(self), fields(successful = self.successful_iterations, failed = self.failed_iterations))]
    #[tool]
    pub fn finish(&mut self) {
        self.completed = true;
        tracing::info!(
            successful = self.successful_iterations,
            failed = self.failed_iterations,
            budget_exhausted = self.budget_exhausted,
            "Carousel finished"
        );
    }
}

/// Result of carousel execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters, elicitation::Elicit)]
pub struct CarouselResult {
    /// Total iterations attempted
    iterations_attempted: u32,

    /// Successful iterations
    successful_iterations: u32,

    /// Failed iterations
    failed_iterations: u32,

    /// Whether all requested iterations completed
    completed: bool,

    /// Whether stopped due to budget constraints
    budget_exhausted: bool,
}

impl<T: botticelli_interface::Tier + std::fmt::Debug> From<&CarouselState<T>> for CarouselResult {
    #[tracing::instrument(skip(state), fields(
        iterations_attempted = state.current_iteration(),
        successful = state.successful_iterations(),
        failed = state.failed_iterations(),
        completed = state.completed(),
        budget_exhausted = state.budget_exhausted()
    ))]
    fn from(state: &CarouselState<T>) -> Self {
        tracing::debug!(
            iterations_attempted = state.current_iteration(),
            successful = state.successful_iterations(),
            failed = state.failed_iterations(),
            completed = state.completed(),
            budget_exhausted = state.budget_exhausted(),
            "Creating carousel result from state"
        );
        Self {
            iterations_attempted: *state.current_iteration(),
            successful_iterations: *state.successful_iterations(),
            failed_iterations: *state.failed_iterations(),
            completed: *state.completed(),
            budget_exhausted: *state.budget_exhausted(),
        }
    }
}
