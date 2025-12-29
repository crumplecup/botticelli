//! Tests for echo tool.
//!
//! Tests use the actual rmcp API, not mocked interfaces.

use botticelli_mcp::{BotticelliServer, EchoParams, EchoResult};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_echo_basic_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: "Hello, MCP!".to_string(),
    };
    
    let result = server.echo(Parameters(params))
        .await
        .expect("Echo should succeed");
    
    assert_eq!(result.0.echo, "Hello, MCP!");
    assert!(!result.0.timestamp.is_empty());
}

#[tokio::test]
async fn test_echo_empty_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: String::new(),
    };
    
    let result = server.echo(Parameters(params))
        .await
        .expect("Echo should succeed even with empty message");
    
    assert_eq!(result.0.echo, "");
}

#[tokio::test]
async fn test_echo_unicode_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: "Hello 世界 🌍".to_string(),
    };
    
    let result = server.echo(Parameters(params))
        .await
        .expect("Echo should handle unicode");
    
    assert_eq!(result.0.echo, "Hello 世界 🌍");
}
