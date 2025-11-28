use crate::config::CurationConfig;
use botticelli_interface::BotticelliDriver;
use botticelli_narrative::NarrativeExecutor;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
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
pub struct CurationBot<D: BotticelliDriver> {
    config: CurationConfig,
    executor: Arc<NarrativeExecutor<D>>,
    database: Arc<Pool<ConnectionManager<PgConnection>>>,
    rx: mpsc::Receiver<CurationMessage>,
}

impl<D: BotticelliDriver> CurationBot<D> {
    /// Creates a new curation bot.
    pub fn new(
        config: CurationConfig,
        executor: Arc<NarrativeExecutor<D>>,
        database: Arc<Pool<ConnectionManager<PgConnection>>>,
        rx: mpsc::Receiver<CurationMessage>,
    ) -> Self {
        Self {
            config,
            executor,
            database,
            rx,
        }
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
        debug!("Checking for pending content");

        loop {
            let pending_count = self.check_pending_count().await?;

            if pending_count == 0 {
                info!("No pending content to curate");
                break;
            }

            debug!(pending_count, "Found pending content");

            // Process batch
            self.executor
                .execute_narrative_by_name(
                    &self.config.narrative_path.to_string_lossy(),
                    &self.config.narrative_name,
                )
                .await?;

            info!(batch_size = self.config.batch_size, "Curated batch");
        }

        Ok(())
    }

    async fn check_pending_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        // Query potential_discord_posts for records with status = "pending"
        let mut conn = self.database.get()?;
        
        use diesel::prelude::*;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;
        
        let count: i64 = diesel::select(sql::<BigInt>(
            "COUNT(*) FROM potential_discord_posts WHERE status = 'pending'"
        ))
        .get_result(&mut conn)
        .unwrap_or(0);
        
        Ok(count as usize)
    }
}
