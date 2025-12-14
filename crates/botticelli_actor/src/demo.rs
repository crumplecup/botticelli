use crate::{ActorError, ActorErrorKind, ActorResult};
use async_trait::async_trait;
use derive_getters::Getters;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument};

/// Scenario for automated demo execution.
#[derive(Debug, Clone, Getters)]
pub struct DemoScenario {
    name: String,
    description: String,
    prompts: Vec<String>,
    delay_between_prompts: Duration,
}

impl DemoScenario {
    /// Create a builder for demo scenario.
    pub fn builder() -> DemoScenarioBuilder {
        DemoScenarioBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct DemoScenarioBuilder {
    name: Option<String>,
    description: Option<String>,
    prompts: Vec<String>,
    delay_between_prompts: Option<Duration>,
}

impl DemoScenarioBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompts.push(prompt.into());
        self
    }

    pub fn prompts(mut self, prompts: Vec<String>) -> Self {
        self.prompts = prompts;
        self
    }

    pub fn delay_between_prompts(mut self, delay: Duration) -> Self {
        self.delay_between_prompts = Some(delay);
        self
    }

    pub fn build(self) -> ActorResult<DemoScenario> {
        let name = self.name.ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "DemoScenario requires a name".to_string(),
            ))
        })?;

        let description = self.description.ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "DemoScenario requires a description".to_string(),
            ))
        })?;

        if self.prompts.is_empty() {
            return Err(ActorError::new(ActorErrorKind::InvalidConfiguration(
                "DemoScenario requires at least one prompt".to_string(),
            )));
        }

        Ok(DemoScenario {
            name,
            description,
            prompts: self.prompts,
            delay_between_prompts: self.delay_between_prompts.unwrap_or(Duration::from_secs(2)),
        })
    }
}

/// Executor for running demo scenarios.
#[async_trait]
pub trait DemoExecutor: Send + Sync {
    /// Execute a demo scenario and return responses.
    async fn execute_scenario(&mut self, scenario: &DemoScenario) -> ActorResult<Vec<String>>;
}

/// Demo executor for chat interface.
pub struct ChatDemoExecutor {
    input_tx: mpsc::UnboundedSender<String>,
    output_rx: mpsc::UnboundedReceiver<String>,
}

impl ChatDemoExecutor {
    /// Create a new chat demo executor.
    pub fn new(
        input_tx: mpsc::UnboundedSender<String>,
        output_rx: mpsc::UnboundedReceiver<String>,
    ) -> Self {
        Self {
            input_tx,
            output_rx,
        }
    }
}

#[async_trait]
impl DemoExecutor for ChatDemoExecutor {
    #[instrument(skip(self, scenario))]
    async fn execute_scenario(&mut self, scenario: &DemoScenario) -> ActorResult<Vec<String>> {
        info!(name = %scenario.name, "Starting demo scenario");
        debug!(description = %scenario.description, "Scenario details");

        let mut responses = Vec::new();

        for (i, prompt) in scenario.prompts.iter().enumerate() {
            info!(
                prompt_num = i + 1,
                total = scenario.prompts.len(),
                "Sending prompt"
            );
            debug!(prompt = %prompt, "Prompt content");

            self.input_tx.send(prompt.clone()).map_err(|e| {
                ActorError::new(ActorErrorKind::PlatformPermanent(format!(
                    "Failed to send prompt: {}",
                    e
                )))
            })?;

            tokio::time::sleep(scenario.delay_between_prompts).await;

            if let Ok(Some(resp)) =
                tokio::time::timeout(Duration::from_secs(30), self.output_rx.recv()).await
            {
                debug!(response_len = resp.len(), "Received response");
                responses.push(resp);
            }
        }

        info!(
            responses_collected = responses.len(),
            "Demo scenario complete"
        );
        Ok(responses)
    }
}

/// Create a comprehensive MCP tools demo scenario.
pub fn create_mcp_tools_demo() -> ActorResult<DemoScenario> {
    DemoScenario::builder()
        .name("MCP Tools Comprehensive Demo")
        .description("Exercises all MCP tools through natural language prompts")
        .prompt("Create a narrative to generate social media posts for the MINT navigation center in Grants Pass, Oregon. Use schema inference to structure the posts.")
        .prompt("List all available narratives in the system")
        .prompt("Load the narrative we just created and show me its structure")
        .prompt("Validate the narrative TOML to ensure it's correctly formatted")
        .prompt("List all actors configured in the system")
        .delay_between_prompts(Duration::from_secs(3))
        .build()
}

/// Create a basic chat demo scenario.
pub fn create_basic_demo() -> ActorResult<DemoScenario> {
    DemoScenario::builder()
        .name("Basic Chat Demo")
        .description("Basic interaction to verify chat functionality")
        .prompt("Hello! Can you help me create content?")
        .prompt("What tools do you have available?")
        .delay_between_prompts(Duration::from_secs(2))
        .build()
}
