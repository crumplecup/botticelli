//! MCP tool for creating narrative elicitation sessions.

use crate::tools::McpTool;
use crate::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info, instrument};

use super::NarrativeRegistry;

/// MCP tool for initializing narrative creation sessions.
///
/// Creates a new session with a UUID, analyzes the user's description,
/// and provides suggestions for narrative structure.
pub struct CreateNarrativeSessionTool {
    registry: Arc<NarrativeRegistry>,
}

impl CreateNarrativeSessionTool {
    /// Create tool with shared registry.
    pub fn new(registry: Arc<NarrativeRegistry>) -> Self {
        Self { registry }
    }

    /// Analyze description and suggest structure.
    ///
    /// Simple heuristic-based analysis for MVP.
    /// Future: Use LLM for smarter analysis.
    #[instrument(skip(description))]
    fn analyze_description(description: &str) -> Value {
        debug!("Analyzing description");

        let word_count = description.split_whitespace().count();
        let has_steps = description.to_lowercase().contains("step")
            || description.to_lowercase().contains("then")
            || description.to_lowercase().contains("first");
        let has_data = description.to_lowercase().contains("database")
            || description.to_lowercase().contains("table")
            || description.to_lowercase().contains("query");

        // Suggest name from first few words
        let suggested_name = description
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join("_")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

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
        if word_count < 10 {
            recommendations.push("Brief description - you may want to provide more detail about the workflow");
        }

        json!({
            "suggested_name": suggested_name,
            "complexity": complexity,
            "has_steps": has_steps,
            "has_data_sources": has_data,
            "recommendations": recommendations
        })
    }
}

#[async_trait]
impl McpTool for CreateNarrativeSessionTool {
    fn name(&self) -> &str {
        "create_narrative_session"
    }

    fn description(&self) -> &str {
        "Initialize a new narrative creation session. Call this when the user wants to create a narrative. \
         Returns a narrative_id (UUID) that must be used in all subsequent tool calls for this narrative."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "User's initial description of what they want to create"
                }
            },
            "required": ["description"]
        })
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Creating narrative session");

        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'description' field".to_string()))?;

        // Create initial state with description
        let state = json!({
            "description": description,
            "acts": {},
            "act_order": []
        });

        // Register session
        let narrative_id = self.registry.create_session(state);

        // Analyze description
        let analysis = Self::analyze_description(description);

        info!(narrative_id = %narrative_id, "Created narrative session");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "analysis": analysis
        }))
    }
}
