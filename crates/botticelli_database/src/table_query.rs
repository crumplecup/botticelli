//! `TableQueryRegistry` implementation backed by `BotStorage`.
//!
//! Bridges the narrative executor's table query interface to the KV storage
//! backend, enabling narratives to read content tables without a SQL database.

use async_trait::async_trait;
use botticelli_interface::{BotStorage, TableQueryRegistry, TableQueryView};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Implements [`TableQueryRegistry`] over an [`Arc<dyn BotStorage>`].
///
/// Queries are resolved by calling [`BotStorage::list_content`], filtering
/// in Rust with the provided WHERE clause, then formatting as JSON, Markdown,
/// or CSV.
pub struct BotStorageTableQueryRegistry {
    storage: Arc<dyn BotStorage>,
}

impl BotStorageTableQueryRegistry {
    /// Create a new registry backed by the given storage.
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        Self { storage }
    }
}

impl std::fmt::Debug for BotStorageTableQueryRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotStorageTableQueryRegistry").finish_non_exhaustive()
    }
}

/// Apply a simple `col = 'value'` or `col = number` WHERE clause to a JSON row.
#[instrument(skip(json))]
fn matches_where(json: &JsonValue, where_clause: &str) -> bool {
    let clause = where_clause.trim();
    if let Some((col, val_part)) = clause.split_once(" = ") {
        let col = col.trim();
        let val = val_part.trim();
        if val.starts_with('\'') && val.ends_with('\'') {
            let expected = &val[1..val.len() - 1];
            return json.get(col).and_then(|v| v.as_str()) == Some(expected);
        }
        if let Ok(n) = val.parse::<i64>() {
            return json.get(col).and_then(|v| v.as_i64()) == Some(n);
        }
    }
    false
}

/// Extract the columns specified in the query view, or return all keys.
#[instrument(skip(json, columns))]
fn project_columns(json: &JsonValue, columns: Option<&[String]>) -> JsonValue {
    match columns {
        None => json.clone(),
        Some(cols) => {
            let mut map = serde_json::Map::new();
            for col in cols {
                if let Some(v) = json.get(col) {
                    map.insert(col.clone(), v.clone());
                }
            }
            JsonValue::Object(map)
        }
    }
}

/// Sort a list of JSON values by the ORDER BY clause.
///
/// Supports `field ASC` and `field DESC` (case-insensitive suffix).
#[instrument(skip(rows))]
fn apply_order_by(rows: &mut [JsonValue], order_by: &str) {
    let order_by = order_by.trim();
    let (field, descending) = if let Some(col) = order_by.strip_suffix(" DESC") {
        (col.trim(), true)
    } else if let Some(col) = order_by.strip_suffix(" ASC") {
        (col.trim(), false)
    } else {
        (order_by, false)
    };

    rows.sort_by(|a, b| {
        let va = a.get(field);
        let vb = b.get(field);
        let ord = compare_json_values(va, vb);
        if descending { ord.reverse() } else { ord }
    });
}

fn compare_json_values(a: Option<&JsonValue>, b: Option<&JsonValue>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(a), Some(b)) => {
            if let (Some(an), Some(bn)) = (a.as_f64(), b.as_f64()) {
                an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
            } else if let (Some(as_), Some(bs)) = (a.as_str(), b.as_str()) {
                as_.cmp(bs)
            } else {
                a.to_string().cmp(&b.to_string())
            }
        }
    }
}

/// Format a row list as a JSON array string.
#[instrument(skip(rows))]
fn format_json(rows: &[JsonValue]) -> String {
    serde_json::to_string_pretty(&JsonValue::Array(rows.to_vec()))
        .unwrap_or_else(|_| "[]".to_string())
}

/// Format a row list as a Markdown table string.
#[instrument(skip(rows))]
fn format_markdown(rows: &[JsonValue]) -> String {
    if rows.is_empty() {
        return "_No results_".to_string();
    }

    let all_keys: Vec<String> = rows
        .iter()
        .filter_map(|r| r.as_object())
        .flat_map(|m| m.keys().cloned())
        .fold(Vec::new(), |mut acc, k| {
            if !acc.contains(&k) {
                acc.push(k);
            }
            acc
        });

    let header = format!("| {} |", all_keys.join(" | "));
    let separator = format!("| {} |", all_keys.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));

    let row_lines: Vec<String> = rows
        .iter()
        .map(|r| {
            let cells: Vec<String> = all_keys
                .iter()
                .map(|k| {
                    r.get(k)
                        .map(|v| match v {
                            JsonValue::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            format!("| {} |", cells.join(" | "))
        })
        .collect();

    [header, separator].into_iter().chain(row_lines).collect::<Vec<_>>().join("\n")
}

/// Format a row list as CSV.
#[instrument(skip(rows))]
fn format_csv(rows: &[JsonValue]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let all_keys: Vec<String> = rows
        .iter()
        .filter_map(|r| r.as_object())
        .flat_map(|m| m.keys().cloned())
        .fold(Vec::new(), |mut acc, k| {
            if !acc.contains(&k) {
                acc.push(k);
            }
            acc
        });

    let header = all_keys.join(",");

    let row_lines: Vec<String> = rows
        .iter()
        .map(|r| {
            all_keys
                .iter()
                .map(|k| {
                    r.get(k)
                        .map(|v| match v {
                            JsonValue::String(s) => {
                                if s.contains(',') || s.contains('"') || s.contains('\n') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            other => other.to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();

    [header].into_iter().chain(row_lines).collect::<Vec<_>>().join("\n")
}

/// Execute the full query pipeline: load → filter → project → sort → paginate → format.
#[instrument(skip(storage, query), fields(table_name = %query.table_name(), format = %query.format()))]
async fn execute_query(
    storage: &Arc<dyn BotStorage>,
    query: &TableQueryView,
) -> Result<Vec<(String, JsonValue)>, Box<dyn std::error::Error + Send + Sync>> {
    let limit_for_fetch = (*query.limit())
        .map(|l| (l + (*query.offset()).unwrap_or(0)) as usize)
        .unwrap_or(10_000);

    let records = storage
        .list_content(query.table_name(), limit_for_fetch)
        .await
        .map_err(|e| format!("Storage error: {e}"))?;

    debug!(table_name = %query.table_name(), fetched = records.len(), "Loaded records from storage");

    let mut rows: Vec<(String, JsonValue)> = records
        .into_iter()
        .filter(|r| {
            if let Some(where_clause) = query.filter() {
                matches_where(&r.content_json, where_clause)
            } else {
                true
            }
        })
        .map(|r| {
            let projected = project_columns(&r.content_json, query.columns().as_deref());
            (r.id, projected)
        })
        .collect();

    if let Some(order_by) = query.order_by() {
        let mut just_rows: Vec<JsonValue> = rows.iter().map(|(_, v)| v.clone()).collect();
        apply_order_by(&mut just_rows, order_by);
        rows = rows.into_iter().zip(just_rows).map(|((id, _), v)| (id, v)).collect();
    }

    if let Some(offset) = *query.offset()
        && offset > 0
    {
        let skip = offset as usize;
        if skip >= rows.len() {
            return Ok(vec![]);
        }
        rows = rows.into_iter().skip(skip).collect();
    }

    if let Some(limit) = *query.limit() {
        rows.truncate(limit as usize);
    }

    Ok(rows)
}

#[async_trait]
impl TableQueryRegistry for BotStorageTableQueryRegistry {
    #[instrument(skip(self, query), fields(table_name = %query.table_name(), format = %query.format()))]
    async fn query_table(
        &self,
        query: &TableQueryView,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let rows = execute_query(&self.storage, query).await?;
        let json_rows: Vec<JsonValue> = rows.into_iter().map(|(_, v)| v).collect();

        info!(table_name = %query.table_name(), rows = json_rows.len(), format = %query.format(), "Query complete");

        let output = match query.format().as_str() {
            "markdown" => format_markdown(&json_rows),
            "csv" => format_csv(&json_rows),
            _ => format_json(&json_rows),
        };

        Ok(output)
    }

    #[instrument(skip(self, query), fields(table_name = %query.table_name(), format = %query.format()))]
    async fn query_and_delete_table(
        &self,
        query: &TableQueryView,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let rows = execute_query(&self.storage, query).await?;

        let (ids, json_rows): (Vec<String>, Vec<JsonValue>) = rows.into_iter().unzip();

        for id in &ids {
            self.storage
                .delete_content(id)
                .await
                .map_err(|e| format!("Failed to delete content row '{}': {}", id, e))?;
        }

        info!(
            table_name = %query.table_name(),
            rows = json_rows.len(),
            "Query-and-delete complete"
        );

        let output = match query.format().as_str() {
            "markdown" => format_markdown(&json_rows),
            "csv" => format_csv(&json_rows),
            _ => format_json(&json_rows),
        };

        Ok(output)
    }
}
