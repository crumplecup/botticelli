use botticelli_database::establish_connection;
use botticelli_models::GeminiClient;
use botticelli_narrative::{MultiNarrative, NarrativeExecutor, NarrativeProvider};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, instrument, warn};

/// Messages for the GenerationBot actor
#[derive(Debug, Clone)]
pub enum GenerationMessage {
    /// Start the generation loop
    Start,
    /// Stop the generation loop
    Stop,
    /// Run one generation cycle
    RunCycle,
}

/// Arguments for GenerationBot initialization
#[derive(Debug, Clone)]
pub struct GenerationBotArgs {
    /// How often to run generation
    pub interval: Duration,
    /// Path to generation narrative
    pub narrative_path: PathBuf,
    /// Narrative name within the file
    pub narrative_name: String,
}

/// Bot that generates content on a schedule
pub struct GenerationBot {
    args: GenerationBotArgs,
}

impl GenerationBot {
    /// Creates a new generation bot
    pub fn new(args: GenerationBotArgs) -> Self {
        Self { args }
    }

    #[instrument(skip(self))]
    async fn run_generation_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Running generation cycle");

        // Load narrative with database connection
        let mut conn = establish_connection()?;
        let multi_narrative = MultiNarrative::from_file_with_db(
            &self.args.narrative_path,
            &self.args.narrative_name,
            &mut conn,
        )?;

        tracing::debug!(
            narrative_key = %self.args.narrative_name,
            narrative_name = %multi_narrative.name(),
            "Loaded multi-narrative structure"
        );

        // Create executor with Gemini client
        let client = GeminiClient::new()?;
        let executor = NarrativeExecutor::new(client);

        // Execute the narrative
        match executor.execute(&multi_narrative).await {
            Ok(_) => {
                info!("Generation cycle completed successfully");
                Ok(())
            }
            Err(e) => {
                error!(error = ?e, "Generation narrative failed");
                Err(e.into())
            }
        }
    }
}

/// State for the Generation Bot
pub struct GenerationBotState {
    running: bool,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

#[async_trait::async_trait]
impl Actor for GenerationBot {
    type Msg = GenerationMessage;
    type State = GenerationBotState;
    type Arguments = GenerationBotArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: GenerationBotArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(
            interval_secs = ?args.interval.as_secs(),
            narrative = %args.narrative_name,
            "GenerationBot starting"
        );
        Ok(GenerationBotState {
            running: false,
            task_handle: None,
        })
    }

    #[instrument(skip(self, myself, state))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            GenerationMessage::Start => {
                if state.running {
                    warn!("Generation loop already running");
                    return Ok(());
                }

                info!("Starting generation loop");
                state.running = true;

                // Spawn background task for periodic execution
                let interval = self.args.interval;
                let myself_clone = myself.clone();
                let handle = tokio::spawn(async move {
                    let mut ticker = time::interval(interval);
                    loop {
                        ticker.tick().await;
                        if let Err(e) = myself_clone.send_message(GenerationMessage::RunCycle) {
                            error!(error = ?e, "Failed to send RunCycle message");
                            break;
                        }
                    }
                });
                state.task_handle = Some(handle);
            }
            GenerationMessage::Stop => {
                info!("Stopping generation loop");
                state.running = false;
                if let Some(handle) = state.task_handle.take() {
                    handle.abort();
                }
            }
            GenerationMessage::RunCycle => {
                if let Err(e) = self.run_generation_cycle().await {
                    error!(error = ?e, "Generation cycle failed");
                }
            }
        }
        Ok(())
    }
}
