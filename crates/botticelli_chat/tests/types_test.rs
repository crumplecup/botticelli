//! Tests for core types (Message, Response, UserInput).

use botticelli_chat::{Message, Response, UserInput};

#[test]
fn test_message_user() {
    let msg = Message::user("hello");
    assert!(msg.is_user());
    assert!(!msg.is_assistant());
    assert!(!msg.is_system());
    assert_eq!(msg.content(), "hello");
}

#[test]
fn test_message_assistant() {
    let msg = Message::assistant("response");
    assert!(msg.is_assistant());
    assert!(!msg.is_user());
    assert!(!msg.is_system());
    assert_eq!(msg.content(), "response");
}

#[test]
fn test_message_system() {
    let msg = Message::system("info");
    assert!(msg.is_system());
    assert!(!msg.is_user());
    assert!(!msg.is_assistant());
    assert_eq!(msg.content(), "info");
}

#[test]
fn test_response_text() {
    let resp = Response::text("hello");
    assert!(!resp.is_error());
    assert!(!resp.is_success());
}

#[test]
fn test_response_success() {
    let resp = Response::success("done");
    assert!(resp.is_success());
    assert!(!resp.is_error());
}

#[test]
fn test_response_success_with_data() {
    let resp = Response::success_with_data("done", "data");
    assert!(resp.is_success());
}

#[test]
fn test_response_error() {
    let resp = Response::error("failed");
    assert!(resp.is_error());
    assert!(!resp.is_success());
}

#[test]
fn test_response_error_with_details() {
    let resp = Response::error_with_details("failed", "details");
    assert!(resp.is_error());
}

#[test]
fn test_response_confirmation() {
    let resp = Response::confirmation("proceed?", true);
    assert!(resp.is_confirmation());
}

#[test]
fn test_response_info() {
    let resp = Response::info("information");
    assert!(!resp.is_error());
}

#[test]
fn test_response_warning() {
    let resp = Response::warning("be careful");
    assert!(!resp.is_error());
}

#[test]
fn test_user_input_text() {
    let input = UserInput::text("command");
    assert!(input.is_text());
    assert_eq!(input.as_text(), Some("command"));
}

#[test]
fn test_user_input_confirmation() {
    let input = UserInput::confirmation(true);
    assert!(input.is_confirmation());
    assert_eq!(input.as_confirmation(), Some(true));
}

#[test]
fn test_user_input_file_path() {
    let input = UserInput::file_path("test.txt");
    assert!(input.is_file_path());
    assert_eq!(input.as_file_path(), Some("test.txt"));
}

#[test]
fn test_user_input_exit() {
    let input = UserInput::exit();
    assert!(input.is_exit());
}
