//! WebSocket client for Gemini Live API.
//!
//! This module provides a WebSocket-based client for the Gemini Live API, enabling
//! bidirectional streaming communication with Gemini models.
//!
//! # Architecture
//!
//! - `GeminiLiveClient` - Factory for creating WebSocket sessions
//! - `LiveSession` - Active WebSocket connection for bidirectional communication
//!
//! # Example
//!
//! ```no_run
//! use botticelli_models::GeminiLiveClient;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiLiveClient::new()?;
//!
//! // Connect and perform setup handshake
//! let mut session = client.connect("models/gemini-2.0-flash-exp").await?;
//!
//! // Send a message
//! let response = session.send_text("Hello!").await?;
//! println!("Response: {}", response);
//!
//! // Close session
//! session.close().await?;
//! # Ok(())
//! # }
//! ```

use futures_util::{SinkExt, StreamExt};
use std::env;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, trace, warn};

use botticelli_core::Output;
use botticelli_error::{GeminiError, GeminiErrorKind};
use botticelli_interface::{FinishReason, StreamChunk};

use super::{GeminiResult, live_protocol::*, live_rate_limit::LiveRateLimiter};

/// WebSocket endpoint for Gemini Live API.
const LIVE_API_ENDPOINT: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";

/// Client for creating Gemini Live API WebSocket sessions.
///
/// This client handles API key management and creates WebSocket connections
/// to the Gemini Live API.
#[derive(Clone)]
pub struct GeminiLiveClient {
    api_key: String,
    rate_limiter: Option<Arc<LiveRateLimiter>>,
}

impl GeminiLiveClient {
    /// Create a new Live API client.
    ///
    /// Reads the API key from the `GEMINI_API_KEY` environment variable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiLiveClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GeminiLiveClient::new()?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_live_client_new")]
    pub fn new() -> GeminiResult<Self> {
        Self::new_with_rate_limit(None)
    }

    /// Create a new Live API client with rate limiting.
    ///
    /// # Arguments
    ///
    /// * `max_messages_per_minute` - Optional RPM limit. If None, no rate limiting is applied.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiLiveClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // With rate limiting (10 messages per minute)
    /// let client = GeminiLiveClient::new_with_rate_limit(Some(10))?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_live_client_new_with_rate_limit")]
    pub fn new_with_rate_limit(max_messages_per_minute: Option<u32>) -> GeminiResult<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| GeminiError::new(GeminiErrorKind::MissingApiKey))?;

        let rate_limiter = max_messages_per_minute.map(|rpm| Arc::new(LiveRateLimiter::new(rpm)));

        Ok(Self {
            api_key,
            rate_limiter,
        })
    }

    /// Connect to the Live API and perform setup handshake.
    ///
    /// Establishes a WebSocket connection, sends the setup message, and waits for
    /// `setupComplete` confirmation before returning the session.
    ///
    /// # Arguments
    ///
    /// * `model` - Model name (e.g., "models/gemini-2.0-flash-exp")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiLiveClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GeminiLiveClient::new()?;
    /// let mut session = client.connect("models/gemini-2.0-flash-exp").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_live_client_connect", skip(self))]
    pub async fn connect(&self, model: &str) -> GeminiResult<LiveSession> {
        LiveSession::new(&self.api_key, model, None, self.rate_limiter.clone()).await
    }

    /// Connect with custom generation config.
    ///
    /// # Arguments
    ///
    /// * `model` - Model name
    /// * `generation_config` - Generation parameters (temperature, max_tokens, etc.)
    #[instrument(name = "gemini_live_client_connect_with_config", skip(self, generation_config))]
    pub async fn connect_with_config(
        &self,
        model: &str,
        generation_config: GenerationConfig,
    ) -> GeminiResult<LiveSession> {
        LiveSession::new(
            &self.api_key,
            model,
            Some(generation_config),
            self.rate_limiter.clone(),
        )
        .await
    }
}

/// Active WebSocket session with the Gemini Live API.
///
/// Provides methods for sending messages and receiving responses over the WebSocket
/// connection.
pub struct LiveSession {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    model: String,
    rate_limiter: Option<Arc<LiveRateLimiter>>,
}

impl LiveSession {
    /// Create a new Live API session.
    ///
    /// Performs WebSocket connection and setup handshake.
    async fn new(
        api_key: &str,
        model: &str,
        generation_config: Option<GenerationConfig>,
        rate_limiter: Option<Arc<LiveRateLimiter>>,
    ) -> GeminiResult<Self> {
        info!("Connecting to Gemini Live API for model: {}", model);

        // Build WebSocket URL with API key
        let url = format!("{}?key={}", LIVE_API_ENDPOINT, api_key);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&url).await.map_err(|e| {
            error!("WebSocket connection failed: {}", e);
            GeminiError::new(GeminiErrorKind::WebSocketConnection(e.to_string()))
        })?;

        debug!("WebSocket connection established");

        let mut session = Self {
            ws_stream,
            model: model.to_string(),
            rate_limiter,
        };

        // Perform setup handshake
        session.setup_handshake(model, generation_config).await?;

        info!("Live API session established for model: {}", model);
        Ok(session)
    }

    /// Perform setup handshake with the server.
    ///
    /// Sends setup message and waits for setupComplete confirmation.
    async fn setup_handshake(
        &mut self,
        model: &str,
        generation_config: Option<GenerationConfig>,
    ) -> GeminiResult<()> {
        debug!("Sending setup message");

        // Build setup message
        let setup = SetupMessage {
            setup: SetupConfig {
                model: model.to_string(),
                generation_config,
                system_instruction: None,
                tools: None,
            },
        };

        // Serialize to JSON
        let json = serde_json::to_string(&setup).map_err(|e| {
            error!("Failed to serialize setup message: {}", e);
            GeminiError::new(GeminiErrorKind::WebSocketHandshake(format!(
                "Serialization error: {}",
                e
            )))
        })?;

        trace!("Setup message JSON: {}", json);

        // Send setup message
        self.ws_stream
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| {
                error!("Failed to send setup message: {}", e);
                GeminiError::new(GeminiErrorKind::WebSocketHandshake(format!(
                    "Send error: {}",
                    e
                )))
            })?;

        debug!("Setup message sent, waiting for setupComplete");

        // Wait for setupComplete response
        while let Some(msg_result) = self.ws_stream.next().await {
            let msg = msg_result.map_err(|e| {
                error!("Error receiving setup response: {}", e);
                GeminiError::new(GeminiErrorKind::WebSocketHandshake(format!(
                    "Receive error: {}",
                    e
                )))
            })?;

            if let Message::Text(text) = msg {
                trace!("Received message: {}", text);

                let server_msg: ServerMessage = serde_json::from_str(&text).map_err(|e| {
                    error!("Failed to parse server message: {}", e);
                    GeminiError::new(GeminiErrorKind::InvalidServerMessage(format!(
                        "Parse error: {}",
                        e
                    )))
                })?;

                if server_msg.is_setup_complete() {
                    debug!("Received setupComplete");
                    return Ok(());
                } else if server_msg.is_go_away() {
                    let reason = server_msg
                        .go_away
                        .map(|ga| ga.reason)
                        .unwrap_or_else(|| "unknown".to_string());
                    error!("Server sent goAway during setup: {}", reason);
                    return Err(GeminiError::new(GeminiErrorKind::ServerDisconnect(reason)));
                } else {
                    warn!("Unexpected message during setup: {:?}", server_msg);
                }
            }
        }

        error!("WebSocket closed before setupComplete received");
        Err(GeminiError::new(GeminiErrorKind::WebSocketHandshake(
            "Connection closed before setup complete".to_string(),
        )))
    }

    /// Send a text message and wait for complete response.
    ///
    /// Sends a user message and collects all response chunks until `turnComplete` is received.
    ///
    /// # Arguments
    ///
    /// * `text` - The user message text
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use botticelli_models::GeminiLiveClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = GeminiLiveClient::new()?;
    /// # let mut session = client.connect("models/gemini-2.0-flash-exp").await?;
    /// let response = session.send_text("Hello, how are you?").await?;
    /// println!("Response: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "live_session_send_text", skip(self))]
    pub async fn send_text(&mut self, text: &str) -> GeminiResult<String> {
        debug!("Sending text message: {}", text);

        // Check rate limit before sending
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        // Build client content message
        let message = ClientContentMessage {
            client_content: ClientContent {
                turns: vec![Turn {
                    role: "user".to_string(),
                    parts: vec![Part::text(text)],
                }],
                turn_complete: true,
            },
        };

        // Serialize to JSON
        let json = serde_json::to_string(&message).map_err(|e| {
            error!("Failed to serialize message: {}", e);
            GeminiError::new(GeminiErrorKind::ApiRequest(format!(
                "Serialization error: {}",
                e
            )))
        })?;

        trace!("Message JSON: {}", json);

        // Send message
        self.ws_stream
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| {
                error!("Failed to send message: {}", e);
                GeminiError::new(GeminiErrorKind::ApiRequest(format!("Send error: {}", e)))
            })?;

        // Record message sent for rate limiting
        if let Some(limiter) = &self.rate_limiter {
            limiter.record();
        }

        debug!("Message sent, waiting for response");

        // Collect response chunks
        let mut full_response = String::new();

        while let Some(msg_result) = self.ws_stream.next().await {
            let msg = msg_result.map_err(|e| {
                error!("Error receiving response: {}", e);
                GeminiError::new(GeminiErrorKind::StreamInterrupted(e.to_string()))
            })?;

            if let Message::Text(text) = msg {
                trace!("Received response chunk: {}", text);

                let server_msg: ServerMessage = serde_json::from_str(&text).map_err(|e| {
                    error!("Failed to parse server message: {}", e);
                    GeminiError::new(GeminiErrorKind::InvalidServerMessage(format!(
                        "Parse error: {}",
                        e
                    )))
                })?;

                // Check for disconnect
                if server_msg.is_go_away() {
                    let reason = server_msg
                        .go_away
                        .map(|ga| ga.reason)
                        .unwrap_or_else(|| "unknown".to_string());
                    error!("Server disconnecting: {}", reason);
                    return Err(GeminiError::new(GeminiErrorKind::ServerDisconnect(reason)));
                }

                // Extract text from response
                if let Some(text) = server_msg.extract_text() {
                    full_response.push_str(&text);
                }

                // Check if turn is complete
                if server_msg.is_turn_complete() {
                    debug!("Turn complete, response length: {}", full_response.len());
                    break;
                }
            } else if let Message::Close(_) = msg {
                warn!("WebSocket closed during response");
                break;
            }
        }

        Ok(full_response)
    }

    /// Send a text message and stream responses incrementally.
    ///
    /// Returns a stream of `StreamChunk` values as the model generates the response.
    /// The final chunk will have `is_final: true`.
    ///
    /// # Arguments
    ///
    /// * `text` - The user message text
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use botticelli_models::GeminiLiveClient;
    /// # use futures_util::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = GeminiLiveClient::new()?;
    /// # let mut session = client.connect("models/gemini-2.0-flash-exp").await?;
    /// let mut stream = session.send_text_stream("Tell me a story").await?;
    ///
    /// while let Some(chunk_result) = stream.next().await {
    ///     let chunk = chunk_result?;
    ///     print!("{:?}", chunk.content);
    ///     if chunk.is_final {
    ///         break;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "live_session_send_text_stream", skip(self))]
    pub async fn send_text_stream(
        mut self,
        text: &str,
    ) -> GeminiResult<
        std::pin::Pin<
            Box<dyn futures_util::stream::Stream<Item = GeminiResult<StreamChunk>> + Send>,
        >,
    > {
        use futures_util::stream;

        debug!("Sending text message for streaming: {}", text);

        // Check rate limit before sending
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        // Build client content message
        let message = ClientContentMessage {
            client_content: ClientContent {
                turns: vec![Turn {
                    role: "user".to_string(),
                    parts: vec![Part::text(text)],
                }],
                turn_complete: true,
            },
        };

        // Serialize to JSON
        let json = serde_json::to_string(&message).map_err(|e| {
            error!("Failed to serialize message: {}", e);
            GeminiError::new(GeminiErrorKind::ApiRequest(format!(
                "Serialization error: {}",
                e
            )))
        })?;

        trace!("Message JSON: {}", json);

        // Send message
        self.ws_stream
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| {
                error!("Failed to send message: {}", e);
                GeminiError::new(GeminiErrorKind::ApiRequest(format!("Send error: {}", e)))
            })?;

        // Record message sent for rate limiting
        if let Some(limiter) = &self.rate_limiter {
            limiter.record();
        }

        debug!("Message sent, returning stream");

        // Create a stream that yields chunks as they arrive
        // The stream owns the WebSocket, so it will be dropped when the stream ends
        let stream = stream::try_unfold((self.ws_stream, false), |(mut ws, done)| async move {
            if done {
                return Ok(None);
            }

            while let Some(msg_result) = ws.next().await {
                let msg = msg_result.map_err(|e| {
                    error!("Error receiving response: {}", e);
                    GeminiError::new(GeminiErrorKind::StreamInterrupted(e.to_string()))
                })?;

                if let Message::Text(text) = msg {
                    trace!("Received response chunk: {}", text);

                    let server_msg: ServerMessage = serde_json::from_str(&text).map_err(|e| {
                        error!("Failed to parse server message: {}", e);
                        GeminiError::new(GeminiErrorKind::InvalidServerMessage(format!(
                            "Parse error: {}",
                            e
                        )))
                    })?;

                    // Check for disconnect
                    if server_msg.is_go_away() {
                        let reason = server_msg
                            .go_away
                            .map(|ga| ga.reason)
                            .unwrap_or_else(|| "unknown".to_string());
                        error!("Server disconnecting: {}", reason);
                        return Err(GeminiError::new(GeminiErrorKind::ServerDisconnect(reason)));
                    }

                    // Extract text from response
                    if let Some(text) = server_msg.extract_text() {
                        let is_final = server_msg.is_turn_complete();

                        let chunk = StreamChunk {
                            content: Output::Text(text),
                            is_final,
                            finish_reason: if is_final {
                                Some(FinishReason::Stop)
                            } else {
                                None
                            },
                        };

                        return Ok(Some((chunk, (ws, is_final))));
                    }

                    // If no text but turn complete, we're done
                    if server_msg.is_turn_complete() {
                        return Ok(None);
                    }
                } else if let Message::Close(_) = msg {
                    warn!("WebSocket closed during streaming");
                    return Ok(None);
                }
            }

            Ok(None)
        });

        Ok(Box::pin(stream))
    }

    /// Close the WebSocket session gracefully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use botticelli_models::GeminiLiveClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = GeminiLiveClient::new()?;
    /// # let mut session = client.connect("models/gemini-2.0-flash-exp").await?;
    /// session.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "live_session_close", skip(self))]
    pub async fn close(mut self) -> GeminiResult<()> {
        debug!("Closing WebSocket session");

        self.ws_stream.close(None).await.map_err(|e| {
            error!("Error closing WebSocket: {}", e);
            GeminiError::new(GeminiErrorKind::WebSocketConnection(format!(
                "Close error: {}",
                e
            )))
        })?;

        info!("WebSocket session closed");
        Ok(())
    }

    /// Get the model name for this session.
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_api_endpoint() {
        assert!(LIVE_API_ENDPOINT.starts_with("wss://"));
        assert!(LIVE_API_ENDPOINT.contains("BidiGenerateContent"));
    }

    // Integration tests with #[cfg_attr(not(feature = "api"), ignore)]
    // will be in tests/gemini_live_basic_test.rs
}
