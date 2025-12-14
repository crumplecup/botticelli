//! Tests for conversation modeling types.

use botticelli_core::ToolCall;
use botticelli_mcp::{ConversationSession, ConversationTurn, ToolResult};

#[test]
fn test_conversation_session_creation() {
    let session = ConversationSession::new("You are a helpful assistant");

    assert!(!session.id.is_empty());
    assert_eq!(session.system_prompt, "You are a helpful assistant");
    assert_eq!(session.turn_count(), 0);
    assert!(session.is_active());
    assert_eq!(session.max_turns, 50);
}

#[test]
fn test_add_user_message() {
    let mut session = ConversationSession::new("Test");

    session.add_turn(ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    });

    assert_eq!(session.turn_count(), 1);
    assert!(session.is_active());
}

#[test]
fn test_add_assistant_message() {
    let mut session = ConversationSession::new("Test");

    session.add_turn(ConversationTurn::AssistantMessage {
        content: "Hi there!".to_string(),
    });

    assert_eq!(session.turn_count(), 1);
}

#[test]
fn test_add_tool_calls() {
    let mut session = ConversationSession::new("Test");

    let tool_call = ToolCall::new(
        "call_123".to_string(),
        "test_tool".to_string(),
        serde_json::json!({"arg": "value"}),
    );

    session.add_turn(ConversationTurn::AssistantToolCalls {
        calls: vec![tool_call],
        thinking: Some("Let me check that...".to_string()),
    });

    assert_eq!(session.turn_count(), 1);
}

#[test]
fn test_add_tool_results() {
    let mut session = ConversationSession::new("Test");

    let result = ToolResult {
        tool_call_id: "call_123".to_string(),
        output: serde_json::json!({"result": "success"}),
        is_error: false,
        error_message: None,
    };

    session.add_turn(ConversationTurn::ToolResults {
        results: vec![result],
    });

    assert_eq!(session.turn_count(), 1);
}

#[test]
fn test_max_turns_exceeded() {
    let mut session = ConversationSession::new("Test");
    session.max_turns = 3;

    // Add 3 turns
    for _ in 0..3 {
        session.add_turn(ConversationTurn::UserMessage {
            content: "Test".to_string(),
            attachments: None,
        });
    }

    assert_eq!(session.turn_count(), 3);
    assert!(!session.is_active());
}

#[test]
fn test_serialization() {
    let turn = ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    };

    let json = serde_json::to_string(&turn).expect("Should serialize");
    let _deserialized: ConversationTurn = serde_json::from_str(&json).expect("Should deserialize");
}
