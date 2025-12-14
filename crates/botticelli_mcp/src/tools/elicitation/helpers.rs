/// Helper methods for narrative analysis and validation.

use serde_json::{json, Value};
use tracing::instrument;

/// Extract suggested name from description.
#[instrument]
pub fn extract_suggested_name(description: &str) -> String {
    description
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Analyze description complexity and characteristics.
#[instrument]
pub fn analyze_complexity(description: &str) -> Value {
    let word_count = description.split_whitespace().count();
    let has_steps = description.to_lowercase().contains("step")
        || description.to_lowercase().contains("then")
        || description.to_lowercase().contains("first");
    let has_data = description.to_lowercase().contains("database")
        || description.to_lowercase().contains("table")
        || description.to_lowercase().contains("query");
    let has_iteration = description.to_lowercase().contains("repeat")
        || description.to_lowercase().contains("loop")
        || description.to_lowercase().contains("iterate")
        || description.to_lowercase().contains("for each");

    let complexity = if word_count < 20 {
        "simple"
    } else if word_count < 50 {
        "moderate"
    } else {
        "complex"
    };

    let mut recommendations = Vec::new();
    if has_steps {
        recommendations.push("Description mentions steps - consider using sequential or named acts");
    }
    if has_data {
        recommendations.push("Description mentions data sources - consider adding Table inputs");
    }
    if has_iteration {
        recommendations.push("Description mentions iteration - consider adding a carousel");
    }
    if word_count < 10 {
        recommendations.push("Brief description - you may want to provide more detail");
    }

    json!({
        "complexity": complexity,
        "word_count": word_count,
        "has_steps": has_steps,
        "has_data_sources": has_data,
        "has_iteration": has_iteration,
        "recommendations": recommendations
    })
}

/// Validate narrative name format.
#[instrument]
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}
