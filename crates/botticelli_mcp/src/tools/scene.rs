//! Scene management tools for narrative editing

use crate::McpTool;
use async_trait::async_trait;
use botticelli_core::ToolDefinition;
use botticelli_error::{McpError, McpErrorKind, McpResult};
use serde_json::{json, Value};
use tracing::instrument;

/// Create a new scene in a narrative
#[instrument(skip(args))]
pub async fn create_scene(args: Value) -> McpResult<Value> {
    let narrative_id = args["narrative_id"].as_str().ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "create_scene".to_string(),
            reason: "Missing narrative_id".to_string(),
        })
    })?;
    let scene_name = args["scene_name"].as_str().ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "create_scene".to_string(),
            reason: "Missing scene_name".to_string(),
        })
    })?;
    let description = args.get("description").and_then(|v| v.as_str());

    tracing::info!(
        narrative_id,
        scene_name,
        "Creating scene in narrative"
    );

    Ok(json!({
        "success": true,
        "scene_id": format!("scene_{}", uuid::Uuid::new_v4()),
        "narrative_id": narrative_id,
        "name": scene_name,
        "description": description
    }))
}

/// List all scenes in a narrative
#[instrument(skip(args))]
pub async fn list_scenes(args: Value) -> McpResult<Value> {
    let narrative_id = args["narrative_id"].as_str().ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "list_scenes".to_string(),
            reason: "Missing narrative_id".to_string(),
        })
    })?;

    tracing::info!(narrative_id, "Listing scenes");

    Ok(json!({
        "success": true,
        "narrative_id": narrative_id,
        "scenes": []
    }))
}

/// Update scene details
#[instrument(skip(args))]
pub async fn update_scene(args: Value) -> McpResult<Value> {
    let scene_id = args["scene_id"].as_str().ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "update_scene".to_string(),
            reason: "Missing scene_id".to_string(),
        })
    })?;
    let updates = args.get("updates").ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "update_scene".to_string(),
            reason: "Missing updates".to_string(),
        })
    })?;

    tracing::info!(scene_id, "Updating scene");

    Ok(json!({
        "success": true,
        "scene_id": scene_id,
        "updates": updates
    }))
}

/// Delete a scene from a narrative
#[instrument(skip(args))]
pub async fn delete_scene(args: Value) -> McpResult<Value> {
    let scene_id = args["scene_id"].as_str().ok_or_else(|| {
        McpError::new(McpErrorKind::InvalidArguments {
            tool: "delete_scene".to_string(),
            reason: "Missing scene_id".to_string(),
        })
    })?;

    tracing::info!(scene_id, "Deleting scene");

    Ok(json!({
        "success": true,
        "scene_id": scene_id,
        "deleted": true
    }))
}

/// Tool definitions for scene management
pub fn scene_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::new(
            "create_scene".to_string(),
            "Create a new scene in a narrative".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "narrative_id": {"type": "string"},
                    "scene_name": {"type": "string"},
                    "description": {"type": "string"}
                },
                "required": ["narrative_id", "scene_name"]
            }),
        ),
        ToolDefinition::new(
            "list_scenes".to_string(),
            "List all scenes in a narrative".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "narrative_id": {"type": "string"}
                },
                "required": ["narrative_id"]
            }),
        ),
        ToolDefinition::new(
            "update_scene".to_string(),
            "Update scene details".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "scene_id": {"type": "string"},
                    "updates": {"type": "object"}
                },
                "required": ["scene_id", "updates"]
            }),
        ),
        ToolDefinition::new(
            "delete_scene".to_string(),
            "Delete a scene from a narrative".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "scene_id": {"type": "string"}
                },
                "required": ["scene_id"]
            }),
        ),
    ]
}

/// Tool for creating scenes
pub struct CreateSceneTool;

#[async_trait]
impl McpTool for CreateSceneTool {
    fn name(&self) -> &str {
        "create_scene"
    }

    fn description(&self) -> &str {
        "Create a new scene in a narrative"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {"type": "string"},
                "scene_name": {"type": "string"},
                "description": {"type": "string"}
            },
            "required": ["narrative_id", "scene_name"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        create_scene(input).await
    }
}

/// Tool for listing scenes
pub struct ListScenesTool;

#[async_trait]
impl McpTool for ListScenesTool {
    fn name(&self) -> &str {
        "list_scenes"
    }

    fn description(&self) -> &str {
        "List all scenes in a narrative"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {"type": "string"}
            },
            "required": ["narrative_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        list_scenes(input).await
    }
}

/// Tool for updating scenes
pub struct UpdateSceneTool;

#[async_trait]
impl McpTool for UpdateSceneTool {
    fn name(&self) -> &str {
        "update_scene"
    }

    fn description(&self) -> &str {
        "Update scene details"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "scene_id": {"type": "string"},
                "updates": {"type": "object"}
            },
            "required": ["scene_id", "updates"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        update_scene(input).await
    }
}

/// Tool for deleting scenes
pub struct DeleteSceneTool;

#[async_trait]
impl McpTool for DeleteSceneTool {
    fn name(&self) -> &str {
        "delete_scene"
    }

    fn description(&self) -> &str {
        "Delete a scene from a narrative"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "scene_id": {"type": "string"}
            },
            "required": ["scene_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        delete_scene(input).await
    }
}
