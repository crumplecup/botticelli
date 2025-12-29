//! Tests for query_content tool.

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_without_database() {
    use botticelli_mcp::{BotticelliServer, QueryContentParams};
    use rmcp::handler::server::wrapper::Parameters;

    let server = BotticelliServer::builder().build();
    let params = QueryContentParams {
        table: "content".to_string(),
        limit: 10,
    };

    let result = server.query_content(Parameters(params)).await;

    // Should fail when database is not configured
    assert!(
        result.is_err(),
        "Should fail when database operations not configured"
    );
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_params_default_limit() {
    use botticelli_mcp::QueryContentParams;
    use serde_json::json;

    let json_value = json!({
        "table": "users"
    });

    let params: QueryContentParams =
        serde_json::from_value(json_value).expect("Should deserialize with default limit");

    assert_eq!(params.table, "users");
    assert_eq!(params.limit, 10, "Should use default limit of 10");
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_params_custom_limit() {
    use botticelli_mcp::QueryContentParams;
    use serde_json::json;

    let json_value = json!({
        "table": "users",
        "limit": 50
    });

    let params: QueryContentParams =
        serde_json::from_value(json_value).expect("Should deserialize with custom limit");

    assert_eq!(params.table, "users");
    assert_eq!(params.limit, 50);
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_result_creation() {
    use botticelli_mcp::QueryContentResult;
    use serde_json::json;

    let rows = vec![
        json!({"id": 1, "name": "Alice"}),
        json!({"id": 2, "name": "Bob"}),
    ];

    let result = QueryContentResult::new("users".to_string(), 10, rows);

    assert_eq!(result.status, "success");
    assert_eq!(result.table, "users");
    assert_eq!(result.count, 2);
    assert_eq!(result.limit, 10);
    assert_eq!(result.rows.len(), 2);
}
