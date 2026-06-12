use botticelli_interface::BotticelliDriver;
use botticelli_narrative::{MultiNarrative, NarrativeExecutor, NarrativeProvider};
use derive_new::new;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
#[derive(Debug, Clone, new)]
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
    driver: Arc<dyn BotticelliDriver>,
    args: GenerationBotArgs,
}

impl GenerationBot {
    /// Creates a new generation bot with the given driver and args.
    pub fn new(driver: Arc<dyn BotticelliDriver>, args: GenerationBotArgs) -> Self {
        Self { driver, args }
    }

    #[instrument(skip(self))]
    async fn run_generation_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Running generation cycle");

        let multi_narrative =
            MultiNarrative::from_file(&self.args.narrative_path, &self.args.narrative_name)?;

        tracing::debug!(
            narrative_key = %self.args.narrative_name,
            narrative_name = %multi_narrative.name(),
            "Loaded multi-narrative structure"
        );

        let executor = NarrativeExecutor::new(self.driver.clone());

        info!("Executing generation narrative");
        let start = Instant::now();
        match executor.execute(&multi_narrative).await {
            Ok(_) => {
                info!(
                    elapsed_secs = start.elapsed().as_secs(),
                    "Generation cycle completed"
                );
                Ok(())
            }
            Err(e) => {
                error!(elapsed_secs = start.elapsed().as_secs(), error = ?e, "Generation narrative failed");
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

    #[instrument(skip(self, _myself, args), fields(narrative = %args.narrative_name))]
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
