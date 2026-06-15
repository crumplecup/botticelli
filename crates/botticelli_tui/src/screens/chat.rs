//! Chat screen — scrollable message history with a text input buffer.
//!
//! The screen is pure: it never calls the LLM directly. On Enter it emits
//! [`BotTransition::ChatSend`]; the controller performs the async LLM call
//! and delivers the reply via [`BotScreen::on_chat_response`].

use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::instrument;

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
}

impl ChatScreen {
    /// Create an empty chat screen, starting in the loading state.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            waiting: false,
            model_status: ModelStatus::Loading,
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
            ModelStatus::Replying => Some("  ⏳ Replying…".to_string()),
            ModelStatus::Failed(msg) => Some(format!("  ✗ Model load failed: {msg}")),
            ModelStatus::Ready => None,
        };

        // One constraint per child:  header + [status] + N messages + filler + input + help
        let n = self.messages.len();
        let extra = if status_text.is_some() { 1 } else { 0 };
        let mut constraints = Vec::with_capacity(n + 4 + extra);
        constraints.push(ConstraintJson::Length { value: 3 });
        if status_text.is_some() {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        for _ in 0..n {
            constraints.push(ConstraintJson::Length { value: 1 });
        }
        constraints.push(ConstraintJson::Fill { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });
        constraints.push(ConstraintJson::Length { value: 1 });

        let mut children = Vec::with_capacity(n + 4 + extra);
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
    /// Deliver a completed LLM response.
    pub fn on_chat_response(&mut self, response: String) {
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: response,
        });
        self.waiting = false;
        self.model_status = ModelStatus::Ready;
    }

    /// Signal that a generate call is in flight.
    pub fn on_chat_replying(&mut self) {
        self.model_status = ModelStatus::Replying;
    }

    /// Signal that the LLM driver finished loading successfully.
    pub fn on_model_ready(&mut self) {
        self.model_status = ModelStatus::Ready;
    }

    /// Signal that the LLM driver failed to load.
    pub fn on_model_failed(&mut self, msg: String) {
        self.model_status = ModelStatus::Failed(msg);
    }
}
