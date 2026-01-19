//! Table reference resolution for narrative inputs.

use botticelli_error::{BotticelliError, BotticelliResult};
use botticelli_interface::ContentRepository;
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// Reference to content in a database table.
///
/// Allows narratives to include dynamically generated content
/// by referencing tables created during content generation.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Getters,
    Builder,
    elicitation::Elicit,
)]
#[builder(setter(into))]
pub struct TableReference {
    /// Name of the table to query
    table_name: String,

    /// Optional filter by review status
    #[builder(default)]
    status_filter: Option<String>,

    /// Maximum number of rows to retrieve
    #[builder(default = "10")]
    limit: i64,
}

impl TableReference {
    /// Create a new builder for constructing a table reference.
    pub fn builder() -> TableReferenceBuilder {
        TableReferenceBuilder::default()
    }

    /// Resolve this reference to actual content as JSON values.
    pub async fn resolve(
        &self,
        repository: &dyn ContentRepository<Error = botticelli_error::DatabaseError>,
    ) -> BotticelliResult<Vec<serde_json::Value>> {
        repository
            .query_content(
                &self.table_name,
                self.status_filter.as_deref(),
                Some(self.limit),
            )
            .await
            .map_err(BotticelliError::from)
    }
}
