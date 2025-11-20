//! Concrete table view implementations with builder pattern.

use crate::TableView;
use derive_builder::Builder;
use derive_getters::Getters;

/// A query view for selecting rows from a database table.
#[derive(Debug, Clone, Builder, Getters)]
#[builder(setter(into, strip_option))]
pub struct TableQueryView {
    /// Name of the table to query.
    #[builder(setter(into))]
    table_name: String,

    /// Columns to select (None = SELECT *).
    #[builder(default)]
    columns: Option<Vec<String>>,

    /// WHERE clause for filtering.
    #[builder(default)]
    filter: Option<String>,

    /// ORDER BY clause.
    #[builder(default)]
    order_by: Option<String>,

    /// Row limit.
    #[builder(default)]
    limit: Option<i64>,

    /// Row offset for pagination.
    #[builder(default)]
    offset: Option<i64>,

    /// Output format: "json", "markdown", or "csv".
    #[builder(default = "String::from(\"json\")")]
    format: String,
}

impl TableView for TableQueryView {
    fn table_name(&self) -> &str {
        &self.table_name
    }

    fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    fn order_by(&self) -> Option<&str> {
        self.order_by.as_deref()
    }

    fn limit(&self) -> Option<i64> {
        self.limit
    }

    fn offset(&self) -> Option<i64> {
        self.offset
    }
}

/// A view for counting rows in a database table.
#[derive(Debug, Clone, Builder, Getters)]
#[builder(setter(into, strip_option))]
pub struct TableCountView {
    /// Name of the table to query.
    #[builder(setter(into))]
    table_name: String,

    /// WHERE clause for filtering.
    #[builder(default)]
    filter: Option<String>,
}

impl TableView for TableCountView {
    fn table_name(&self) -> &str {
        &self.table_name
    }

    fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }
}
