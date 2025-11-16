//! Act processing traits and registry.
//!
//! Processors are invoked after an act completes to extract structured
//! data and perform side effects (database insertion, file writing, etc.).

use crate::{ActExecution, BoticelliResult};
use async_trait::async_trait;

/// Trait for processing act execution results.
///
/// Processors are invoked after an act completes to extract structured
/// data and perform side effects (database insertion, file writing, etc.).
///
/// # Example
///
/// ```rust,ignore
/// use boticelli::{ActProcessor, ActExecution, BoticelliResult};
/// use async_trait::async_trait;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl ActProcessor for MyProcessor {
///     async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
///         // Extract and process data from execution.response
///         Ok(())
///     }
///
///     fn should_process(&self, act_name: &str, response: &str) -> bool {
///         act_name.contains("my_data")
///     }
///
///     fn name(&self) -> &str {
///         "MyProcessor"
///     }
/// }
/// ```
#[async_trait]
pub trait ActProcessor: Send + Sync {
    /// Process an act execution result.
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
    async fn process(&self, execution: &ActExecution) -> BoticelliResult<()>;

    /// Check if this processor should handle the given act.
    ///
    /// Implementations can check act name, response content, metadata, etc.
    /// to determine if this processor is appropriate for the act.
    ///
    /// # Arguments
    ///
    /// * `act_name` - The name of the act from the narrative
    /// * `response` - The LLM response text
    ///
    /// # Returns
    ///
    /// `true` if this processor should process the act, `false` otherwise.
    fn should_process(&self, act_name: &str, response: &str) -> bool;

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
/// use boticelli::ProcessorRegistry;
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
    /// # Errors
    ///
    /// Returns an error if any processor fails. The error message includes
    /// all processor errors concatenated together.
    pub async fn process(&self, execution: &ActExecution) -> BoticelliResult<()> {
        let mut errors = Vec::new();

        for processor in &self.processors {
            if processor.should_process(&execution.act_name, &execution.response) {
                if let Err(e) = processor.process(execution).await {
                    tracing::warn!(
                        processor = processor.name(),
                        act = %execution.act_name,
                        error = %e,
                        "Processor failed"
                    );
                    errors.push(format!("{}: {}", processor.name(), e));
                } else {
                    tracing::debug!(
                        processor = processor.name(),
                        act = %execution.act_name,
                        "Processor succeeded"
                    );
                }
            }
        }

        if !errors.is_empty() {
            return Err(crate::BackendError::new(format!(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;

    struct TestProcessor {
        name: String,
        should_process: bool,
        fail: bool,
    }

    #[async_trait]
    impl ActProcessor for TestProcessor {
        async fn process(&self, _execution: &ActExecution) -> BoticelliResult<()> {
            if self.fail {
                Err(crate::BackendError::new("Test error").into())
            } else {
                Ok(())
            }
        }

        fn should_process(&self, _act_name: &str, _response: &str) -> bool {
            self.should_process
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    fn create_test_execution(act_name: &str, response: &str) -> ActExecution {
        ActExecution {
            act_name: act_name.to_string(),
            inputs: vec![Input::Text("test input".to_string())],
            model: None,
            temperature: None,
            max_tokens: None,
            response: response.to_string(),
            sequence_number: 0,
        }
    }

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = ProcessorRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        let execution = create_test_execution("test", "test response");
        registry.process(&execution).await.unwrap();
    }

    #[tokio::test]
    async fn test_register_and_process() {
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(TestProcessor {
            name: "Test1".to_string(),
            should_process: true,
            fail: false,
        }));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let execution = create_test_execution("test", "test response");
        registry.process(&execution).await.unwrap();
    }

    #[tokio::test]
    async fn test_should_process_filtering() {
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(TestProcessor {
            name: "ShouldRun".to_string(),
            should_process: true,
            fail: false,
        }));
        registry.register(Box::new(TestProcessor {
            name: "ShouldNotRun".to_string(),
            should_process: false,
            fail: false,
        }));

        let execution = create_test_execution("test", "test response");
        registry.process(&execution).await.unwrap();
    }

    #[tokio::test]
    async fn test_processor_error_handling() {
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(TestProcessor {
            name: "Failing".to_string(),
            should_process: true,
            fail: true,
        }));

        let execution = create_test_execution("test", "test response");
        let result = registry.process(&execution).await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failing"));
        assert!(err_msg.contains("Test error"));
    }

    #[tokio::test]
    async fn test_multiple_processors() {
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(TestProcessor {
            name: "Processor1".to_string(),
            should_process: true,
            fail: false,
        }));
        registry.register(Box::new(TestProcessor {
            name: "Processor2".to_string(),
            should_process: true,
            fail: false,
        }));

        assert_eq!(registry.len(), 2);

        let execution = create_test_execution("test", "test response");
        registry.process(&execution).await.unwrap();
    }

    #[tokio::test]
    async fn test_partial_failure() {
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(TestProcessor {
            name: "Success".to_string(),
            should_process: true,
            fail: false,
        }));
        registry.register(Box::new(TestProcessor {
            name: "Failure1".to_string(),
            should_process: true,
            fail: true,
        }));
        registry.register(Box::new(TestProcessor {
            name: "Failure2".to_string(),
            should_process: true,
            fail: true,
        }));

        let execution = create_test_execution("test", "test response");
        let result = registry.process(&execution).await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failure1"));
        assert!(err_msg.contains("Failure2"));
    }
}
