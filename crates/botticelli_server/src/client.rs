use crate::{
    convert, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ServerConfig,
};
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{ServerError, ServerErrorKind};
use botticelli_interface::{BotticelliDriver, StreamChunk, Streaming};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tracing::instrument;

/// Type alias for streaming responses
type ChatCompletionStream =
    Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ServerError>> + Send>>;

/// Client for interacting with local inference server
#[derive(Debug, Clone)]
pub struct ServerClient {
    config: ServerConfig,
    client: reqwest::Client,
}

impl ServerClient {
    /// Create a new server client
    #[instrument(skip(config), fields(base_url = %config.base_url, model = %config.model))]
    pub fn new(config: ServerConfig) -> Self {
        tracing::debug!("Creating server client");
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Check if the server is running and responding
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<(), ServerError> {
        let url = format!("{}/health", self.config.base_url);
        tracing::debug!("Checking server health at {}", url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            tracing::error!("Health check failed: {}", e);
            ServerError::new(ServerErrorKind::Http(format!("Health check failed: {}", e)))
        })?;

        if response.status().is_success() {
            tracing::debug!("Server is healthy");
            Ok(())
        } else {
            let status = response.status();
            tracing::error!("Server health check returned error: {}", status);
            Err(ServerError::new(ServerErrorKind::Api(format!(
                "Server returned: {}",
                status
            ))))
        }
    }

    /// Send a chat completion request
    #[instrument(skip(self, request), fields(model = %request.model))]
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ServerError> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        tracing::debug!("Sending chat completion request to {}", url);

        let mut req = self
            .client
            .post(&url)
            .json(&request)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| {
            tracing::error!("Request failed: {}", e);
            ServerError::new(ServerErrorKind::Http(format!("Request failed: {}", e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("Server returned error: {}", status);
            return Err(ServerError::new(ServerErrorKind::Api(format!(
                "Server returned: {}",
                status
            ))));
        }

        let result = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse response: {}", e);
            ServerError::new(ServerErrorKind::Deserialization(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        tracing::debug!("Chat completion successful");
        Ok(result)
    }

    /// Send a streaming chat completion request
    #[instrument(skip(self, request), fields(model = %request.model))]
    pub async fn chat_completion_stream(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<ChatCompletionStream, ServerError> {
        request.stream = Some(true);

        let url = format!("{}/v1/chat/completions", self.config.base_url);
        tracing::debug!("Sending streaming chat completion request to {}", url);

        let mut req = self
            .client
            .post(&url)
            .json(&request)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| {
            tracing::error!("Request failed: {}", e);
            ServerError::new(ServerErrorKind::Http(format!("Request failed: {}", e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("Server returned error: {}", status);
            return Err(ServerError::new(ServerErrorKind::Api(format!(
                "Server returned: {}",
                status
            ))));
        }

        tracing::debug!("Streaming request successful, parsing SSE stream");
        Ok(Box::pin(parse_sse_stream(response)))
    }
}

/// Parse Server-Sent Events stream into chat completion chunks
fn parse_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<ChatCompletionChunk, ServerError>> {
    use futures::StreamExt;

    response.bytes_stream().scan(String::new(), |buffer, bytes_result| {
        let bytes = match bytes_result {
            Ok(b) => b,
            Err(e) => {
                return futures::future::ready(Some(Err(ServerError::new(
                    ServerErrorKind::Stream(format!("Stream error: {}", e)),
                ))));
            }
        };

        let text = match std::str::from_utf8(&bytes) {
            Ok(t) => t,
            Err(e) => {
                return futures::future::ready(Some(Err(ServerError::new(
                    ServerErrorKind::Stream(format!("Invalid UTF-8: {}", e)),
                ))));
            }
        };

        buffer.push_str(text);

        // Process complete SSE events
        if let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            buffer.drain(..pos + 2);

            // Parse SSE event
            if let Some(data) = event.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return futures::future::ready(None);
                }

                match serde_json::from_str::<ChatCompletionChunk>(data) {
                    Ok(chunk) => return futures::future::ready(Some(Ok(chunk))),
                    Err(e) => {
                        return futures::future::ready(Some(Err(ServerError::new(
                            ServerErrorKind::Deserialization(format!(
                                "Failed to parse chunk: {}",
                                e
                            )),
                        ))));
                    }
                }
            }
        }

        futures::future::ready(None)
    })
    .filter_map(|item| futures::future::ready(Some(item)))
}

#[async_trait::async_trait]
impl BotticelliDriver for ServerClient {
    #[instrument(skip(self, req))]
    async fn generate(
        &self,
        req: &GenerateRequest,
    ) -> botticelli_error::BotticelliResult<GenerateResponse> {
        let chat_request = convert::to_chat_request(req.clone(), self.config.model.clone())
            .map_err(|e| botticelli_error::BotticelliError::new(e.into()))?;

        let response = self
            .chat_completion(chat_request)
            .await
            .map_err(|e| botticelli_error::BotticelliError::new(e.into()))?;

        convert::from_chat_response(response)
            .map_err(|e| botticelli_error::BotticelliError::new(e.into()))
    }

    fn provider_name(&self) -> &'static str {
        "local-server"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        // Local server has no rate limits - return unlimited config
        static UNLIMITED: botticelli_rate_limit::RateLimitConfig =
            botticelli_rate_limit::RateLimitConfig {
                requests_per_minute: u64::MAX,
                tokens_per_minute: u64::MAX,
                requests_per_day: u64::MAX,
                tokens_per_day: u64::MAX,
            };
        &UNLIMITED
    }
}

#[async_trait::async_trait]
impl Streaming for ServerClient {
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> botticelli_error::BotticelliResult<
        Pin<Box<dyn Stream<Item = botticelli_error::BotticelliResult<StreamChunk>> + Send>>,
    > {
        tracing::debug!("Starting stream generation");

        let mut chat_request = convert::to_chat_request(req.clone(), self.config.model.clone())
            .map_err(|e| botticelli_error::BotticelliError::new(e.into()))?;
        chat_request.stream = Some(true);

        let stream = self
            .chat_completion_stream(chat_request)
            .await
            .map_err(|e| botticelli_error::BotticelliError::new(e.into()))?;

        let converted_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| botticelli_error::BotticelliError::new(e.into()))
                .and_then(|chunk| {
                    convert::chunk_to_stream_chunk(chunk)
                        .map_err(|e| botticelli_error::BotticelliError::new(e.into()))
                })
        });

        Ok(Box::pin(converted_stream))
    }
}
