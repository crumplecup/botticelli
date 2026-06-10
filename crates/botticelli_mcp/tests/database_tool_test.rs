//! Tests for database query tools.

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_interface::DatabaseRegistryOperations;
use botticelli_mcp::{McpTool, QueryContentTool};
use serde_json::{Value, json};
use std::sync::Arc;

/// No-op database for unit testing tool logic without a real DB.
struct NoopDatabase;

#[async_trait]
impl DatabaseRegistryOperations for NoopDatabase {
    async fn execute_query(&self, _query: &str) -> BotticelliResult<Vec<Value>> {
        Ok(vec![])
    }

    async fn list_tables(&self) -> BotticelliResult<Vec<String>> {
        Ok(vec![])
    }

    async fn get_schema(&self, _table: &str) -> BotticelliResult<Value> {
        Ok(json!({}))
    }

    async fn query_content(
        &self,
        _table_name: &str,
        _status_filter: Option<&str>,
        _limit: i64,
    ) -> BotticelliResult<Vec<Value>> {
        Ok(vec![])
    }

    async fn create_table(
        &self,
        _table_name: &str,
        _template_source: &str,
        _narrative_file: Option<&str>,
        _description: Option<&str>,
    ) -> BotticelliResult<()> {
        Ok(())
    }

    async fn table_exists(&self, _table_name: &str) -> BotticelliResult<bool> {
        Ok(false)
    }
}

fn noop_tool() -> QueryContentTool {
    QueryContentTool::new(Arc::new(NoopDatabase))
}

#[tokio::test]
async fn test_query_content_tool_returns_success() {
    let tool = noop_tool();

    let result = tool
        .execute(json!({"table": "content", "limit": 5}))
        .await
        .expect("tool executed");

    assert_eq!(result["status"], "success");
    assert_eq!(result["count"], 0);
    assert!(result["rows"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_query_content_requires_table() {
    let tool = noop_tool();

    let result = tool.execute(json!({"limit": 5})).await;
    assert!(result.is_err(), "should fail without table parameter");
}

#[tokio::test]
async fn test_query_content_name_and_description() {
    let tool = noop_tool();
    assert_eq!(tool.name(), "query_content");
    assert!(!tool.description().is_empty());
}
