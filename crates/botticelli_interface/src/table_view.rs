//! Table view trait for database query specifications.

use serde::{Deserialize, Serialize};

/// Trait for table view specifications that define database queries.
///
/// A table view encapsulates the parameters needed to query a specific table,
/// including filtering, ordering, and pagination options.
pub trait TableView: Send + Sync {
    /// The name of the table being queried.
    fn table_name(&self) -> &str;

    /// Optional filter conditions for the query.
    fn filter(&self) -> Option<&str> {
        None
    }

    /// Optional ordering specification (e.g., "created_at DESC").
    fn order_by(&self) -> Option<&str> {
        None
    }

    /// Optional limit on the number of rows returned.
    fn limit(&self) -> Option<i64> {
        None
    }

    /// Optional offset for pagination.
    fn offset(&self) -> Option<i64> {
        None
    }

    /// Additional query parameters as key-value pairs.
    fn parameters(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }
}

/// Reference to a table query in a narrative.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
)]
#[setters(prefix = "with_")]
pub struct TableReference {
    /// Unique identifier for this table reference.
    id: String,
    /// The table view specification.
    view: String,
}

impl TableReference {
    /// Creates a new table reference.
    pub fn new(id: impl Into<String>, view: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            view: view.into(),
        }
    }
}
