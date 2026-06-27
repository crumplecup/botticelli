//! Integration tests for table references in narratives.
//!
//! Tests that narratives can query content tables via `BotStorageTableQueryRegistry`
//! and include the results in prompts. Uses an in-memory `RedbStorage` backend.

#![cfg(feature = "database")]

use botticelli_core::{Input, Output, TableFormat};
use botticelli_database::{BotStorageTableQueryRegistry, RedbStorage};
use botticelli_error::BotticelliResult;
use botticelli_interface::{BotStorage, ContentRecord};
use botticelli_narrative::{ActConfig, NarrativeExecutor, NarrativeMetadata, NarrativeProvider};
use std::sync::Arc;

/// Seed a content table with JSON rows for testing.
async fn seed_table(storage: &Arc<dyn BotStorage>, table_name: &str, rows: Vec<serde_json::Value>) {
    for (i, row) in rows.into_iter().enumerate() {
        let record = ContentRecord {
            id: format!("{}:{}", table_name, i),
            table_name: table_name.to_string(),
            content_json: row,
            created_at: chrono::Utc::now(),
        };
        storage.save_content(&record).await.expect("seed content");
    }
}

/// Test narrative provider that queries a table.
struct TableReferenceNarrative {
    metadata: NarrativeMetadata,
    act_names: Vec<String>,
    acts: Vec<(String, ActConfig)>,
}

impl TableReferenceNarrative {
    fn new(table_name: &str, format: TableFormat) -> BotticelliResult<Self> {
        let metadata = serde_json::from_str(
            r#"{
            "name": "table_reference_test",
            "description": "Test narrative with table references",
            "skip_content_generation": false
        }"#,
        )
        .expect("Failed to parse test metadata");

        let table_input = Input::Table {
            table_name: table_name.to_string(),
            columns: None,
            where_clause: None,
            limit: Some(5),
            offset: None,
            order_by: None,
            alias: Some("test_data".to_string()),
            format,
            sample: None,
            destructive_read: false,
            history_retention: Default::default(),
        };

        let act_config = ActConfig::new(
            vec![table_input],
            Some("gemini-2.0-flash-lite".to_string()),
            Some(0.7),
            Some(100),
            None,
            None,
        );

        let act_name = "query_table".to_string();
        Ok(Self {
            metadata,
            act_names: vec![act_name.clone()],
            acts: vec![(act_name, act_config)],
        })
    }
}

impl NarrativeProvider for TableReferenceNarrative {
    fn name(&self) -> &str {
        "table_reference_test"
    }

    fn metadata(&self) -> &NarrativeMetadata {
        &self.metadata
    }

    fn act_names(&self) -> &[String] {
        &self.act_names
    }

    fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
        self.acts
            .iter()
            .find(|(name, _)| name == act_name)
            .map(|(_, config)| config.clone())
    }
}

/// Mock driver that echoes the input text back for verification.
struct MockDriver;

#[async_trait::async_trait]
impl botticelli_interface::BotticelliDriver for MockDriver {
    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        use botticelli_rate_limit::RateLimitConfig;
        static RATE_LIMIT: std::sync::OnceLock<RateLimitConfig> = std::sync::OnceLock::new();
        RATE_LIMIT.get_or_init(|| RateLimitConfig {
            requests_per_minute: u64::MAX,
            tokens_per_minute: u64::MAX,
            requests_per_day: u64::MAX,
            tokens_per_day: u64::MAX,
        })
    }

    async fn generate(
        &self,
        request: &botticelli_core::GenerateRequest,
    ) -> BotticelliResult<botticelli_core::GenerateResponse> {
        let mut table_content = String::new();
        for message in request.messages() {
            for input in message.content() {
                if let Input::Text(text) = input {
                    table_content = text.clone();
                    break;
                }
            }
        }

        Ok(botticelli_core::GenerateResponse::builder()
            .outputs(vec![Output::Text(format!(
                "Received table data: {}",
                table_content
            ))])
            .build()
            .expect("Valid response"))
    }
}

#[tokio::test]
async fn test_table_reference_query() -> BotticelliResult<()> {
    let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::in_memory().expect("in-memory redb"));

    seed_table(
        &storage,
        "test_products",
        vec![
            serde_json::json!({"name": "Widget", "price": 9.99, "category": "Tools"}),
            serde_json::json!({"name": "Gadget", "price": 19.99, "category": "Electronics"}),
            serde_json::json!({"name": "Doohickey", "price": 4.99, "category": "Tools"}),
            serde_json::json!({"name": "Thingamajig", "price": 14.99, "category": "Home"}),
            serde_json::json!({"name": "Whatsit", "price": 7.99, "category": "Electronics"}),
        ],
    )
    .await;

    let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(&storage));
    let narrative = TableReferenceNarrative::new("test_products", TableFormat::Markdown)?;
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));

    let execution = executor.execute(&narrative).await?;
    assert_eq!(execution.act_executions().len(), 1);
    let act_exec = &execution.act_executions()[0];
    assert_eq!(act_exec.act_name(), "query_table");

    assert!(!execution.act_executions()[0].inputs().is_empty());

    match &execution.act_executions()[0].inputs()[0] {
        Input::Text(text) => {
            assert!(
                text.contains("Widget") || text.contains("Gadget"),
                "Expected product data in input, got: {}",
                text
            );
        }
        _ => panic!("Expected Text input after table processing"),
    }

    Ok(())
}

#[tokio::test]
async fn test_table_reference_with_filter() -> BotticelliResult<()> {
    let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::in_memory().expect("in-memory redb"));

    seed_table(
        &storage,
        "test_orders",
        vec![
            serde_json::json!({"customer": "Alice", "total": 100.0, "status": "completed"}),
            serde_json::json!({"customer": "Bob", "total": 150.0, "status": "pending"}),
            serde_json::json!({"customer": "Charlie", "total": 200.0, "status": "completed"}),
            serde_json::json!({"customer": "Diana", "total": 75.0, "status": "cancelled"}),
            serde_json::json!({"customer": "Eve", "total": 300.0, "status": "completed"}),
        ],
    )
    .await;

    let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(&storage));

    let metadata: NarrativeMetadata = serde_json::from_str(
        r#"{
        "name": "filtered_query_test",
        "description": "Test with WHERE clause filtering",
        "skip_content_generation": false
    }"#,
    )
    .unwrap();

    let table_input = Input::Table {
        table_name: "test_orders".to_string(),
        columns: Some(vec!["customer".to_string(), "total".to_string()]),
        where_clause: Some("status = 'completed'".to_string()),
        limit: Some(10),
        offset: None,
        order_by: Some("total DESC".to_string()),
        alias: Some("completed_orders".to_string()),
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: Default::default(),
    };

    let act_config = ActConfig::new(
        vec![table_input],
        Some("gemini-2.0-flash-lite".to_string()),
        Some(0.7),
        Some(100),
        None,
        None,
    );

    struct FilteredNarrative {
        metadata: NarrativeMetadata,
        act_names: Vec<String>,
        act_config: ActConfig,
    }

    impl NarrativeProvider for FilteredNarrative {
        fn name(&self) -> &str {
            "filtered_query_test"
        }

        fn metadata(&self) -> &NarrativeMetadata {
            &self.metadata
        }

        fn act_names(&self) -> &[String] {
            &self.act_names
        }

        fn get_act_config(&self, _act_name: &str) -> Option<ActConfig> {
            Some(self.act_config.clone())
        }
    }

    let narrative = FilteredNarrative {
        metadata,
        act_names: vec!["query_filtered".to_string()],
        act_config,
    };

    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));
    let execution = executor.execute(&narrative).await?;

    assert_eq!(execution.act_executions().len(), 1);
    let act_exec = &execution.act_executions()[0];

    match &act_exec.inputs()[0] {
        Input::Text(text) => {
            assert!(
                text.contains("Eve") || text.contains("300"),
                "Expected completed orders (Eve/300) in output: {}",
                text
            );
            assert!(
                !text.contains("Bob"),
                "Bob is pending, should be filtered out"
            );
            assert!(
                !text.contains("Diana"),
                "Diana is cancelled, should be filtered out"
            );
        }
        _ => panic!("Expected Text input after table processing"),
    }
    Ok(())
}

#[tokio::test]
async fn test_table_reference_format_csv() -> BotticelliResult<()> {
    let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::in_memory().expect("in-memory redb"));

    seed_table(
        &storage,
        "test_employees",
        vec![
            serde_json::json!({"name": "Alice", "department": "Engineering", "salary": 95000}),
            serde_json::json!({"name": "Bob", "department": "Marketing", "salary": 75000}),
            serde_json::json!({"name": "Charlie", "department": "Engineering", "salary": 105000}),
        ],
    )
    .await;

    let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(&storage));
    let narrative = TableReferenceNarrative::new("test_employees", TableFormat::Csv)?;
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));
    let execution = executor.execute(&narrative).await?;

    assert_eq!(execution.act_executions().len(), 1);
    let act_exec = &execution.act_executions()[0];

    match &act_exec.inputs()[0] {
        Input::Text(text) => {
            assert!(text.contains(','), "CSV output should contain commas");
            assert!(
                text.contains("Alice") || text.contains("Bob") || text.contains("Charlie"),
                "CSV should contain employee names: {}",
                text
            );
        }
        _ => panic!("Expected Text input after table processing"),
    }
    Ok(())
}
