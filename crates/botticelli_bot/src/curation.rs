use crate::config::CurationConfig;
use crate::metrics::BotMetrics;
use botticelli_interface::{BotStorage, BotticelliDriver};
use botticelli_narrative::NarrativeExecutor;
use derive_getters::Getters;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

/// Message types for curation bot.
#[derive(Debug)]
pub enum CurationMessage {
    /// Check for content to curate
    CheckForContent,
    /// Shutdown the bot
    Shutdown,
}

/// Bot that curates generated content.
#[derive(Getters)]
pub struct CurationBot<D: BotticelliDriver> {
    config: CurationConfig,
    executor: Arc<NarrativeExecutor<D>>,
    storage: Arc<dyn BotStorage>,
    metrics: Arc<BotMetrics>,
    rx: mpsc::Receiver<CurationMessage>,
}

impl<D: BotticelliDriver> CurationBot<D> {
    /// Creates a new curation bot.
    pub fn new(
        config: CurationConfig,
        executor: Arc<NarrativeExecutor<D>>,
        storage: Arc<dyn BotStorage>,
        metrics: Arc<BotMetrics>,
        rx: mpsc::Receiver<CurationMessage>,
    ) -> Self {
        Self { config, executor, storage, metrics, rx }
    }

    /// Runs the curation bot loop.
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        info!("Curation bot started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                CurationMessage::CheckForContent => {
                    if let Err(e) = self.process_pending_content().await {
                        error!(error = ?e, "Curation processing failed");
                    }
                }
                CurationMessage::Shutdown => {
                    info!("Curation bot shutting down");
                    break;
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn process_pending_content(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        self.metrics.record_curation_execution();

        debug!("Checking for pending content");

        let result = async {
            loop {
                let pending_count = self.check_pending_count().await?;

                if pending_count == 0 {
                    info!("No pending content to curate");
                    break;
                }

                debug!(pending_count, "Found pending content, processing batch");

                self.executor
                    .execute_narrative_by_name(
                        &self.config.narrative_path().to_string_lossy(),
                        self.config.narrative_name(),
                    )
                    .await?;

                info!(batch_size = *self.config.batch_size(), "Curated batch");
            }

            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                self.metrics.record_curation_success();
                info!(duration_ms = duration.as_millis(), "Curation completed");
                Ok(())
            }
            Err(e) => {
                self.metrics.record_curation_failure();
                error!(duration_ms = duration.as_millis(), error = ?e, "Curation failed");
                Err(e)
            }
        }
    }

    #[instrument(skip(self))]
    async fn check_pending_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let rows = self
            .storage
            .list_content("potential_discord_posts", 1)
            .await
            .map_err(|e| format!("Storage error: {e}"))?;
        Ok(rows.len())
    }
}
