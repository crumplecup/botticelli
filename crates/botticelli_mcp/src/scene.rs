//! Scene management types for narrative editing.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for creating a new scene in a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSceneParams {
    /// The narrative ID to add the scene to
    pub narrative_id: String,

    /// Name of the new scene
    pub scene_name: String,

    /// Optional description of the scene
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Result from creating a scene.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CreateSceneResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// The generated scene ID
    pub scene_id: String,

    /// The narrative ID the scene belongs to
    pub narrative_id: String,

    /// The scene name
    pub name: String,

    /// Optional scene description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl CreateSceneResult {
    /// Create a new scene creation result.
    pub fn new(
        scene_id: String,
        narrative_id: String,
        name: String,
        description: Option<String>,
    ) -> Self {
        Self {
            success: true,
            scene_id,
            narrative_id,
            name,
            description,
        }
    }
}

/// Parameters for listing scenes in a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListScenesParams {
    /// The narrative ID to list scenes from
    pub narrative_id: String,
}

/// Result from listing scenes.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ListScenesResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// The narrative ID
    pub narrative_id: String,

    /// List of scenes (currently empty - placeholder for future implementation)
    pub scenes: Vec<Value>,
}

impl ListScenesResult {
    /// Create a new scene list result.
    pub fn new(narrative_id: String, scenes: Vec<Value>) -> Self {
        Self {
            success: true,
            narrative_id,
            scenes,
        }
    }
}

/// Parameters for updating a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSceneParams {
    /// The scene ID to update
    pub scene_id: String,

    /// Updates to apply (flexible object structure)
    pub updates: Value,
}

/// Result from updating a scene.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct UpdateSceneResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// The scene ID that was updated
    pub scene_id: String,

    /// The updates that were applied
    pub updates: Value,
}

impl UpdateSceneResult {
    /// Create a new scene update result.
    pub fn new(scene_id: String, updates: Value) -> Self {
        Self {
            success: true,
            scene_id,
            updates,
        }
    }
}

/// Parameters for deleting a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteSceneParams {
    /// The scene ID to delete
    pub scene_id: String,
}

/// Result from deleting a scene.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DeleteSceneResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// The scene ID that was deleted
    pub scene_id: String,

    /// Confirmation that the scene was deleted
    pub deleted: bool,
}

impl DeleteSceneResult {
    /// Create a new scene deletion result.
    pub fn new(scene_id: String) -> Self {
        Self {
            success: true,
            scene_id,
            deleted: true,
        }
    }
}
