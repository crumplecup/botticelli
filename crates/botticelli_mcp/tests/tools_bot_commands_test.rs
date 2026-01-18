//! Tests for bot commands.

use botticelli_mcp::tools::{BotCommandRequest, BotCommandResponse};
use serde_json::json;

#[test]
fn test_bot_command_request_serialization() {
    let request = BotCommandRequest {
        command: "server.get_stats".to_string(),
        args: json!({"guild_id": "123456"}),
    };

    let json = serde_json::to_string(&request).expect("Serialize");
    assert!(json.contains("server.get_stats"));
    assert!(json.contains("guild_id"));
}

#[test]
fn test_bot_command_response_serialization() {
    let response = BotCommandResponse {
        command: "server.get_stats".to_string(),
        data: json!({"member_count": 100}),
        success: true,
        error: None,
    };

    let json = serde_json::to_string(&response).expect("Serialize");
    assert!(json.contains("server.get_stats"));
    assert!(json.contains("member_count"));
}
