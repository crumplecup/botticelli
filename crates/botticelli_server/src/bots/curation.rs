use botticelli_interface::BotticelliDriver;
use botticelli_narrative::{MultiNarrative, NarrativeExecutor};
use derive_new::new;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{error, info, instrument, warn};

/// Messages for the CurationBot actor
#[derive(Debug, Clone)]
pub enum CurationMessage {
    /// Start the curation loop
    Start,
    /// Stop the curation loop
    Stop,
    /// Check for content and curate until queue is empty
    ProcessQueue,
}

/// Arguments for CurationBot initialization
#[derive(Debug, Clone, new)]
pub struct CurationBotArgs {
    /// How often to check for content
    pub check_interval: Duration,
    /// Path to curation narrative
    pub narrative_path: PathBuf,
    /// Narrative name within the file
    pub narrative_name: String,
}

/// Bot that curates generated content
pub struct CurationBot {
    driver: Arc<dyn BotticelliDriver>,
    args: CurationBotArgs,
}

impl CurationBot {
    /// Creates a new curation bot with the given driver and args.
    pub fn new(driver: Arc<dyn BotticelliDriver>, args: CurationBotArgs) -> Self {
        Self { driver, args }
    }

    #[instrument(skip(self))]
    async fn process_curation_queue(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting queue processing cycle");

        let narrative =
            MultiNarrative::from_file(&self.args.narrative_path, &self.args.narrative_name)?;

        let executor = NarrativeExecutor::new(self.driver.clone());

        info!("Executing curation narrative");
        let start = Instant::now();
        match executor.execute(&narrative).await {
            Ok(_) => {
                info!(
                    elapsed_secs = start.elapsed().as_secs(),
                    "Curation cycle completed"
                );
                Ok(())
            }
            Err(e) => {
                error!(elapsed_secs = start.elapsed().as_secs(), error = ?e, "Curation narrative failed");
                Err(e.into())
            }
        }
    }
}

/// State for the curation bot actor
pub struct CurationBotState {
    running: bool,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

#[async_trait::async_trait]
impl Actor for CurationBot {
    type Msg = CurationMessage;
    type State = CurationBotState;
    type Arguments = CurationBotArgs;

    #[instrument(skip(self, _myself, args), fields(narrative = %args.narrative_name))]
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: CurationBotArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(
            check_interval_hours = ?args.check_interval.as_secs() / 3600,
            narrative = %args.narrative_name,
            "CurationBot starting"
        );
        Ok(CurationBotState {
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
            CurationMessage::Start => {
                if state.running {
                    warn!("Curation loop already running");
                    return Ok(());
                }

                info!("Starting curation loop");
                state.running = true;

                let check_interval = self.args.check_interval;
                let myself_clone = myself.clone();
                let handle = tokio::spawn(async move {
                    let mut timer = time::interval(check_interval);
                    loop {
                        timer.tick().await;
                        if let Err(e) = myself_clone.send_message(CurationMessage::ProcessQueue) {
                            error!(error = ?e, "Failed to send ProcessQueue message");
                            break;
                        }
                    }
                });
                state.task_handle = Some(handle);
            }
            CurationMessage::Stop => {
                info!("Stopping curation loop");
                state.running = false;
                if let Some(handle) = state.task_handle.take() {
                    handle.abort();
                }
            }
            CurationMessage::ProcessQueue => {
                if let Err(e) = self.process_curation_queue().await {
                    error!(error = ?e, "Queue processing failed");
                }
            }
        }
        Ok(())
    }
}
