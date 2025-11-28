use crate::config::PostingConfig;
use botticelli_interface::BotticelliDriver;
use botticelli_narrative::NarrativeExecutor;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
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
pub struct PostingBot<D: BotticelliDriver> {
    config: PostingConfig,
    executor: Arc<NarrativeExecutor<D>>,
    database: Arc<Pool<ConnectionManager<PgConnection>>>,
    rx: mpsc::Receiver<PostingMessage>,
}

impl<D: BotticelliDriver> PostingBot<D> {
    /// Creates a new posting bot.
    pub fn new(
        config: PostingConfig,
        executor: Arc<NarrativeExecutor<D>>,
        database: Arc<Pool<ConnectionManager<PgConnection>>>,
        rx: mpsc::Receiver<PostingMessage>,
    ) -> Self {
        Self {
            config,
            executor,
            database,
            rx,
        }
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
        debug!("Checking for approved content to post");

        let has_approved = self.check_approved_content().await?;

        if !has_approved {
            info!("No approved content available to post");
            return Ok(());
        }

        debug!("Found approved content, executing posting narrative");

        self.executor
            .execute_narrative_by_name(
                &self.config.narrative_path.to_string_lossy(),
                &self.config.narrative_name,
            )
            .await?;

        info!("Successfully posted content");
        Ok(())
    }

    async fn check_approved_content(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut conn = self.database.get()?;
        
        use diesel::prelude::*;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;
        
        let count: i64 = diesel::select(sql::<BigInt>(
            "COUNT(*) FROM approved_discord_posts WHERE posted_at IS NULL"
        ))
        .get_result(&mut conn)
        .unwrap_or(0);
        
        Ok(count > 0)
    }

    /// Calculates next post time with jitter.
    pub fn calculate_next_post_time(&self) -> Duration {
        let base = Duration::from_secs(self.config.base_interval_hours * 3600);
        let jitter_secs = self.config.jitter_minutes * 60;
        
        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(0..=jitter_secs);
        
        // Add or subtract jitter randomly
        if rng.gen_bool(0.5) {
            base + Duration::from_secs(jitter)
        } else {
            base.saturating_sub(Duration::from_secs(jitter))
        }
    }
}
