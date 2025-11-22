//! Integration tests for table references in narratives.
//!
//! These tests verify that narratives can query database tables
//! and include the results in prompts.

#![cfg(feature = "database")]

use botticelli::{
    ActConfig, DatabaseTableQueryRegistry, Input, NarrativeExecutor, NarrativeMetadata, NarrativeProvider, Output,
    TableFormat, TableQueryExecutor,
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::{env, sync::{Arc, Mutex}};

/// Test narrative provider that queries a table.
struct TableReferenceNarrative {
    metadata: NarrativeMetadata,
    act_names: Vec<String>,
    acts: Vec<(String, ActConfig)>,
}

impl TableReferenceNarrative {
    fn new(table_name: &str) -> Self {
        let metadata = NarrativeMetadata {
            name: "table_reference_test".to_string(),
            description: "Test narrative with table references".to_string(),
            template: None,
            skip_content_generation: false,
        };

        let act_config = ActConfig {
            inputs: vec![Input::Table {
                table_name: table_name.to_string(),
                columns: None,
                where_clause: None,
                limit: Some(5),
                offset: None,
                order_by: None,
                alias: Some("test_data".to_string()),
                format: TableFormat::Markdown,
                sample: None,
            }],
            model: Some("gemini-2.0-flash-lite".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(100),
        };

        let act_name = "query_table".to_string();
        Self {
            metadata,
            act_names: vec![act_name.clone()],
            acts: vec![(act_name, act_config)],
        }
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
        self.acts.iter().find(|(name, _)| name == act_name).map(|(_, config)| config.clone())
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

    async fn generate(
        &self,
        request: &botticelli::GenerateRequest,
    ) -> botticelli::BotticelliResult<botticelli::GenerateResponse> {
        // Extract the table data from the request messages
        let mut table_content = String::new();
        for message in &request.messages {
            for input in &message.content {
                if let Input::Text(text) = input {
                    table_content = text.clone();
                    break;
                }
            }
        }

        Ok(botticelli::GenerateResponse {
            outputs: vec![Output::Text(format!("Received table data: {}", table_content))],
        })
    }
}

/// Establish database connection for tests.
fn establish_connection() -> Pool<ConnectionManager<PgConnection>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager).expect("Failed to create pool")
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_table_reference_query() {
    let pool = establish_connection();
    let mut conn = pool.get().expect("Failed to get connection");

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
    .expect("Failed to create test table");

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
    .expect("Failed to insert test data");

    // Create table query registry
    // Note: TableQueryExecutor needs Arc<Mutex<PgConnection>>, not a pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let query_conn = PgConnection::establish(&database_url).expect("Failed to establish connection");
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(query_conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative
    let narrative = TableReferenceNarrative::new("test_products");

    // Create executor with table registry
    let executor = NarrativeExecutor::new(MockDriver).with_table_registry(Box::new(table_registry));

    // Execute narrative
    let execution = executor.execute(&narrative).await.expect("Execution failed");

    // Verify execution
    assert_eq!(execution.act_executions.len(), 1);
    let act_exec = &execution.act_executions[0];
    assert_eq!(act_exec.act_name, "query_table");

    // Verify that table data was processed
    assert_eq!(act_exec.inputs.len(), 1);
    match &act_exec.inputs[0] {
        Input::Text(text) => {
            // Should contain formatted table data
            assert!(text.contains("Widget") || text.contains("Gadget"));
        }
        _ => panic!("Expected Text input after table processing"),
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_table_reference_with_filter() {
    let pool = establish_connection();
    let mut conn = pool.get().expect("Failed to get connection");

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

    // Create table query registry
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let query_conn = PgConnection::establish(&database_url).expect("Failed to establish connection");
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(query_conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative with WHERE clause
    let metadata = NarrativeMetadata {
        name: "filtered_query_test".to_string(),
        description: "Test with WHERE clause filtering".to_string(),
        template: None,
        skip_content_generation: false,
    };

    let act_config = ActConfig {
        inputs: vec![Input::Table {
            table_name: "test_orders".to_string(),
            columns: Some(vec!["customer".to_string(), "total".to_string()]),
            where_clause: Some("status = 'completed'".to_string()),
            limit: Some(10),
            offset: None,
            order_by: Some("total DESC".to_string()),
            alias: Some("completed_orders".to_string()),
            format: TableFormat::Json,
            sample: None,
        }],
        model: Some("gemini-2.0-flash-lite".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };

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
    let execution = executor.execute(&narrative).await.expect("Execution failed");

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
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_table_reference_format_csv() {
    let pool = establish_connection();
    let mut conn = pool.get().expect("Failed to get connection");

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
    .expect("Failed to create test table");

    // Insert test data
    diesel::sql_query(
        "INSERT INTO test_employees (name, department, salary) VALUES
            ('Alice', 'Engineering', 95000),
            ('Bob', 'Marketing', 75000),
            ('Charlie', 'Engineering', 105000)",
    )
    .execute(&mut conn)
    .expect("Failed to insert test data");

    // Create table query registry
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let query_conn = PgConnection::establish(&database_url).expect("Failed to establish connection");
    let query_executor = TableQueryExecutor::new(Arc::new(Mutex::new(query_conn)));
    let table_registry = DatabaseTableQueryRegistry::new(query_executor);

    // Create narrative with CSV format
    let metadata = NarrativeMetadata {
        name: "csv_format_test".to_string(),
        description: "Test CSV format output".to_string(),
        template: None,
        skip_content_generation: false,
    };

    let act_config = ActConfig {
        inputs: vec![Input::Table {
            table_name: "test_employees".to_string(),
            columns: None,
            where_clause: None,
            limit: Some(10),
            offset: None,
            order_by: None,
            alias: Some("employees".to_string()),
            format: TableFormat::Csv,
            sample: None,
        }],
        model: Some("gemini-2.0-flash-lite".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };

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
    let execution = executor.execute(&narrative).await.expect("Execution failed");

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
}
