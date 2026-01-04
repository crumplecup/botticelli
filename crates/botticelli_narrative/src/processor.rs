//! Act processing traits and registry.
//!
//! Processors are invoked after an act completes to extract structured
//! data and perform side effects (database insertion, file writing, etc.).

use crate::NarrativeMetadata;
use async_trait::async_trait;
use botticelli_core::ActExecution;
use botticelli_error::BotticelliResult;

/// Context provided to processors for act processing.
///
/// Contains both act-level and narrative-level information needed
/// for processors to make routing decisions and access metadata.
#[derive(Debug, Clone)]
pub struct ProcessorContext<'a> {
    /// The act execution result
    pub execution: &'a ActExecution,

    /// Narrative metadata (name, description, template)
    pub narrative_metadata: &'a NarrativeMetadata,

    /// Full narrative name for tracking
    pub narrative_name: &'a str,

    /// Whether this is the last act in the narrative
    pub is_last_act: bool,

    /// Whether to extract and store output from this act
    /// (determined by act config or defaults to last act only)
    pub should_extract_output: bool,
}

/// Trait for processing act execution results with narrative context.
///
/// Processors are invoked after an act completes to extract structured
/// data and perform side effects (database insertion, file writing, etc.).
///
/// Processors receive a `ProcessorContext` containing both act-level data
/// (execution results) and narrative-level metadata (name, description, template).
///
/// # Example
///
/// ```rust,ignore
/// use botticelli_narrative::{ActProcessor, ProcessorContext};
/// use botticelli_error::BotticelliResult;
/// use async_trait::async_trait;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl ActProcessor for MyProcessor {
///     async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
///         // Extract and process data from context.execution.response
///         // Access narrative metadata via context.narrative_metadata
///         Ok(())
///     }
///
///     fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
///         context.execution.act_name().contains("my_data")
///     }
///
///     fn name(&self) -> &str {
///         "MyProcessor"
///     }
/// }
/// ```
#[async_trait]
pub trait ActProcessor: Send + Sync {
    /// Process an act execution result with narrative context.
    ///
    /// This method is called after an act completes successfully.
    /// Implementations should extract structured data from the response
    /// and perform any necessary side effects.
    ///
    /// The context provides access to:
    /// - Act execution results (response, model, etc.)
    /// - Narrative metadata (name, description, template)
    /// - Narrative name for tracking
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails. The error should be descriptive
    /// and include context about what went wrong. Note that processor errors
    /// do not fail the entire narrative execution.
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()>;

    /// Check if this processor should handle the given act.
    ///
    /// Implementations can check act name, response content, narrative metadata, etc.
    /// to determine if this processor is appropriate for the act.
    ///
    /// # Arguments
    ///
    /// * `context` - Full context including execution and narrative metadata
    ///
    /// # Returns
    ///
    /// `true` if this processor should process the act, `false` otherwise.
    fn should_process(&self, context: &ProcessorContext<'_>) -> bool;

    /// Return a human-readable name for this processor.
    ///
    /// Used for logging and error messages.
    fn name(&self) -> &str;
}

/// Registry of act processors with smart routing.
///
/// The registry manages multiple processors and routes act executions
/// to the appropriate handlers based on their `should_process` logic.
///
/// # Example
///
/// ```rust,ignore
/// use botticelli_narrative::ProcessorRegistry;
///
/// let mut registry = ProcessorRegistry::new();
/// registry.register(Box::new(DiscordGuildProcessor::new(pool.clone())));
/// registry.register(Box::new(DiscordChannelProcessor::new(pool.clone())));
///
/// // Later, in the narrative executor
/// registry.process(&act_execution).await?;
/// ```
pub struct ProcessorRegistry {
    processors: Vec<Box<dyn ActProcessor>>,
}

impl ProcessorRegistry {
    /// Create a new empty processor registry.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Register a new processor.
    ///
    /// Processors are invoked in registration order. If multiple processors
    /// match an act, all matching processors will be called.
    pub fn register(&mut self, processor: Box<dyn ActProcessor>) {
        self.processors.push(processor);
    }

    /// Process an act execution with all matching processors.
    ///
    /// Calls each processor that returns `true` from `should_process`.
    /// Continues processing even if some processors fail, collecting all errors.
    ///
    /// # Arguments
    ///
    /// * `context` - Context containing execution and narrative metadata
    ///
    /// # Errors
    ///
    /// Returns an error if any processor fails. The error message includes
    /// all processor errors concatenated together.
    #[tracing::instrument(skip(self, context), fields(act = %context.execution.act_name(), processor_count = self.processors.len()))]
    pub async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        tracing::debug!(
            act = %context.execution.act_name(),
            processor_count = self.processors.len(),
            "ProcessorRegistry: Starting processor execution"
        );

        let mut errors = Vec::new();

        for processor in &self.processors {
            tracing::debug!(
                processor = processor.name(),
                act = %context.execution.act_name(),
                "Checking if processor should process"
            );
            if processor.should_process(context) {
                tracing::info!(
                    processor = processor.name(),
                    act = %context.execution.act_name(),
                    "Processor will process this act"
                );

                if let Err(e) = processor.process(context).await {
                    tracing::warn!(
                        processor = processor.name(),
                        act = %context.execution.act_name(),
                        error = %e,
                        "Processor failed"
                    );
                    errors.push(format!("{}: {}", processor.name(), e));
                } else {
                    tracing::debug!(
                        processor = processor.name(),
                        act = %context.execution.act_name(),
                        "Processor succeeded"
                    );
                }
            }
        }

        if !errors.is_empty() {
            return Err(botticelli_error::BackendError::new(format!(
                "Processor errors: {}",
                errors.join("; ")
            ))
            .into());
        }

        Ok(())
    }

    /// Get the number of registered processors.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// Get references to all registered processors.
    ///
    /// Useful for debugging or introspection.
    pub fn processors(&self) -> &[Box<dyn ActProcessor>] {
        &self.processors
    }
}

impl Default for ProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
