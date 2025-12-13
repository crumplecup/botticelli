//! Tests for conversation state.

use botticelli_chat::{ConversationState, Message};
use std::path::PathBuf;

#[test]
fn test_conversation_state_new() {
    let state = ConversationState::new();
    assert_eq!(state.history().len(), 0);
    assert!(state.current_narrative().is_none());
    assert!(state.current_bot().is_none());
}

#[test]
fn test_conversation_state_builder() {
    let state = ConversationState::new();
    assert_eq!(state.history().len(), 0);
}

#[test]
fn test_add_message() {
    let mut state = ConversationState::new();
    let msg = Message::user("test");

    state.add_message(msg);

    assert_eq!(state.history().len(), 1);
    assert_eq!(state.history()[0].content(), "test");
}

#[test]
fn test_clear_history() {
    let mut state = ConversationState::new();
    state.add_message(Message::user("test1"));
    state.add_message(Message::user("test2"));

    state.clear_history();

    assert_eq!(state.history().len(), 0);
}

#[test]
fn test_set_narrative() {
    let mut state = ConversationState::new();
    state.set_narrative("test.toml");

    assert_eq!(
        state.current_narrative(),
        Some(&PathBuf::from("test.toml"))
    );
}

#[test]
fn test_clear_narrative() {
    let mut state = ConversationState::new();
    state.set_narrative("test.toml");
    state.clear_narrative();

    assert!(state.current_narrative().is_none());
}

#[test]
fn test_set_bot() {
    let mut state = ConversationState::new();
    state.set_bot("bot123");

    assert_eq!(state.current_bot(), Some("bot123"));
}

#[test]
fn test_clear_bot() {
    let mut state = ConversationState::new();
    state.set_bot("bot123");
    state.clear_bot();

    assert!(state.current_bot().is_none());
}

#[test]
fn test_session_timing() {
    let state = ConversationState::new();

    let duration = state.session_duration();
    assert!(duration >= 0);

    let idle = state.time_since_last_activity();
    assert!(idle >= 0);
}

#[test]
fn test_last_activity_updates() {
    let mut state = ConversationState::new();
    let initial = state.last_activity();

    std::thread::sleep(std::time::Duration::from_millis(10));
    state.add_message(Message::user("test"));

    assert!(state.last_activity() > initial);
}
