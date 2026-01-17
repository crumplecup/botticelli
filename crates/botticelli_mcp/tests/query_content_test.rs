//! Tests for query_content tool.

mod helpers;

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_without_database() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing query_content without database");

    use botticelli_mcp::{BotticelliServer, QueryContentParams};
    use rmcp::handler::server::wrapper::Parameters;

    let server = BotticelliServer::builder().build();
    let params = QueryContentParams {
        table: "content".to_string(),
        limit: 10,
    };

    let result = server.query_content(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Call completed");

    // Should fail when database is not configured
    assert!(
        result.is_err(),
        "Should fail when database operations not configured"
    );

    tracing::info!("Without database test passed");
    Ok(())
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_params_default_limit() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing QueryContentParams default limit");

    use botticelli_mcp::QueryContentParams;
    use serde_json::json;

    let json_value = json!({
        "table": "users"
    });

    let params: QueryContentParams = serde_json::from_value(json_value)?;
    tracing::debug!(table = %params.table, limit = params.limit, "Deserialized params");

    assert_eq!(params.table, "users");
    assert_eq!(params.limit, 10, "Should use default limit of 10");

    tracing::info!("Default limit test passed");
    Ok(())
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_params_custom_limit() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing QueryContentParams custom limit");

    use botticelli_mcp::QueryContentParams;
    use serde_json::json;

    let json_value = json!({
        "table": "users",
        "limit": 50
    });

    let params: QueryContentParams = serde_json::from_value(json_value)?;
    tracing::debug!(table = %params.table, limit = params.limit, "Deserialized params");

    assert_eq!(params.table, "users");
    assert_eq!(params.limit, 50);

    tracing::info!("Custom limit test passed");
    Ok(())
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_result_creation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing QueryContentResult creation");

    use botticelli_mcp::QueryContentResult;
    use serde_json::json;

    let rows = vec![
        json!({"id": 1, "name": "Alice"}),
        json!({"id": 2, "name": "Bob"}),
    ];

    let result = QueryContentResult::new("users".to_string(), 10, rows);
    tracing::debug!(
        status = %result.status,
        table = %result.table,
        count = result.count,
        "Created result"
    );

    assert_eq!(result.status, "success");
    assert_eq!(result.table, "users");
    assert_eq!(result.count, 2);
    assert_eq!(result.limit, 10);
    assert_eq!(result.rows.len(), 2);

    tracing::info!("Result creation test passed");
    Ok(())
}

