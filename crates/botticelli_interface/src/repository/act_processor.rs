//! Trait for processing act execution results.
//!
//! This module defines the interface for extracting structured data
//! and performing side effects after act completion.

use async_trait::async_trait;

/// Trait for processing act execution results with narrative context.
///
/// Processors are invoked after an act completes to extract structured
/// data and perform side effects (database insertion, file writing, etc.).
///
/// The trait is generic over the context type, allowing implementations
/// to define their own context structures.
///
/// # Example
///
/// ```rust,ignore
/// use botticelli_interface::ActProcessor;
/// use async_trait::async_trait;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl ActProcessor<MyProcessorContext<'_>> for MyProcessor {
///     type Error = MyError;
///
///     async fn process(&self, context: &MyProcessorContext<'_>) -> Result<(), Self::Error> {
///         // Extract and process data
///         Ok(())
///     }
///
///     fn should_process(&self, context: &MyProcessorContext<'_>) -> bool {
///         // Check if processor should handle this act
///         true
///     }
///
///     fn name(&self) -> &str {
///         "MyProcessor"
///     }
/// }
/// ```
#[async_trait]
pub trait ActProcessor<C>: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Process an act execution result with narrative context.
    ///
    /// This method is called after an act completes successfully.
    /// Implementations should extract structured data from the response
    /// and perform any necessary side effects.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails. The error should be descriptive
    /// and include context about what went wrong. Note that processor errors
    /// do not fail the entire narrative execution.
    async fn process(&self, context: &C) -> Result<(), Self::Error>;

    /// Check if this processor should handle the given act.
    ///
    /// Implementations can check act name, response content, narrative metadata, etc.
    /// to determine if this processor is appropriate for the act.
    ///
    /// # Returns
    ///
    /// `true` if this processor should process the act, `false` otherwise.
    fn should_process(&self, context: &C) -> bool;

    /// Return a human-readable name for this processor.
    ///
    /// Used for logging and error messages.
    fn name(&self) -> &str;
}

/// Type-erased processor trait for storage in registries.
///
/// This trait enables storing processors with higher-rank trait bounds
/// (HRTB) in collections like Vec. Implementations should use ActProcessor
/// and convert via an adapter.
#[async_trait]
pub trait ProcessorTrait<C>: Send + Sync {
    /// Process an act execution result.
    async fn process(&self, context: &C) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Check if this processor should handle the given act.
    fn should_process(&self, context: &C) -> bool;

    /// Return a human-readable name for this processor.
    fn name(&self) -> &str;
}
