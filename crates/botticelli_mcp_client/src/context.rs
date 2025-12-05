//! Context management for MCP client conversations.

use crate::{McpClientResult, Message, MessageRole};
use std::collections::VecDeque;

/// Maximum conversation history to maintain.
const MAX_HISTORY: usize = 50;

/// Context manager for MCP client conversations.
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// Conversation history.
    history: VecDeque<Message>,
    /// Maximum history size.
    max_history: usize,
    /// System prompt (if any).
    system_prompt: Option<String>,
}

impl ContextManager {
    /// Creates a new context manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            max_history: MAX_HISTORY,
            system_prompt: None,
        }
    }

    /// Creates a new context manager with a system prompt.
    #[must_use]
    pub fn with_system_prompt(system_prompt: String) -> Self {
        Self {
            history: VecDeque::new(),
            max_history: MAX_HISTORY,
            system_prompt: Some(system_prompt),
        }
    }

    /// Sets the maximum history size.
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max;
        self.truncate_history();
    }

    /// Adds a user message to history.
    pub fn add_user_message(&mut self, content: String) -> McpClientResult<()> {
        let message = Message {
            role: MessageRole::User,
            content,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        };

        self.history.push_back(message);
        self.truncate_history();
        Ok(())
    }

    /// Adds an assistant message to history.
    pub fn add_assistant_message(&mut self, content: String) -> McpClientResult<()> {
        let message = Message {
            role: MessageRole::Assistant,
            content,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        };

        self.history.push_back(message);
        self.truncate_history();
        Ok(())
    }

    /// Gets the full conversation history.
    #[must_use]
    pub fn history(&self) -> &VecDeque<Message> {
        &self.history
    }

    /// Gets messages formatted for LLM request.
    #[must_use]
    pub fn get_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add system prompt if present
        if let Some(prompt) = &self.system_prompt {
            messages.push(Message {
                role: MessageRole::System,
                content: prompt.clone(),
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
            });
        }

        // Add conversation history
        messages.extend(self.history.iter().cloned());

        messages
    }

    /// Clears conversation history.
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Truncates history to max size.
    fn truncate_history(&mut self) {
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Summarizes old messages (placeholder for future implementation).
    pub fn summarize_old_messages(&mut self) -> McpClientResult<()> {
        // TODO: Use LLM to summarize old messages when history gets too long
        Ok(())
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
