//! Table view trait for database query specifications.

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
