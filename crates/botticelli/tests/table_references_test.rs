//! Integration tests for table references in narratives.
//!
//! These tests verify that narratives can query database tables
//! and include the results in prompts.

#![cfg(feature = "database")]

use botticelli::{
    ActConfig, DatabaseTableQueryRegistry, Input, NarrativeExecutor, NarrativeMetadata,
    NarrativeProvider, Output, TableFormat, TableQueryExecutor,
};
use diesel::prelude::*;
use std::{
    env,
    sync::{Arc, Mutex},
};

/// Test narrative provider that queries a table.
struct TableReferenceNarrative {
    metadata: NarrativeMetadata,
    act_names: Vec<String>,
    acts: Vec<(String, ActConfig)>,
}

impl TableReferenceNarrative {
    fn new(table_name: &str) -> botticelli::BotticelliResult<Self> {
        // Create a simple test metadata - NarrativeMetadata is typically deserialized from TOML
        // For testing, we'll construct acts directly
        // Unwrap is acceptable here as this is test setup data that should always parse
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
            format: TableFormat::Markdown,
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

/// Mock driver that returns the table data as-is for verification.
struct MockDriver;

#[async_trait::async_trait]
impl botticelli::BotticelliDriver for MockDriver {
    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn rate_limits(&self) -> &botticelli::RateLimitConfig {
        // For testing, use unlimited rate limits
        use botticelli::RateLimitConfig;
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
        request: &botticelli::GenerateRequest,
    ) -> botticelli::BotticelliResult<botticelli::GenerateResponse> {
        // Extract the table data from the request messages
        let mut table_content = String::new();
        for message in request.messages() {
            for input in message.content() {
                if let Input::Text(text) = input {
                    table_content = text.clone();
                    break;
                }
            }
        }

        Ok(botticelli::GenerateResponse::builder()
            .outputs(vec![Output::Text(format!(
                "Received table data: {}",
                table_content
            ))])
            .build()
            .expect("Valid response"))
    }
}

#[tokio::test]
async fn test_table_reference_query() -> botticelli::BotticelliResult<()> {
    use botticelli::{ConfigError, DatabaseError, DatabaseErrorKind};

    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| ConfigError::new("DATABASE_URL environment variable not set"))?;
    let mut conn = PgConnection::establish(&database_url).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Connection(format!(
            "Failed to establish connection: {}",
            e
        )))
    })?;

    // Create a test table
    diesel::sql_query(
        "CREATE TEMP TABLE test_products (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            price DECIMAL NOT NULL,
            category TEXT NOT NULL
        )",
    )
    .execute(&mut conn)
    .map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to create test table: {}",
            e
        )))
    })?;

    // Insert test data
    diesel::sql_query(
        "INSERT INTO test_products (name, price, category) VALUES
            ('Widget', 9.99, 'Tools'),
            ('Gadget', 19.99, 'Electronics'),
            ('Doohickey', 4.99, 'Tools'),
            ('Thingamajig', 14.99, 'Home'),
            ('Whatsit', 7.99, 'Electronics')",
    )
    .execute(&mut conn)
    .map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to insert test data: {}",
            e
        )))
    })?;

    // Use the same connection for the query executor so it can see the temp table
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative
    let narrative = TableReferenceNarrative::new("test_products")?;

    // Create executor with table registry
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));

    // Execute narrative
    let execution = executor.execute(&narrative).await?;

    // Verify execution
    assert_eq!(execution.act_executions.len(), 1);
    let act_exec = &execution.act_executions[0];
    assert_eq!(act_exec.act_name, "query_table");

    // Verify that table data was processed
    if execution.act_executions[0].inputs.is_empty() {
        return Err(DatabaseError::new(DatabaseErrorKind::Query(
            "No inputs found after table processing".to_string(),
        ))
        .into());
    }

    match &execution.act_executions[0].inputs[0] {
        Input::Text(text) => {
            // Should contain formatted table data
            if !text.contains("Widget") && !text.contains("Gadget") {
                return Err(DatabaseError::new(DatabaseErrorKind::Query(
                    "Table data not found in processed input".to_string(),
                ))
                .into());
            }
        }
        _ => {
            return Err(DatabaseError::new(DatabaseErrorKind::Query(
                "Expected Text input after table processing".to_string(),
            ))
            .into());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_table_reference_with_filter() -> botticelli::BotticelliResult<()> {
    use botticelli::{ConfigError, DatabaseError, DatabaseErrorKind};

    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| ConfigError::new("DATABASE_URL environment variable not set"))?;
    let mut conn = PgConnection::establish(&database_url).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Connection(format!(
            "Failed to establish connection: {}",
            e
        )))
    })?;

    // Create test table
    diesel::sql_query(
        "CREATE TEMP TABLE test_orders (
            id SERIAL PRIMARY KEY,
            customer TEXT NOT NULL,
            total DECIMAL NOT NULL,
            status TEXT NOT NULL
        )",
    )
    .execute(&mut conn)
    .expect("Failed to create test table");

    // Insert test data
    diesel::sql_query(
        "INSERT INTO test_orders (customer, total, status) VALUES
            ('Alice', 100.00, 'completed'),
            ('Bob', 150.00, 'pending'),
            ('Charlie', 200.00, 'completed'),
            ('Diana', 75.00, 'cancelled'),
            ('Eve', 300.00, 'completed')",
    )
    .execute(&mut conn)
    .expect("Failed to insert test data");

    // Use the same connection for the query executor so it can see the temp table
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative with WHERE clause
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

    // Create executor with table registry
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));

    // Execute narrative
    let execution = executor.execute(&narrative).await?;

    // Verify execution
    assert_eq!(execution.act_executions.len(), 1);
    let act_exec = &execution.act_executions[0];

    // Verify filtered data (should only have completed orders, sorted by total DESC)
    match &act_exec.inputs[0] {
        Input::Text(text) => {
            // Should contain Eve (highest total) and not contain Bob or Diana
            assert!(text.contains("Eve") || text.contains("300"));
            assert!(!text.contains("Bob"));
            assert!(!text.contains("Diana"));
        }
        _ => panic!("Expected Text input after table processing"),
    }
    Ok(())
}

#[tokio::test]
async fn test_table_reference_format_csv() -> botticelli::BotticelliResult<()> {
    use botticelli::{ConfigError, DatabaseError, DatabaseErrorKind};

    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| ConfigError::new("DATABASE_URL environment variable not set"))?;
    let mut conn = PgConnection::establish(&database_url).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Connection(format!(
            "Failed to establish connection: {}",
            e
        )))
    })?;

    // Create test table
    diesel::sql_query(
        "CREATE TEMP TABLE test_employees (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            department TEXT NOT NULL,
            salary DECIMAL NOT NULL
        )",
    )
    .execute(&mut conn)
    .map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to create test table: {}",
            e
        )))
    })?;

    // Insert test data
    diesel::sql_query(
        "INSERT INTO test_employees (name, department, salary) VALUES
            ('Alice', 'Engineering', 95000),
            ('Bob', 'Marketing', 75000),
            ('Charlie', 'Engineering', 105000)",
    )
    .execute(&mut conn)
    .map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to insert test data: {}",
            e
        )))
    })?;

    // Use the same connection for the query executor so it can see the temp table
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative with CSV format
    let metadata: NarrativeMetadata = serde_json::from_str(
        r#"{
        "name": "csv_format_test",
        "description": "Test CSV format output",
        "skip_content_generation": false
    }"#,
    )
    .unwrap();

    let table_input = Input::Table {
        table_name: "test_employees".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(10),
        offset: None,
        order_by: None,
        alias: Some("employees".to_string()),
        format: TableFormat::Csv,
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

    struct CsvNarrative {
        metadata: NarrativeMetadata,
        act_names: Vec<String>,
        act_config: ActConfig,
    }

    impl NarrativeProvider for CsvNarrative {
        fn name(&self) -> &str {
            "csv_format_test"
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

    let narrative = CsvNarrative {
        metadata,
        act_names: vec!["query_csv".to_string()],
        act_config,
    };

    // Create executor with table registry
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));

    // Execute narrative
    let execution = executor.execute(&narrative).await?;

    // Verify execution
    assert_eq!(execution.act_executions.len(), 1);
    let act_exec = &execution.act_executions[0];

    // Verify CSV formatting
    match &act_exec.inputs[0] {
        Input::Text(text) => {
            // CSV should have comma-separated values
            assert!(text.contains(','));
            assert!(text.contains("Alice") || text.contains("Bob") || text.contains("Charlie"));
        }
        _ => panic!("Expected Text input after table processing"),
    }
    Ok(())
}
