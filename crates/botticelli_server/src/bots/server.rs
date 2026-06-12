use crate::{
    CurationBot, CurationBotArgs, CurationMessage, GenerationBot, GenerationBotArgs,
    GenerationMessage, PostingBot, PostingBotArgs, PostingMessage,
};
#[cfg(feature = "metrics")]
use crate::{MetricsCollector, create_metrics_router};
use botticelli_error::{BotticelliError, BotticelliResult, ServerError, ServerErrorKind};
use botticelli_interface::BotticelliDriver;
use ractor::{Actor, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "metrics")]
use tokio::task::JoinHandle;
#[cfg(not(feature = "metrics"))]
use tracing::warn;
use tracing::{error, info, instrument};

/// Bot server that orchestrates generation, curation, and posting actors.
pub struct BotServer {
    driver: Arc<dyn BotticelliDriver>,
    generation_ref: Option<ActorRef<GenerationMessage>>,
    curation_ref: Option<ActorRef<CurationMessage>>,
    posting_ref: Option<ActorRef<PostingMessage>>,
    #[cfg(feature = "metrics")]
    metrics_collector: Arc<MetricsCollector>,
    #[cfg(feature = "metrics")]
    metrics_server_handle: Option<JoinHandle<()>>,
}

impl BotServer {
    /// Creates a new bot server with the given inference driver.
    pub fn new(driver: Arc<dyn BotticelliDriver>) -> Self {
        Self {
            driver,
            generation_ref: None,
            curation_ref: None,
            posting_ref: None,
            #[cfg(feature = "metrics")]
            metrics_collector: Arc::new(MetricsCollector::new()),
            #[cfg(feature = "metrics")]
            metrics_server_handle: None,
        }
    }

    /// Gets a reference to the metrics collector.
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.metrics_collector)
    }

    /// Starts all bots with their respective intervals and the metrics HTTP server.
    #[instrument(skip(self))]
    pub async fn start(
        &mut self,
        generation_interval: Duration,
        curation_interval: Duration,
        posting_interval: Duration,
        metrics_port: Option<u16>,
    ) -> BotticelliResult<()> {
        info!("Starting bot server");

        // Start metrics HTTP server if port is provided
        #[cfg(feature = "metrics")]
        if let Some(port) = metrics_port {
            info!(port = port, "Starting metrics HTTP server");
            let app = create_metrics_router(Arc::clone(&self.metrics_collector));
            let addr = format!("0.0.0.0:{}", port);

            let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
                error!(error = ?e, addr = %addr, "Failed to bind metrics server");
                BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                    format!("Failed to bind metrics server: {}", e),
                )))
            })?;

            let handle = tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app).await {
                    error!(error = ?e, "Metrics server error");
                }
            });

            self.metrics_server_handle = Some(handle);
            info!(port = port, "Metrics HTTP server started");
        }

        #[cfg(not(feature = "metrics"))]
        if metrics_port.is_some() {
            warn!("Metrics port specified but metrics feature not enabled");
        }

        let narratives_dir = PathBuf::from("./crates/botticelli_narrative/narratives/discord");

        // Spawn generation bot
        let generation_args = GenerationBotArgs::new(
            generation_interval,
            narratives_dir.join("generation_carousel.toml"),
            "batch_generate".to_string(),
        );

        info!(
            narrative = %generation_args.narrative_name,
            path = ?generation_args.narrative_path,
            interval_hours = generation_interval.as_secs() / 3600,
            "Spawning generation bot"
        );
        let (generation_ref, _) = Actor::spawn(
            Some("generation_bot".to_string()),
            GenerationBot::new(self.driver.clone(), generation_args.clone()),
            generation_args,
        )
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to spawn generation bot");
            BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                "Failed to spawn generation bot".to_string(),
            )))
        })?;

        self.generation_ref = Some(generation_ref.clone());
        generation_ref
            .send_message(GenerationMessage::Start)
            .map_err(|e| {
                error!(error = ?e, "Failed to start generation bot");
                BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                    "Failed to start generation bot".to_string(),
                )))
            })?;
        info!("Generation bot started");

        // Spawn curation bot
        let curation_args = CurationBotArgs::new(
            curation_interval,
            narratives_dir.join("curation.toml"),
            "curate_and_approve".to_string(),
        );

        info!(
            narrative = %curation_args.narrative_name,
            path = ?curation_args.narrative_path,
            interval_hours = curation_interval.as_secs() / 3600,
            "Spawning curation bot"
        );
        let (curation_ref, _) = Actor::spawn(
            Some("curation_bot".to_string()),
            CurationBot::new(self.driver.clone(), curation_args.clone()),
            curation_args,
        )
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to spawn curation bot");
            BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                "Failed to spawn curation bot".to_string(),
            )))
        })?;

        self.curation_ref = Some(curation_ref.clone());
        curation_ref
            .send_message(CurationMessage::Start)
            .map_err(|e| {
                error!(error = ?e, "Failed to start curation bot");
                BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                    "Failed to start curation bot".to_string(),
                )))
            })?;
        info!("Curation bot started");

        // Spawn posting bot
        let posting_args = PostingBotArgs::new(
            posting_interval,
            0.2,
            narratives_dir.join("posting.toml"),
            "post_approved".to_string(),
        );

        info!(
            narrative = %posting_args.narrative_name,
            path = ?posting_args.narrative_path,
            interval_hours = posting_interval.as_secs() / 3600,
            "Spawning posting bot"
        );
        let (posting_ref, _) = Actor::spawn(
            Some("posting_bot".to_string()),
            PostingBot::new(self.driver.clone(), posting_args.clone()),
            posting_args,
        )
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to spawn posting bot");
            BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                "Failed to spawn posting bot".to_string(),
            )))
        })?;

        self.posting_ref = Some(posting_ref.clone());
        posting_ref
            .send_message(PostingMessage::Start)
            .map_err(|e| {
                error!(error = ?e, "Failed to start posting bot");
                BotticelliError::from(ServerError::new(ServerErrorKind::ServerStartFailed(
                    "Failed to start posting bot".to_string(),
                )))
            })?;
        info!("Posting bot started");

        info!("All bots started");
        Ok(())
    }

    /// Stops all bots.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> BotticelliResult<()> {
        info!("Stopping bot server");

        if let Some(ref generation) = self.generation_ref {
            let _ = generation.send_message(GenerationMessage::Stop);
            generation.stop(None);
        }

        if let Some(ref cur) = self.curation_ref {
            let _ = cur.send_message(CurationMessage::Stop);
            cur.stop(None);
        }

        if let Some(ref post) = self.posting_ref {
            let _ = post.send_message(PostingMessage::Stop);
            post.stop(None);
        }

        self.generation_ref = None;
        self.curation_ref = None;
        self.posting_ref = None;

        info!("All bots stopped");
        Ok(())
    }

    /// Returns whether the server is running.
    pub fn is_running(&self) -> bool {
        self.generation_ref.is_some() || self.curation_ref.is_some() || self.posting_ref.is_some()
    }
}
