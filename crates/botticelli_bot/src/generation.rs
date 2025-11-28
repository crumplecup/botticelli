use crate::config::GenerationConfig;
use botticelli_interface::BotticelliDriver;
use botticelli_narrative::NarrativeExecutor;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

/// Message types for generation bot.
#[derive(Debug)]
pub enum GenerationMessage {
    /// Trigger content generation
    Generate,
    /// Shutdown the bot
    Shutdown,
}

/// Bot that generates content on a schedule.
pub struct GenerationBot<D: BotticelliDriver> {
    config: GenerationConfig,
    executor: Arc<NarrativeExecutor<D>>,
    rx: mpsc::Receiver<GenerationMessage>,
}

impl<D: BotticelliDriver> GenerationBot<D> {
    /// Creates a new generation bot.
    pub fn new(
        config: GenerationConfig,
        executor: Arc<NarrativeExecutor<D>>,
        rx: mpsc::Receiver<GenerationMessage>,
    ) -> Self {
        Self {
            config,
            executor,
            rx,
        }
    }

    /// Runs the generation bot loop.
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        info!("Generation bot started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                GenerationMessage::Generate => {
                    if let Err(e) = self.generate_content().await {
                        error!(error = ?e, "Content generation failed");
                    }
                }
                GenerationMessage::Shutdown => {
                    info!("Generation bot shutting down");
                    break;
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn generate_content(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!(
            narrative = %self.config.narrative_name,
            "Starting content generation"
        );

        self.executor
            .execute_narrative_by_name(
                &self.config.narrative_path.to_string_lossy(),
                &self.config.narrative_name,
            )
            .await?;

        info!("Content generation completed");
        Ok(())
    }
}
