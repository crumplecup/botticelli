//! Table reference resolution for narrative inputs.

use botticelli_error::BotticelliResult;
use botticelli_interface::ContentRepository;
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// Reference to content in a database table.
///
/// Allows narratives to include dynamically generated content
/// by referencing tables created during content generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Getters, Builder)]
#[builder(setter(into))]
pub struct TableReference {
    /// Name of the table to query
    table_name: String,
    
    /// Optional filter by review status
    #[builder(default)]
    status_filter: Option<String>,
    
    /// Maximum number of rows to retrieve
    #[builder(default = "10")]
    limit: usize,
}

impl TableReference {
    /// Create a new builder for constructing a table reference.
    pub fn builder() -> TableReferenceBuilder {
        TableReferenceBuilder::default()
    }
    
    /// Resolve this reference to actual content as JSON values.
    pub async fn resolve(
        &self,
        repository: &dyn ContentRepository,
    ) -> BotticelliResult<Vec<serde_json::Value>> {
        repository
            .list_content(
                &self.table_name,
                self.status_filter.as_deref(),
                self.limit,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_reference_builder() {
        let reference = TableReference::builder()
            .table_name("my_content")
            .status_filter(Some("approved".to_string()))
            .limit(20_usize)
            .build()
            .unwrap();

        assert_eq!(reference.table_name(), "my_content");
        assert_eq!(reference.status_filter(), &Some("approved".to_string()));
        assert_eq!(reference.limit(), &20);
    }

    #[test]
    fn test_table_reference_defaults() {
        let reference = TableReference::builder()
            .table_name("my_content")
            .build()
            .unwrap();

        assert_eq!(reference.table_name(), "my_content");
        assert_eq!(reference.status_filter(), &None);
        assert_eq!(reference.limit(), &10);
    }
}
