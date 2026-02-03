//! Data extraction from TOML values.

use rmcp::tool;
use std::collections::HashMap;
use tracing::instrument;

/// Unit struct providing data extraction methods for TOML validation.
///
/// Groups extraction-related functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct DataExtractor;

impl DataExtractor {
    /// Extracts toc.order from a toc value.
    #[instrument(skip(toc_value), fields(order_len = tracing::field::Empty))]
    #[tool]
    pub fn toc_order(toc_value: Option<&toml::Value>) -> Vec<String> {
        let toc_value = match toc_value {
            Some(v) => v,
            None => {
                tracing::debug!("No toc value provided");
                return Vec::new();
            }
        };

        // Handle array format: toc = ["act1", "act2"]
        if let Some(arr) = toc_value.as_array() {
            let order: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            tracing::Span::current().record("order_len", order.len());
            tracing::debug!(len = order.len(), "Extracted toc order from array");
            return order;
        }

        // Handle table format: [toc] with order field
        if let Some(table) = toc_value.as_table()
            && let Some(order) = table.get("order").and_then(|v| v.as_array())
        {
            let order: Vec<String> = order
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            tracing::Span::current().record("order_len", order.len());
            tracing::debug!(len = order.len(), "Extracted toc order from table");
            return order;
        }

        Vec::new()
    }

    /// Extracts acts map from an acts value.
    #[instrument(skip(acts_value), fields(act_count = tracing::field::Empty))]
    #[tool]
    pub fn acts(acts_value: Option<&toml::Value>) -> HashMap<String, toml::Value> {
        let acts_table = match acts_value.and_then(|v| v.as_table()) {
            Some(t) => t,
            None => {
                tracing::debug!("No acts table found");
                return HashMap::new();
            }
        };

        let acts: HashMap<String, toml::Value> = acts_table
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        tracing::Span::current().record("act_count", acts.len());
        tracing::debug!(count = acts.len(), "Extracted acts");
        acts
    }
}
