//! Chat interface trait definition.

use botticelli_error::ChatResult;

use crate::{ Message, Response, UserInput};

/// Trait for implementing chat interfaces across different platforms.
///
/// Implementors must add `#[instrument]` to all methods for observability.
/// This is critical for debugging, performance monitoring, and error tracking.
///
/// # Examples
///
/// ```rust,ignore
/// use botticelli_chat::{ChatInterface, Message, Response, UserInput};
/// use tracing::instrument;
///
/// struct TerminalChat;
///
/// impl ChatInterface for TerminalChat {
///     #[instrument(skip(self))]
///     fn send_message(&mut self, message: Message) -> ChatResult<()> {
///         println!("{}: {}", message_type(&message), message.content());
///         Ok(())
///     }
///
///     #[instrument(skip(self))]
///     fn receive_input(&mut self) -> ChatResult<UserInput> {
///         // Read from stdin...
///         Ok(UserInput::text("example"))
///     }
///
///     #[instrument(skip(self))]
///     fn send_response(&mut self, response: Response) -> ChatResult<()> {
///         // Display response...
///         Ok(())
///     }
/// }
/// ```
pub trait ChatInterface {
    /// Send a message to the chat interface.
    ///
    /// Implementors must add `#[instrument(skip(self))]` for tracing.
    ///
    /// # Errors
    ///
    /// Returns error if message cannot be sent.
    fn send_message(&mut self, message: Message) -> ChatResult<()>;

    /// Receive input from the user.
    ///
    /// Implementors must add `#[instrument(skip(self))]` for tracing.
    ///
    /// # Errors
    ///
    /// Returns error if input cannot be received.
    fn receive_input(&mut self) -> ChatResult<UserInput>;

    /// Send a response to the user.
    ///
    /// Implementors must add `#[instrument(skip(self))]` for tracing.
    ///
    /// # Errors
    ///
    /// Returns error if response cannot be sent.
    fn send_response(&mut self, response: Response) -> ChatResult<()>;

    /// Display conversation history.
    ///
    /// Implementors must add `#[instrument(skip(self, history))]` for tracing.
    ///
    /// # Errors
    ///
    /// Returns error if history cannot be displayed.
    fn show_history(&mut self, history: &[Message]) -> ChatResult<()> {
        for message in history {
            self.send_message(message.clone())?;
        }
        Ok(())
    }

    /// Clear the chat display.
    ///
    /// Implementors must add `#[instrument(skip(self))]` for tracing.
    ///
    /// # Errors
    ///
    /// Returns error if display cannot be cleared.
    fn clear(&mut self) -> ChatResult<()> {
        // Default: no-op
        Ok(())
    }
}
