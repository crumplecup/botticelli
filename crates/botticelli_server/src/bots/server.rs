use crate::{
    CurationBot, CurationBotArgs, CurationMessage, GenerationBot, GenerationBotArgs,
    GenerationMessage, PostingBot, PostingBotArgs, PostingMessage,
};
use botticelli_error::{BotticelliError, BotticelliResult, ServerError, ServerErrorKind};
use ractor::{Actor, ActorRef};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, instrument};

/// Bot server that orchestrates generation, curation, and posting actors.
pub struct BotServer {
    generation_ref: Option<ActorRef<GenerationMessage>>,
    curation_ref: Option<ActorRef<CurationMessage>>,
    posting_ref: Option<ActorRef<PostingMessage>>,
}

impl BotServer {
    /// Creates a new bot server.
    pub fn new() -> Self {
        Self {
            generation_ref: None,
            curation_ref: None,
            posting_ref: None,
        }
    }

    /// Starts all bots with their respective intervals.
    #[instrument(skip(self))]
    pub async fn start(
        &mut self,
        generation_interval: Duration,
        curation_interval: Duration,
        posting_interval: Duration,
    ) -> BotticelliResult<()> {
        info!("Starting bot server");

        // Define narrative paths (relative to workspace root)
        let narratives_dir = PathBuf::from("./crates/botticelli_narrative/narratives/discord");

        // Spawn generation bot
        let generation_args = GenerationBotArgs {
            interval: generation_interval,
            narrative_path: narratives_dir.join("generation_carousel.toml"),
            narrative_name: "batch_generate".to_string(),
        };

        let (generation_ref, _) = Actor::spawn(
            Some("generation_bot".to_string()),
            GenerationBot::new(generation_args.clone()),
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

        // Spawn curation bot
        let curation_args = CurationBotArgs {
            check_interval: curation_interval,
            narrative_path: narratives_dir.join("curation.toml"),
            narrative_name: "curate_and_approve".to_string(),
        };

        let (curation_ref, _) = Actor::spawn(
            Some("curation_bot".to_string()),
            CurationBot::new(curation_args.clone()),
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

        // Spawn posting bot
        let posting_args = PostingBotArgs {
            base_interval: posting_interval,
            jitter_percent: 0.2,
            narrative_path: narratives_dir.join("posting.toml"),
            narrative_name: "post_approved".to_string(),
        };

        let (posting_ref, _) = Actor::spawn(
            Some("posting_bot".to_string()),
            PostingBot::new(posting_args.clone()),
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

impl Default for BotServer {
    fn default() -> Self {
        Self::new()
    }
}
