//! Chat screen — scrollable message history with a text input buffer.
//!
//! The screen is pure: it never calls the LLM directly. On Enter it emits
//! [`BotTransition::ChatSend`]; the controller performs the async LLM call
//! and delivers the reply via [`BotScreen::on_chat_response`].

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::{debug, info, instrument};

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

/// Sender role in a chat conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    /// Human user.
    User,
    /// LLM assistant.
    Assistant,
}

impl ChatRole {
    fn prefix(self) -> &'static str {
        match self {
            ChatRole::User => "You: ",
            ChatRole::Assistant => "Bot: ",
        }
    }
}

/// A single message in the chat history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// Who sent this message.
    pub role: ChatRole,
    /// Message text.
    pub content: String,
}

/// Load and reply state of the LLM driver behind this chat screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    /// Driver is being loaded in the background.
    Loading,
    /// Driver is ready and waiting for input.
    Ready,
    /// The model is emitting chain-of-thought tokens before the answer.
    Thinking,
    /// A reply is being generated; input is blocked.
    Replying,
    /// Driver failed to load; message describes the error.
    Failed(String),
}

/// Chat screen — shows message history and an input line.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ChatScreen {
    /// All messages in the conversation so far.
    messages: Vec<ChatMessage>,
    /// Current text in the input buffer.
    input: String,
    /// `true` while waiting for an LLM response.
    waiting: bool,
    /// Load state of the LLM driver.
    model_status: ModelStatus,
    /// Chain-of-thought tokens accumulating before the answer arrives.
    #[getter(skip)]
    in_progress_thinking: String,
    /// Answer tokens accumulating during streaming.
    #[getter(skip)]
    in_progress_response: String,
}

impl ChatScreen {
    /// Create an empty chat screen, starting in the loading state.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            waiting: false,
            model_status: ModelStatus::Loading,
            in_progress_thinking: String::new(),
            in_progress_response: String::new(),
        }
    }

    fn message_node(msg: &ChatMessage) -> TuiNode {
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(format!("  {}{}", msg.role.prefix(), msg.content)),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        }
    }

    fn input_node(&self) -> TuiNode {
        let cursor = if self.waiting { "⏳" } else { "▶" };
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(format!("  {} {}", cursor, self.input)),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        }
    }
}

impl Default for ChatScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BotScreen for ChatScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        let header = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: Some(BlockJson {
                    title: Some("Chat".to_string()),
                    borders: BordersJson::All,
                    border_type: None,
                    style: None,
                    border_style: None,
                    padding: None,
                }),
            }),
        };

        let filler = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(String::new()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        };

        let help = TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain("  Enter=send  Esc=bots  Ctrl+C=quit".to_string()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: None,
            }),
        };

        let status_text = match &self.model_status {
            ModelStatus::Loading => Some("  ⏳ Loading model…".to_string()),
            ModelStatus::Thinking => Some("  💭 Thinking…".to_string()),
            ModelStatus::Replying => Some("  ⏳ Replying…".to_string()),
            ModelStatus::Failed(msg) => Some(format!("  ✗ Model load failed: {msg}")),
            ModelStatus::Ready => None,
        };

        // In-progress thinking preview: full accumulated reasoning text.
        // wrap: true on the paragraph node handles line breaking; the Min constraint
        // gives it room to grow without a hardcoded character limit.
        let thinking_preview = if !self.in_progress_thinking.is_empty() {
            Some(format!("  💭 {}", self.in_progress_thinking))
        } else {
            None
        };

        // In-progress response preview: growing answer text.
        let response_preview = if !self.in_progress_response.is_empty() {
            Some(format!("  Bot: {}", self.in_progress_response))
        } else {
            None
        };

        // One constraint per child:
        //   header + [status] + N messages + [thinking_preview] + [response_preview]
        //   + filler + input + help
        let n = self.messages.len();
        let extra_status = usize::from(status_text.is_some());
        let extra_thinking = usize::from(thinking_preview.is_some());
        let extra_response = usize::from(response_preview.is_some());
        let total = n + 4 + extra_status + extra_thinking + extra_response;
        let mut constraints = Vec::with_capacity(total);
        constraints.push(ConstraintJson::Length { value: 3 });
        if status_text.is_some() {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        for _ in 0..n {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        if thinking_preview.is_some() {
            constraints.push(ConstraintJson::Min { value: 3 });
        }
        if response_preview.is_some() {
            constraints.push(ConstraintJson::Min { value: 1 });
        }
        constraints.push(ConstraintJson::Fill { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });

        let mut children = Vec::with_capacity(total);
        children.push(header);
        if let Some(text) = status_text {
            children.push(TuiNode::Widget {
                widget: Box::new(WidgetJson::Paragraph {
                    text: ParagraphText::Plain(text),
                    style: None,
                    wrap: true,
                    scroll: None,
                    alignment: None,
                    block: None,
                }),
            });
        }
        for msg in &self.messages {
            children.push(Self::message_node(msg));
        }
        if let Some(preview) = thinking_preview {
            children.push(TuiNode::Widget {
                widget: Box::new(WidgetJson::Paragraph {
                    text: ParagraphText::Plain(preview),
                    style: None,
                    wrap: true,
                    scroll: None,
                    alignment: None,
                    block: None,
                }),
            });
        }
        if let Some(preview) = response_preview {
            children.push(TuiNode::Widget {
                widget: Box::new(WidgetJson::Paragraph {
                    text: ParagraphText::Plain(preview),
                    style: None,
                    wrap: true,
                    scroll: None,
                    alignment: None,
                    block: None,
                }),
            });
        }
        children.push(filler);
        children.push(self.input_node());
        children.push(help);

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints,
            children,
            margin: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        match key.code {
            // Navigation is always available, even while waiting for a reply.
            KeyCode::Esc => BotTransition::GoToBots,
            // All other input is blocked while a reply is in flight.
            _ if self.waiting => BotTransition::Stay,
            KeyCode::Enter if !self.input.is_empty() => {
                let content = std::mem::take(&mut self.input);
                self.messages.push(ChatMessage {
                    role: ChatRole::User,
                    content: content.clone(),
                });
                self.waiting = true;
                BotTransition::ChatSend { content }
            }
            KeyCode::Backspace => {
                self.input.pop();
                BotTransition::Stay
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                BotTransition::Stay
            }
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Chat"
    }
}

impl ChatScreen {
    /// Deliver a completed LLM response (non-streaming path, or convenience wrapper in tests).
    pub fn on_chat_response(&mut self, response: String) {
        self.on_content_chunk(response);
        self.on_stream_done();
    }

    /// Append a thinking/reasoning chunk from the live stream.
    #[instrument(skip(self), fields(text_len = text.len()))]
    pub fn on_thinking_chunk(&mut self, text: String) {
        debug!(
            total_thinking_len = self.in_progress_thinking.len() + text.len(),
            "Appending thinking chunk"
        );
        self.in_progress_thinking.push_str(&text);
        self.model_status = ModelStatus::Thinking;
    }

    /// Append an answer content chunk from the live stream.
    #[instrument(skip(self), fields(text_len = text.len()))]
    pub fn on_content_chunk(&mut self, text: String) {
        debug!(
            total_response_len = self.in_progress_response.len() + text.len(),
            prior_status = ?self.model_status,
            "Appending content chunk"
        );
        self.in_progress_response.push_str(&text);
        if self.model_status == ModelStatus::Thinking {
            // Transition: thinking phase ended, answer is starting.
            self.model_status = ModelStatus::Replying;
        }
    }

    /// Called when the stream is exhausted (final sentinel received).
    ///
    /// Moves accumulated in-progress text into the message history and resets
    /// streaming state.
    #[instrument(skip(self))]
    pub fn on_stream_done(&mut self) {
        let content = std::mem::take(&mut self.in_progress_response);
        let thinking_len = self.in_progress_thinking.len();
        self.in_progress_thinking.clear();
        info!(
            content_len = content.len(),
            thinking_len,
            message_count = self.messages.len() + if content.is_empty() { 0 } else { 1 },
            "Stream done — committing response to message history"
        );
        if !content.is_empty() {
            self.messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content,
            });
        } else {
            debug!("Empty content on stream done — no message pushed");
        }
        self.waiting = false;
        self.model_status = ModelStatus::Ready;
    }

    /// Signal that a generate call is in flight.
    #[instrument(skip(self))]
    pub fn on_chat_replying(&mut self) {
        info!("Chat replying started — clearing in-progress buffers");
        self.model_status = ModelStatus::Replying;
        self.in_progress_thinking.clear();
        self.in_progress_response.clear();
    }

    /// Signal that the LLM driver finished loading successfully.
    #[instrument(skip(self))]
    pub fn on_model_ready(&mut self) {
        info!("Model ready");
        self.model_status = ModelStatus::Ready;
    }

    /// Signal that the LLM driver failed to load.
    #[instrument(skip(self))]
    pub fn on_model_failed(&mut self, msg: String) {
        tracing::error!(message = %msg, "Model failed to load");
        self.model_status = ModelStatus::Failed(msg);
    }
}
