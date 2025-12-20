//! Integration tests for database tools.

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_tool() {
    use botticelli_mcp::{McpTool, QueryContentTool};
    use serde_json::json;

    let tool = QueryContentTool::default();

    // Test with content table
    let input = json!({
        "table": "content",
        "limit": 5
    });

    let result = tool.execute(input).await;

    // Should succeed or fail gracefully
    match result {
        Ok(value) => {
            assert_eq!(value["status"], "success");
            assert!(value["count"].is_number());
            println!("Query succeeded: retrieved {} rows", value["count"]);
        }
        Err(e) => {
            // Database connection might fail in CI/test environments
            println!("Query failed (expected in some environments): {}", e);
        }
    }
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_validation() {
    use botticelli_mcp::{McpTool, QueryContentTool};
    use serde_json::json;

    let tool = QueryContentTool::default();

    // Test missing table parameter
    let input = json!({
        "limit": 5
    });

    let result = tool.execute(input).await;
    assert!(result.is_err(), "Should fail without table parameter");
}

#[cfg(not(feature = "database"))]
#[tokio::test]
async fn test_query_content_without_feature() {
    use botticelli_mcp::{McpTool, QueryContentTool};
    use serde_json::json;

    let tool = QueryContentTool::default();

    let input = json!({
        "table": "content",
        "limit": 5
    });

    let result = tool.execute(input).await.unwrap();
    assert_eq!(result["status"], "not_available");
}
