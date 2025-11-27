use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::interval;
use tracing::{debug, error, info, instrument, warn};

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

/// Bot that curates generated content
pub struct CurationBot {
    narrative_path: PathBuf,
    state_dir: PathBuf,
    botticelli_bin: PathBuf,
}

impl CurationBot {
    /// Creates a new curation bot
    pub fn new(
        narrative_path: PathBuf,
        state_dir: PathBuf,
        botticelli_bin: Option<PathBuf>,
    ) -> Self {
        Self {
            narrative_path,
            state_dir,
            botticelli_bin: botticelli_bin.unwrap_or_else(|| PathBuf::from("botticelli")),
        }
    }

    #[instrument(skip(self))]
    async fn process_curation_queue(&self) -> Result<usize, Box<dyn std::error::Error>> {
        info!("Starting queue processing cycle");
        
        // Execute curation narrative via CLI
        let output = Command::new(&self.botticelli_bin)
            .arg("run")
            .arg("--narrative")
            .arg(&self.narrative_path)
            .arg("--narrative-name")
            .arg("curate_and_approve")
            .arg("--save")
            .arg("--state-dir")
            .arg(&self.state_dir)
            .arg("--process-discord")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            debug!(stdout = %stdout, stderr = %stderr, "Curation narrative completed");
            info!("Queue processing complete");
            
            // For MVP, assume we processed content if command succeeded
            // TODO: Parse output to get actual count
            Ok(10)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(stderr = %stderr, status = %output.status, "Curation narrative failed");
            Err(format!("Curation command failed: {}", stderr).into())
        }
    }
}

/// State for the curation bot actor
pub struct CurationBotState {
    running: bool,
    check_interval: Duration,
}

#[async_trait::async_trait]
impl Actor for CurationBot {
    type Msg = CurationMessage;
    type State = CurationBotState;
    type Arguments = Duration;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        check_interval: Duration,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(check_interval_hours = ?check_interval.as_secs() / 3600, "CurationBot starting");
        Ok(CurationBotState {
            running: false,
            check_interval,
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
                
                // Spawn background task for periodic checks
                let actor_ref = myself.clone();
                let check_interval = state.check_interval;
                
                tokio::spawn(async move {
                    let mut timer = interval(check_interval);
                    
                    loop {
                        timer.tick().await;
                        
                        if let Err(e) = actor_ref.send_message(CurationMessage::ProcessQueue) {
                            error!(error = ?e, "Failed to send ProcessQueue message");
                            break;
                        }
                    }
                });
            }
            CurationMessage::Stop => {
                info!("Stopping curation loop");
                state.running = false;
            }
            CurationMessage::ProcessQueue => {
                match self.process_curation_queue().await {
                    Ok(count) => info!(processed = count, "Queue processed"),
                    Err(e) => error!(error = ?e, "Queue processing failed"),
                }
            }
        }
        Ok(())
    }
}
