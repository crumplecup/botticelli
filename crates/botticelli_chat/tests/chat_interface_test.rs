//! Tests for ChatInterface trait and mock implementation.

use botticelli_chat::{ChatInterface, Message, Response, UserInput};
use botticelli_error::{ChatError, ChatResult};
use std::collections::VecDeque;

/// Mock chat interface for testing.
#[derive(Debug)]
struct MockChatInterface {
    /// Messages sent to the interface.
    sent_messages: Vec<Message>,
    /// Responses sent to the interface.
    sent_responses: Vec<Response>,
    /// Queued user inputs to return.
    input_queue: VecDeque<UserInput>,
}

impl MockChatInterface {
    fn new() -> Self {
        Self {
            sent_messages: Vec::new(),
            sent_responses: Vec::new(),
            input_queue: VecDeque::new(),
        }
    }

    fn queue_input(&mut self, input: UserInput) {
        self.input_queue.push_back(input);
    }

    fn sent_messages(&self) -> &[Message] {
        &self.sent_messages
    }

    fn sent_responses(&self) -> &[Response] {
        &self.sent_responses
    }
}

impl ChatInterface for MockChatInterface {
    fn send_message(&mut self, message: Message) -> ChatResult<()> {
        self.sent_messages.push(message);
        Ok(())
    }

    fn receive_input(&mut self) -> ChatResult<UserInput> {
        self.input_queue
            .pop_front()
            .ok_or_else(|| ChatError::invalid_input("No input queued"))
    }

    fn send_response(&mut self, response: Response) -> ChatResult<()> {
        self.sent_responses.push(response);
        Ok(())
    }
}

#[test]
fn test_mock_send_message() {
    let mut mock = MockChatInterface::new();
    let msg = Message::user("test message");

    mock.send_message(msg.clone()).unwrap();

    assert_eq!(mock.sent_messages().len(), 1);
    assert_eq!(mock.sent_messages()[0].content(), "test message");
}

#[test]
fn test_mock_receive_input() {
    let mut mock = MockChatInterface::new();
    mock.queue_input(UserInput::text("hello"));

    let input = mock.receive_input().unwrap();

    assert!(input.is_text());
    assert_eq!(input.as_text(), Some("hello"));
}

#[test]
fn test_mock_send_response() {
    let mut mock = MockChatInterface::new();
    let response = Response::success("operation complete");

    mock.send_response(response.clone()).unwrap();

    assert_eq!(mock.sent_responses().len(), 1);
    assert!(mock.sent_responses()[0].is_success());
}

#[test]
fn test_show_history() {
    let mut mock = MockChatInterface::new();
    let msg1 = Message::user("first");
    let msg2 = Message::assistant("second");

    mock.show_history(&[msg1, msg2]).unwrap();

    assert_eq!(mock.sent_messages().len(), 2);
}

#[test]
fn test_clear_default_impl() {
    let mut mock = MockChatInterface::new();

    // Default clear() is a no-op, should not error
    mock.clear().unwrap();
}
