//! Act processing registry.
//!
//! Processors are invoked after an act completes to extract structured
//! data and perform side effects (database insertion, file writing, etc.).

use crate::NarrativeMetadata;
use botticelli_core::ActExecution;
use botticelli_error::BotticelliResult;
use botticelli_interface::{ActProcessor, ProcessorTrait};

/// Context provided to processors for act processing.
///
/// Contains both act-level and narrative-level information needed
/// for processors to make routing decisions and access metadata.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ProcessorContext<'a> {
    /// The act execution result
    execution: &'a ActExecution,

    /// Narrative metadata (name, description, template)
    narrative_metadata: &'a NarrativeMetadata,

    /// Full narrative name for tracking
    narrative_name: &'a str,

    /// Whether this is the last act in the narrative
    is_last_act: bool,

    /// Whether to extract and store output from this act
    /// (determined by act config or defaults to last act only)
    should_extract_output: bool,
}

impl<'a> ProcessorContext<'a> {
    /// Create a new processor context.
    pub fn new(
        execution: &'a ActExecution,
        narrative_metadata: &'a NarrativeMetadata,
        narrative_name: &'a str,
        is_last_act: bool,
        should_extract_output: bool,
    ) -> Self {
        Self {
            execution,
            narrative_metadata,
            narrative_name,
            is_last_act,
            should_extract_output,
        }
    }
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
/// registry.register(DiscordGuildProcessor::new(pool.clone()));
/// registry.register(DiscordChannelProcessor::new(pool.clone()));
///
/// // Later, in the narrative executor
/// registry.process(&context).await?;
/// ```
#[derive(Default)]
pub struct ProcessorRegistry {
    processors: Vec<Box<dyn for<'a> ProcessorTrait<ProcessorContext<'a>>>>,
}

/// Adapter to convert ActProcessor implementations to ProcessorTrait.
struct ProcessorAdapter<P> {
    processor: P,
}

// Implement for all lifetimes
#[async_trait::async_trait]
impl<'ctx, P> ProcessorTrait<ProcessorContext<'ctx>> for ProcessorAdapter<P>
where
    P: for<'a> ActProcessor<ProcessorContext<'a>, Error = botticelli_error::BotticelliError>
        + Send
        + Sync,
{
    async fn process(
        &self,
        context: &ProcessorContext<'ctx>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.processor
            .process(context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn should_process(&self, context: &ProcessorContext<'ctx>) -> bool {
        self.processor.should_process(context)
    }

    fn name(&self) -> &str {
        self.processor.name()
    }
}

impl ProcessorRegistry {
    /// Create a new empty processor registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new processor.
    ///
    /// Processors are invoked in registration order. If multiple processors
    /// match an act, all matching processors will be called.
    pub fn register<P>(&mut self, processor: P)
    where
        P: for<'a> ActProcessor<ProcessorContext<'a>, Error = botticelli_error::BotticelliError>
            + Send
            + Sync
            + 'static,
    {
        self.processors
            .push(Box::new(ProcessorAdapter { processor }));
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
    #[tracing::instrument(skip(self, context), fields(processor_count = self.processors.len()))]
    pub async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        tracing::debug!(
            processor_count = self.processors.len(),
            "ProcessorRegistry: Starting processor execution"
        );

        let mut errors = Vec::new();

        for processor in &self.processors {
            tracing::debug!(
                processor = processor.name(),
                "Checking if processor should process"
            );
            if processor.should_process(context) {
                tracing::info!(
                    processor = processor.name(),
                    "Processor will process this act"
                );

                if let Err(e) = processor.process(context).await {
                    tracing::warn!(
                        processor = processor.name(),
                        error = %e,
                        "Processor failed"
                    );
                    errors.push(format!("{}: {}", processor.name(), e));
                } else {
                    tracing::debug!(processor = processor.name(), "Processor succeeded");
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
}
