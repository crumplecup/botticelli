//! In-process transport for MCP client/server communication.
//!
//! Provides channel-based communication between a pmcp::Client and pmcp::Server
//! in the same process, avoiding stdio or HTTP overhead.

use async_trait::async_trait;
use pmcp::types::TransportMessage;
use pmcp::{Server, Transport};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, instrument};

/// In-process transport using channels.
///
/// This transport connects a pmcp::Client to a pmcp::Server running
/// in the same process via tokio channels, avoiding external I/O overhead.
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp::transport::InProcTransport;
/// use pmcp::{Server, Client};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Build MCP server
/// let server = Server::builder()
///     .name("my-server")
///     .version("0.1.0")
///     .build()?;
///
/// // Create paired transports
/// let (client_transport, server_transport) = InProcTransport::pair();
///
/// // Spawn server task
/// tokio::spawn(async move {
///     server.run(server_transport).await.unwrap();
/// });
///
/// // Create client with transport
/// let mut client = Client::new(client_transport);
/// client.initialize().await?;
///
/// // Use client...
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct InProcTransport {
    /// Channel for sending messages
    tx: mpsc::Sender<TransportMessage>,
    /// Channel for receiving messages
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<TransportMessage>>>,
}

/// Server handle that manages the server task lifecycle.
pub struct InProcServerHandle {
    /// The server task handle
    task: JoinHandle<pmcp::Result<()>>,
}

impl InProcTransport {
    /// Create a paired client and server transport.
    ///
    /// Returns (client_transport, server_transport) where:
    /// - client_transport is used to create pmcp::Client
    /// - server_transport is passed to server.run()
    ///
    /// # Returns
    ///
    /// A tuple of (client transport, server transport)
    #[instrument]
    pub fn pair() -> (Self, Self) {
        debug!("Creating InProcTransport pair");

        // Create bidirectional channels
        let (client_tx, server_rx) = mpsc::channel(100);
        let (server_tx, client_rx) = mpsc::channel(100);

        let client_transport = Self {
            tx: client_tx,
            rx: Arc::new(tokio::sync::Mutex::new(client_rx)),
        };

        let server_transport = Self {
            tx: server_tx,
            rx: Arc::new(tokio::sync::Mutex::new(server_rx)),
        };

        debug!("InProcTransport pair created");
        (client_transport, server_transport)
    }

    /// Spawn a server with this transport.
    ///
    /// Convenience method that spawns the server task and returns a handle
    /// to manage its lifecycle.
    ///
    /// # Arguments
    ///
    /// * `server` - The pmcp::Server instance to run
    /// * `transport` - The server-side transport from `pair()`
    ///
    /// # Returns
    ///
    /// A handle to the running server task
    #[instrument(skip(server, transport))]
    pub fn spawn_server(server: Server, transport: Self) -> InProcServerHandle {
        debug!("Spawning in-process server");

        let task = tokio::spawn(async move {
            debug!("Server task starting");
            let result = server.run(transport).await;
            if let Err(ref e) = result {
                error!("Server error: {}", e);
            } else {
                debug!("Server task completed successfully");
            }
            result
        });

        InProcServerHandle { task }
    }
}

#[async_trait]
impl Transport for InProcTransport {
    #[instrument(skip(self, message), fields(msg_type = %std::any::type_name_of_val(&message)))]
    async fn send(&mut self, message: TransportMessage) -> pmcp::Result<()> {
        debug!("Sending message");
        self.tx
            .send(message)
            .await
            .map_err(|e| pmcp::Error::internal(format!("Failed to send message: {}", e)))
    }

    #[instrument(skip(self))]
    async fn receive(&mut self) -> pmcp::Result<TransportMessage> {
        debug!("Receiving message");
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| pmcp::Error::internal("Channel closed".to_string()))
    }

    async fn close(&mut self) -> pmcp::Result<()> {
        debug!("Closing in-process transport");
        Ok(())
    }
}

impl InProcServerHandle {
    /// Wait for the server task to complete.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server task panicked
    /// - The server returned an error
    #[instrument(skip(self))]
    pub async fn wait(self) -> pmcp::Result<()> {
        debug!("Waiting for server task to complete");
        match self.task.await {
            Ok(result) => {
                debug!("Server task joined successfully");
                result
            }
            Err(e) => {
                error!("Server task panicked: {}", e);
                Err(pmcp::Error::internal(format!("Server task panicked: {}", e)))
            }
        }
    }

    /// Abort the server task.
    ///
    /// This forcefully terminates the server without waiting for graceful shutdown.
    #[instrument(skip(self))]
    pub fn abort(self) {
        debug!("Aborting server task");
        self.task.abort();
    }
}
