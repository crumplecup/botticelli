//! Server lifecycle management for local inference servers

use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, instrument, warn};

use botticelli_error::{ServerError, ServerErrorKind};

/// Handle for managing a running inference server process
pub struct ServerHandle {
    process: Child,
    port: u16,
    model_path: String,
}

impl ServerHandle {
    /// Start a new inference server process (for internal use)
    #[instrument(skip_all, fields(port = %port, model_path = %model_path))]
    fn start_internal(port: u16, model_path: String, tokenizer_id: String) -> Result<Self, ServerError> {
        info!("Starting inference server on port {}", port);
        
        // Extract directory and filename from model_path
        let path = std::path::Path::new(&model_path);
        let model_dir = path.parent().ok_or_else(|| {
            ServerError::new(ServerErrorKind::ServerStartFailed(
                "Model path must have a parent directory".to_string()
            ))
        })?;
        let filename = path.file_name().ok_or_else(|| {
            ServerError::new(ServerErrorKind::ServerStartFailed(
                "Model path must have a filename".to_string()
            ))
        })?;
        
        debug!(
            model_dir = ?model_dir,
            filename = ?filename,
            tokenizer = %tokenizer_id,
            "Starting mistralrs-server"
        );
        
        let process = Command::new("mistralrs-server")
            .arg("--port")
            .arg(port.to_string())
            .arg("gguf")
            .arg("-m")
            .arg(model_dir)
            .arg("-f")
            .arg(filename)
            .arg("-t")
            .arg(&tokenizer_id)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                ServerError::new(ServerErrorKind::ServerStartFailed(format!(
                    "Failed to spawn mistralrs-server: {}. Make sure it's installed.",
                    e
                )))
            })?;

        debug!("Server process spawned with PID: {:?}", process.id());

        Ok(Self {
            process,
            port,
            model_path,
        })
    }

    /// Wait for the server to be ready to accept requests
    #[instrument(skip(self))]
    pub async fn wait_until_ready(&self, timeout: Duration) -> Result<(), ServerError> {
        info!("Waiting for server to be ready (timeout: {:?})", timeout);
        
        let start = std::time::Instant::now();
        let client = reqwest::Client::new();
        let health_url = format!("http://localhost:{}/health", self.port);

        loop {
            if start.elapsed() > timeout {
                return Err(ServerError::new(ServerErrorKind::ServerStartFailed(
                    format!("Server did not become ready within {:?}", timeout),
                )));
            }

            match client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    info!("Server is ready");
                    return Ok(());
                }
                Ok(response) => {
                    debug!("Server health check returned status: {}", response.status());
                }
                Err(e) => {
                    debug!("Server not ready yet: {}", e);
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    /// Get the port the server is running on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the model path
    pub fn model_path(&self) -> &str {
        &self.model_path
    }

    /// Start a new inference server (CLI-friendly interface)
    #[instrument(skip_all)]
    pub fn start(
        _config: crate::ServerConfig,
        model_path: std::path::PathBuf,
        port: u16,
    ) -> Result<Self, ServerError> {
        let model_path_str = model_path.to_string_lossy().to_string();
        // Use default tokenizer for now - should be configurable later
        let tokenizer_id = "mistralai/Mistral-7B-Instruct-v0.3".to_string();
        Self::start_internal(port, model_path_str, tokenizer_id)
    }

    /// Wait for the server process to exit
    pub async fn wait(mut self) -> Result<(), ServerError> {
        info!("Waiting for server process to exit");
        tokio::task::spawn_blocking(move || {
            self.process.wait().map_err(|e| {
                ServerError::new(ServerErrorKind::ServerStopFailed(format!(
                    "Failed to wait for server: {}",
                    e
                )))
            })
        })
        .await
        .map_err(|e| {
            ServerError::new(ServerErrorKind::ServerStopFailed(format!(
                "Join error: {}",
                e
            )))
        })?
        .map(|_| ())
    }

    /// Stop the server gracefully
    #[instrument(skip(self))]
    pub fn stop(mut self) -> Result<(), ServerError> {
        info!("Stopping inference server");
        
        self.process
            .kill()
            .map_err(|e| {
                ServerError::new(ServerErrorKind::ServerStopFailed(format!(
                    "Failed to stop server: {}",
                    e
                )))
            })?;

        self.process
            .wait()
            .map_err(|e| {
                ServerError::new(ServerErrorKind::ServerStopFailed(format!(
                    "Failed to wait for server shutdown: {}",
                    e
                )))
            })?;

        info!("Server stopped successfully");
        Ok(())
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        warn!("ServerHandle dropped, attempting to kill server process");
        let _ = self.process.kill();
    }
}
