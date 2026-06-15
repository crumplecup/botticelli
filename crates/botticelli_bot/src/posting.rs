use crate::config::PostingConfig;
use crate::metrics::BotMetrics;
use botticelli_interface::{BotStorage, BotticelliDriver};
use botticelli_narrative::NarrativeExecutor;
use derive_getters::Getters;
use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

/// Message types for posting bot.
#[derive(Debug)]
pub enum PostingMessage {
    /// Post next approved content
    PostNext,
    /// Shutdown the bot
    Shutdown,
}

/// Bot that posts approved content to Discord.
#[derive(Getters)]
pub struct PostingBot<D: BotticelliDriver> {
    config: PostingConfig,
    executor: Arc<NarrativeExecutor<D>>,
    storage: Arc<dyn BotStorage>,
    metrics: Arc<BotMetrics>,
    rx: mpsc::Receiver<PostingMessage>,
}

impl<D: BotticelliDriver> PostingBot<D> {
    /// Creates a new posting bot.
    pub fn new(
        config: PostingConfig,
        executor: Arc<NarrativeExecutor<D>>,
        storage: Arc<dyn BotStorage>,
        metrics: Arc<BotMetrics>,
        rx: mpsc::Receiver<PostingMessage>,
    ) -> Self {
        Self { config, executor, storage, metrics, rx }
    }

    /// Runs the posting bot loop.
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        info!("Posting bot started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                PostingMessage::PostNext => {
                    if let Err(e) = self.post_next_content().await {
                        error!(error = ?e, "Posting failed");
                    }
                }
                PostingMessage::Shutdown => {
                    info!("Posting bot shutting down");
                    break;
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn post_next_content(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        self.metrics.record_posting_execution();

        debug!("Checking for approved content to post");

        let has_approved = self.check_approved_content().await?;

        if !has_approved {
            info!("No approved content available to post");
            self.metrics.record_posting_success();
            return Ok(());
        }

        debug!("Found approved content, executing posting narrative");

        let result = self
            .executor
            .execute_narrative_by_name(
                &self.config.narrative_path().to_string_lossy(),
                self.config.narrative_name(),
            )
            .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                self.metrics.record_posting_success();
                info!(duration_ms = duration.as_millis(), "Successfully posted content");
                Ok(())
            }
            Err(e) => {
                self.metrics.record_posting_failure();
                error!(duration_ms = duration.as_millis(), error = ?e, "Posting failed");
                Err(e.into())
            }
        }
    }

    #[instrument(skip(self))]
    async fn check_approved_content(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let rows = self
            .storage
            .list_content("approved_discord_posts", 10_000)
            .await
            .map_err(|e| format!("Storage error: {e}"))?;

        let has_unposted = rows.iter().any(|r| {
            r.content_json
                .get("posted_at")
                .map(|v| v.is_null())
                .unwrap_or(true)
        });

        Ok(has_unposted)
    }

    /// Calculates next post time with jitter.
    pub fn calculate_next_post_time(&self) -> Duration {
        let base = Duration::from_secs(*self.config.base_interval_hours() * 3600);
        let jitter_secs = *self.config.jitter_minutes() * 60;

        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(0..=jitter_secs);

        if rng.gen_bool(0.5) {
            base + Duration::from_secs(jitter)
        } else {
            base.saturating_sub(Duration::from_secs(jitter))
        }
    }
}
