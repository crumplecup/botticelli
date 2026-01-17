//! Tests for conversation modeling types.

mod helpers;

use botticelli_core::{ToolCall, ToolResult};
use botticelli_mcp::{ConversationSession, ConversationTurn};

#[test]
fn test_conversation_session_creation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing conversation session creation");

    let session = ConversationSession::new("You are a helpful assistant");
    tracing::debug!(
        id = %session.id,
        system_prompt = %session.system_prompt,
        turn_count = session.turn_count(),
        "Created session"
    );

    assert!(!session.id.is_empty());
    assert_eq!(session.system_prompt, "You are a helpful assistant");
    assert_eq!(session.turn_count(), 0);
    assert!(session.is_active());
    assert_eq!(session.max_turns, 50);

    tracing::info!("Session creation test passed");
    Ok(())
}

#[test]
fn test_add_user_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing add user message");

    let mut session = ConversationSession::new("Test");

    session.add_turn(ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    });
    tracing::debug!(turn_count = session.turn_count(), "Added user message");

    assert_eq!(session.turn_count(), 1);
    assert!(session.is_active());

    tracing::info!("Add user message test passed");
    Ok(())
}

#[test]
fn test_add_assistant_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing add assistant message");

    let mut session = ConversationSession::new("Test");

    session.add_turn(ConversationTurn::AssistantMessage {
        content: "Hi there!".to_string(),
    });
    tracing::debug!(turn_count = session.turn_count(), "Added assistant message");

    assert_eq!(session.turn_count(), 1);

    tracing::info!("Add assistant message test passed");
    Ok(())
}

#[test]
fn test_add_tool_calls() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing add tool calls");

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
    tracing::debug!(turn_count = session.turn_count(), "Added tool calls");

    assert_eq!(session.turn_count(), 1);

    tracing::info!("Add tool calls test passed");
    Ok(())
}

#[test]
fn test_add_tool_results() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing add tool results");

    let mut session = ConversationSession::new("Test");

    let result = ToolResult::new(
        "call_123".to_string(),
        serde_json::json!({"result": "success"}),
        false,
    );

    session.add_turn(ConversationTurn::ToolResults {
        results: vec![result],
    });
    tracing::debug!(turn_count = session.turn_count(), "Added tool results");

    assert_eq!(session.turn_count(), 1);

    tracing::info!("Add tool results test passed");
    Ok(())
}

#[test]
fn test_max_turns_exceeded() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing max turns exceeded");

    let mut session = ConversationSession::new("Test");
    session.max_turns = 3;

    // Add 3 turns
    for i in 1..=3 {
        session.add_turn(ConversationTurn::UserMessage {
            content: "Test".to_string(),
            attachments: None,
        });
        tracing::debug!(turn = i, "Added turn");
    }

    tracing::debug!(
        turn_count = session.turn_count(),
        is_active = session.is_active(),
        "Reached max turns"
    );

    assert_eq!(session.turn_count(), 3);
    assert!(!session.is_active());

    tracing::info!("Max turns exceeded test passed");
    Ok(())
}

#[test]
fn test_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing conversation turn serialization");

    let turn = ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    };

    let json = serde_json::to_string(&turn)?;
    tracing::debug!(json_len = json.len(), "Serialized turn");

    let _deserialized: ConversationTurn = serde_json::from_str(&json)?;
    tracing::debug!("Deserialized turn");

    tracing::info!("Serialization test passed");
    Ok(())
}

