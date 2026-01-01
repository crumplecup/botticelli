//! Implementation of TableQueryRegistry for narrative integration.

use crate::{TableQueryExecutor, format_as_json};
use async_trait::async_trait;
use botticelli_interface::TableQueryRegistry;
use tracing::{debug, instrument};

/// Implementation of TableQueryRegistry using TableQueryExecutor.
pub struct DatabaseTableQueryRegistry {
    executor: TableQueryExecutor,
}

impl DatabaseTableQueryRegistry {
    /// Creates a new table query registry.
    pub fn new(executor: TableQueryExecutor) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TableQueryRegistry for DatabaseTableQueryRegistry {
    type Error = botticelli_error::BotticelliError;

    #[instrument(
        skip(self, query),
        fields(
            table_name = %query.table_name(),
            has_where = query.filter().is_some(),
            limit = ?query.limit(),
            offset = ?query.offset()
        )
    )]
    async fn query_table(
        &self,
        query: &dyn botticelli_interface::TableView,
    ) -> Result<String, Self::Error> {
        debug!("Executing table query");

        // Execute query
        let rows = self.executor.query_table(query)?;

        debug!(row_count = rows.len(), "Query executed successfully");

        // Format results as JSON (default format)
        let formatted = format_as_json(&rows);

        debug!(output_length = formatted.len(), "Results formatted");
        Ok(formatted)
    }

    #[instrument(
        skip(self, query),
        fields(
            table_name = %query.table_name(),
            has_where = query.filter().is_some(),
            limit = ?query.limit(),
            offset = ?query.offset()
        )
    )]
    async fn query_and_delete_table(
        &self,
        query: &dyn botticelli_interface::TableView,
    ) -> Result<String, Self::Error> {
        debug!("Executing destructive table query");

        // Execute query and delete
        let rows = self.executor.query_and_delete_table(query)?;

        debug!(
            row_count = rows.len(),
            "Query and delete executed successfully"
        );

        // Format results as JSON (default format)
        let formatted = format_as_json(&rows);

        debug!(output_length = formatted.len(), "Results formatted");
        Ok(formatted)
    }
}
