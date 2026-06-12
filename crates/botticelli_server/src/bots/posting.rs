use botticelli_interface::BotticelliDriver;
use botticelli_narrative::{MultiNarrative, NarrativeExecutor};
use derive_new::new;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use rand::Rng;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, error, info, instrument, warn};

/// Messages for the PostingBot actor
#[derive(Debug, Clone)]
pub enum PostingMessage {
    /// Start the posting loop
    Start,
    /// Stop the posting loop
    Stop,
    /// Post one piece of approved content
    PostNext,
}

/// Arguments for PostingBot initialization
#[derive(Debug, Clone, new)]
pub struct PostingBotArgs {
    /// Base interval between posts
    pub base_interval: Duration,
    /// Jitter percentage (0.0 to 1.0)
    pub jitter_percent: f64,
    /// Path to posting narrative
    pub narrative_path: PathBuf,
    /// Narrative name within the file
    pub narrative_name: String,
}

/// Bot that posts curated content to Discord
pub struct PostingBot {
    driver: Arc<dyn BotticelliDriver>,
    args: PostingBotArgs,
}

impl PostingBot {
    /// Creates a new posting bot with the given driver and args.
    pub fn new(driver: Arc<dyn BotticelliDriver>, args: PostingBotArgs) -> Self {
        Self { driver, args }
    }

    /// Calculate next posting delay with jitter.
    #[instrument(skip(self))]
    fn calculate_next_delay(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_range =
            (self.args.base_interval.as_secs_f64() * self.args.jitter_percent) as i64;
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        let next_secs = (self.args.base_interval.as_secs() as i64 + jitter).max(60);

        let delay = Duration::from_secs(next_secs as u64);
        debug!(
            base_secs = self.args.base_interval.as_secs(),
            jitter_secs = jitter,
            next_secs,
            "Calculated next posting delay"
        );
        delay
    }

    #[instrument(skip(self))]
    async fn post_next_content(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Posting next approved content");

        let narrative =
            MultiNarrative::from_file(&self.args.narrative_path, &self.args.narrative_name)?;

        let executor = NarrativeExecutor::new(self.driver.clone());

        info!("Executing posting narrative");
        let start = Instant::now();
        match executor.execute(&narrative).await {
            Ok(_) => {
                info!(
                    elapsed_secs = start.elapsed().as_secs(),
                    "Posting cycle completed"
                );
                Ok(())
            }
            Err(e) => {
                error!(elapsed_secs = start.elapsed().as_secs(), error = ?e, "Posting narrative failed");
                Err(e.into())
            }
        }
    }
}

/// State for the posting bot actor
pub struct PostingBotState {
    running: bool,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

#[async_trait::async_trait]
impl Actor for PostingBot {
    type Msg = PostingMessage;
    type State = PostingBotState;
    type Arguments = PostingBotArgs;

    #[instrument(skip(self, _myself, args), fields(narrative = %args.narrative_name))]
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: PostingBotArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(
            interval_hours = ?args.base_interval.as_secs() / 3600,
            jitter_percent = args.jitter_percent,
            narrative = %args.narrative_name,
            "PostingBot starting"
        );
        Ok(PostingBotState {
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
            PostingMessage::Start => {
                if state.running {
                    warn!("Posting loop already running");
                    return Ok(());
                }

                info!("Starting posting loop");
                state.running = true;

                let myself_clone = myself.clone();
                let bot_clone = PostingBot::new(self.driver.clone(), self.args.clone());
                let handle = tokio::spawn(async move {
                    loop {
                        let delay = bot_clone.calculate_next_delay();
                        time::sleep(delay).await;

                        if let Err(e) = myself_clone.send_message(PostingMessage::PostNext) {
                            error!(error = ?e, "Failed to send PostNext message");
                            break;
                        }
                    }
                });
                state.task_handle = Some(handle);
            }
            PostingMessage::Stop => {
                info!("Stopping posting loop");
                state.running = false;
                if let Some(handle) = state.task_handle.take() {
                    handle.abort();
                }
            }
            PostingMessage::PostNext => {
                if let Err(e) = self.post_next_content().await {
                    error!(error = ?e, "Posting failed");
                }
            }
        }
        Ok(())
    }
}
