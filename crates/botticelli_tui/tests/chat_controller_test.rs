#![recursion_limit = "256"]
//! Integration tests for the chat round-trip through [`BotController`].

use std::sync::Arc;

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Output, StopReason};
use botticelli_error::BotticelliResult;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use botticelli_tui::{BotController, BotScreenContext, BotTransition, ChatRole};

/// Minimal driver that echoes a fixed reply for every generate call.
struct EchoDriver {
    reply: String,
    rate_limits: RateLimitConfig,
}

impl EchoDriver {
    fn new(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
            rate_limits: RateLimitConfig::unlimited("echo"),
        }
    }
}

#[async_trait]
impl BotticelliDriver for EchoDriver {
    async fn generate(&self, _req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        let resp = GenerateResponse::builder()
            .outputs(vec![Output::Text(self.reply.clone())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .expect("valid response");
        Ok(resp)
    }

    fn provider_name(&self) -> &'static str {
        "echo"
    }

    fn model_name(&self) -> &str {
        "echo-1"
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}

#[tokio::test]
async fn chat_round_trip_hello_gets_response() {
    let driver = Arc::new(EchoDriver::new("hello back!"));
    let ctx = BotScreenContext::mock();
    let mut controller = BotController::new(ctx).with_driver(driver);

    controller
        .drive(BotTransition::ChatSend {
            content: "hello".to_string(),
        })
        .await
        .expect("drive should succeed");

    let messages = controller.chat().messages();
    let assistant_reply = messages
        .iter()
        .find(|m| m.role == ChatRole::Assistant)
        .expect("should have an assistant message");

    assert_eq!(assistant_reply.content, "hello back!");
}

#[tokio::test]
async fn chat_round_trip_model_status_returns_to_ready() {
    // After a reply arrives the status should go back to Ready, not stay Replying.
    use botticelli_tui::ModelStatus;

    let driver = Arc::new(EchoDriver::new("got it"));
    let ctx = BotScreenContext::mock();
    let mut controller = BotController::new(ctx).with_driver(driver);

    controller
        .drive(BotTransition::ChatSend {
            content: "ping".to_string(),
        })
        .await
        .expect("drive should succeed");

    assert_eq!(controller.chat().model_status(), &ModelStatus::Ready);
}

#[tokio::test]
async fn chat_history_survives_navigation() {
    // The assistant reply is added by the controller; the user message is added by
    // handle_key (screen responsibility, tested in chat_screen_test.rs).
    let driver = Arc::new(EchoDriver::new("reply"));
    let ctx = BotScreenContext::mock();
    let mut controller = BotController::new(ctx).with_driver(driver);

    controller
        .drive(BotTransition::ChatSend {
            content: "hi".to_string(),
        })
        .await
        .expect("drive should succeed");

    let count_before = controller.chat().messages().len();
    assert_eq!(count_before, 1); // assistant reply only

    // Navigate away and back.
    controller.drive(BotTransition::GoToBots).await.unwrap();
    controller.drive(BotTransition::GoToChat).await.unwrap();

    // History must still be there.
    assert_eq!(controller.chat().messages().len(), count_before);
}
