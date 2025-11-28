use crate::config::{BotConfig, BotSchedule};
use crate::curation::{CurationBot, CurationMessage};
use crate::generation::{GenerationBot, GenerationMessage};
use crate::posting::{PostingBot, PostingMessage};
use botticelli_interface::BotticelliDriver;
use botticelli_narrative::NarrativeExecutor;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{error, info, instrument};

/// Bot server that orchestrates all three bots.
pub struct BotServer<D: BotticelliDriver> {
    config: BotConfig,
    schedule: BotSchedule,
    executor: Arc<NarrativeExecutor<D>>,
    database: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl<D: BotticelliDriver + Send + Sync + 'static> BotServer<D> {
    /// Creates a new bot server.
    pub fn new(
        config: BotConfig,
        executor: NarrativeExecutor<D>,
        database: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        let schedule = BotSchedule::from(&config);
        Self {
            config,
            schedule,
            executor: Arc::new(executor),
            database: Arc::new(database),
        }
    }

    /// Starts the bot server and all bot actors.
    #[instrument(skip(self))]
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting bot server");

        // Create channels
        let (gen_tx, gen_rx) = mpsc::channel(32);
        let (cur_tx, cur_rx) = mpsc::channel(32);
        let (post_tx, post_rx) = mpsc::channel(32);

        // Spawn bot actors
        let generation_bot = GenerationBot::new(
            self.config.generation.clone(),
            Arc::clone(&self.executor),
            gen_rx,
        );

        let curation_bot = CurationBot::new(
            self.config.curation.clone(),
            Arc::clone(&self.executor),
            Arc::clone(&self.database),
            cur_rx,
        );

        let posting_bot = PostingBot::new(
            self.config.posting.clone(),
            Arc::clone(&self.executor),
            Arc::clone(&self.database),
            post_rx,
        );

        tokio::spawn(async move {
            generation_bot.run().await;
        });

        tokio::spawn(async move {
            curation_bot.run().await;
        });

        let posting_handle = tokio::spawn(async move {
            posting_bot.run().await;
        });

        // Spawn schedulers
        Self::spawn_generation_scheduler_static(self.schedule.generation_interval, gen_tx);
        Self::spawn_curation_scheduler_static(self.schedule.curation_interval, cur_tx);
        Self::spawn_posting_scheduler_static(
            self.config.posting.base_interval_hours,
            self.config.posting.jitter_minutes,
            post_tx,
        );

        // Wait for posting bot (it drives the main loop)
        posting_handle.await?;

        info!("Bot server stopped");
        Ok(())
    }

    fn spawn_generation_scheduler_static(
        generation_interval: std::time::Duration,
        tx: mpsc::Sender<GenerationMessage>,
    ) {
        tokio::spawn(async move {
            let mut interval = interval(generation_interval);
            loop {
                interval.tick().await;
                if tx.send(GenerationMessage::Generate).await.is_err() {
                    error!("Generation bot channel closed");
                    break;
                }
            }
        });
    }

    fn spawn_curation_scheduler_static(
        curation_interval: std::time::Duration,
        tx: mpsc::Sender<CurationMessage>,
    ) {
        tokio::spawn(async move {
            let mut interval = interval(curation_interval);
            loop {
                interval.tick().await;
                if tx.send(CurationMessage::CheckForContent).await.is_err() {
                    error!("Curation bot channel closed");
                    break;
                }
            }
        });
    }

    fn spawn_posting_scheduler_static(
        base_interval_hours: u64,
        jitter_minutes: u64,
        tx: mpsc::Sender<PostingMessage>,
    ) {
        tokio::spawn(async move {
            loop {
                // Calculate next post time with jitter
                let base = std::time::Duration::from_secs(base_interval_hours * 3600);
                let jitter_secs = jitter_minutes * 60;
                
                let next_post = {
                    let mut rng = rand::thread_rng();
                    let jitter = rng.gen_range(0..=jitter_secs);
                    
                    if rng.gen_bool(0.5) {
                        base + std::time::Duration::from_secs(jitter)
                    } else {
                        base.saturating_sub(std::time::Duration::from_secs(jitter))
                    }
                };
                
                info!(delay_secs = next_post.as_secs(), "Next post scheduled");
                
                sleep(next_post).await;
                
                if tx.send(PostingMessage::PostNext).await.is_err() {
                    error!("Posting bot channel closed");
                    break;
                }
            }
        });
    }
}
