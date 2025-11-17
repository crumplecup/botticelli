//! Narrative execution logic.
//!
//! This module provides the executor that processes multi-act narratives
//! by calling LLM APIs in sequence, passing context between acts.

use crate::{BotticelliDriver, GenerateRequest, Input, Message, Output, Role};
use crate::{BotticelliResult, NarrativeProvider, ProcessorContext, ProcessorRegistry};
use serde::{Deserialize, Serialize};

/// Execution result for a single act in a narrative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActExecution {
    /// Name of the act (from the narrative).
    pub act_name: String,

    /// The multimodal inputs that were sent to the LLM.
    pub inputs: Vec<Input>,

    /// The model used for this act (if overridden).
    pub model: Option<String>,

    /// The temperature used for this act (if overridden).
    pub temperature: Option<f32>,

    /// The max_tokens used for this act (if overridden).
    pub max_tokens: Option<u32>,

    /// The text response from the LLM.
    pub response: String,

    /// Position in the execution sequence (0-indexed).
    pub sequence_number: usize,
}

/// Complete execution result for a narrative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeExecution {
    /// Name of the narrative that was executed.
    pub narrative_name: String,

    /// Ordered list of act executions.
    pub act_executions: Vec<ActExecution>,
}

/// Executes narratives by calling LLM APIs in sequence.
///
/// The executor processes each act in the narrative's table of contents order,
/// passing previous act outputs as context to subsequent acts.
///
/// Optionally, processors can be registered to extract and process structured
/// data from act responses (e.g., JSON extraction, database insertion).
pub struct NarrativeExecutor<D: BotticelliDriver> {
    driver: D,
    processor_registry: Option<ProcessorRegistry>,
}

impl<D: BotticelliDriver> NarrativeExecutor<D> {
    /// Create a new narrative executor with the given LLM driver.
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            processor_registry: None,
        }
    }

    /// Create a new narrative executor with processors.
    ///
    /// Processors will be invoked after each act completes to extract
    /// and process structured data from the response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli::{NarrativeExecutor, ProcessorRegistry};
    ///
    /// let mut registry = ProcessorRegistry::new();
    /// registry.register(Box::new(MyProcessor::new()));
    ///
    /// let executor = NarrativeExecutor::with_processors(driver, registry);
    /// ```
    pub fn with_processors(driver: D, registry: ProcessorRegistry) -> Self {
        Self {
            driver,
            processor_registry: Some(registry),
        }
    }

    /// Execute a narrative, processing all acts in sequence.
    ///
    /// Each act sees the outputs from all previous acts as conversation history.
    /// The first act receives just its prompt, the second act sees the first act's
    /// response plus its own prompt, and so on.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any LLM API call fails
    /// - The response format is unexpected
    pub async fn execute<N: NarrativeProvider>(
        &self,
        narrative: &N,
    ) -> BotticelliResult<NarrativeExecution> {
        let mut act_executions = Vec::new();
        let mut conversation_history: Vec<Message> = Vec::new();

        for (sequence_number, act_name) in narrative.act_names().iter().enumerate() {
            // Get the configuration for this act
            let config = narrative
                .get_act_config(act_name)
                .expect("NarrativeProvider should ensure all acts exist");

            // Build the request with conversation history + current act inputs
            conversation_history.push(Message {
                role: Role::User,
                content: config.inputs.clone(),
            });

            let request = GenerateRequest {
                messages: conversation_history.clone(),
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                model: config.model.clone(),
            };

            // Call the LLM
            let response = self.driver.generate(&request).await?;

            // Extract text from response
            let response_text = extract_text_from_outputs(&response.outputs)?;

            // Create the act execution
            let act_execution = ActExecution {
                act_name: act_name.clone(),
                inputs: config.inputs.clone(),
                model: config.model,
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                response: response_text.clone(),
                sequence_number,
            };

            // Process with registered processors
            if let Some(registry) = &self.processor_registry {
                tracing::info!(
                    act = %act_name,
                    processors = registry.len(),
                    "Processing act with registered processors"
                );

                // Build processor context
                let context = ProcessorContext {
                    execution: &act_execution,
                    narrative_metadata: narrative.metadata(),
                    narrative_name: narrative.name(),
                };

                if let Err(e) = registry.process(&context).await {
                    tracing::error!(
                        act = %act_name,
                        error = %e,
                        "Act processing failed, continuing execution"
                    );
                    // Note: We don't fail the entire narrative on processor errors
                    // The user still gets the execution results
                }
            }

            // Store the act execution
            act_executions.push(act_execution);

            // Add the assistant's response to conversation history for the next act
            conversation_history.push(Message {
                role: Role::Assistant,
                content: vec![Input::Text(response_text)],
            });
        }

        Ok(NarrativeExecution {
            narrative_name: narrative.name().to_string(),
            act_executions,
        })
    }

    /// Get a reference to the underlying LLM driver.
    pub fn driver(&self) -> &D {
        &self.driver
    }
}

/// Extract text content from LLM outputs.
///
/// Concatenates all text outputs with newlines between them.
fn extract_text_from_outputs(outputs: &[Output]) -> BotticelliResult<String> {
    let mut texts = Vec::new();

    for output in outputs {
        if let Output::Text(text) = output {
            texts.push(text.clone());
        }
    }

    if texts.is_empty() {
        Ok(String::new())
    } else {
        Ok(texts.join("\n"))
    }
}
