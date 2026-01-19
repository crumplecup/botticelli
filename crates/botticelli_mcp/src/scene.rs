//! Scene management types for narrative editing.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for creating a new scene in a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct CreateSceneParams {
    /// The narrative ID to add the scene to
    narrative_id: String,

    /// Name of the new scene
    scene_name: String,

    /// Optional description of the scene
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl CreateSceneParams {
    /// Create new create scene parameters.
    #[tracing::instrument(skip(narrative_id, scene_name), fields(narrative_id = %narrative_id, scene_name = %scene_name))]
    pub fn new(narrative_id: String, scene_name: String, description: Option<String>) -> Self {
        Self {
            narrative_id,
            scene_name,
            description,
        }
    }
}

/// Result from creating a scene.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct CreateSceneResult {
    /// Whether the operation succeeded
    success: bool,

    /// The generated scene ID
    scene_id: String,

    /// The narrative ID the scene belongs to
    narrative_id: String,

    /// The scene name
    name: String,

    /// Optional scene description
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl CreateSceneResult {
    /// Create a new scene creation result.
    #[tracing::instrument(skip(scene_id, narrative_id, name), fields(scene_id = %scene_id, narrative_id = %narrative_id, name = %name))]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct ListScenesParams {
    /// The narrative ID to list scenes from
    narrative_id: String,
}

impl ListScenesParams {
    /// Create new list scenes parameters.
    #[tracing::instrument(skip(narrative_id), fields(narrative_id = %narrative_id))]
    pub fn new(narrative_id: String) -> Self {
        Self { narrative_id }
    }
}

/// Result from listing scenes.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct ListScenesResult {
    /// Whether the operation succeeded
    success: bool,

    /// The narrative ID
    narrative_id: String,

    /// List of scenes (currently empty - placeholder for future implementation)
    scenes: Vec<Value>,
}

impl ListScenesResult {
    /// Create a new scene list result.
    #[tracing::instrument(skip(narrative_id, scenes), fields(narrative_id = %narrative_id, scene_count = scenes.len()))]
    pub fn new(narrative_id: String, scenes: Vec<Value>) -> Self {
        Self {
            success: true,
            narrative_id,
            scenes,
        }
    }
}

/// Parameters for updating a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct UpdateSceneParams {
    /// The scene ID to update
    scene_id: String,

    /// Updates to apply (flexible object structure)
    updates: Value,
}

impl UpdateSceneParams {
    /// Create new update scene parameters.
    #[tracing::instrument(skip(scene_id), fields(scene_id = %scene_id))]
    pub fn new(scene_id: String, updates: Value) -> Self {
        Self { scene_id, updates }
    }
}

/// Result from updating a scene.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct UpdateSceneResult {
    /// Whether the operation succeeded
    success: bool,

    /// The scene ID that was updated
    scene_id: String,

    /// The updates that were applied
    updates: Value,
}

impl UpdateSceneResult {
    /// Create a new scene update result.
    #[tracing::instrument(skip(scene_id), fields(scene_id = %scene_id))]
    pub fn new(scene_id: String, updates: Value) -> Self {
        Self {
            success: true,
            scene_id,
            updates,
        }
    }
}

/// Parameters for deleting a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct DeleteSceneParams {
    /// The scene ID to delete
    scene_id: String,
}

impl DeleteSceneParams {
    /// Create new delete scene parameters.
    #[tracing::instrument(skip(scene_id), fields(scene_id = %scene_id))]
    pub fn new(scene_id: String) -> Self {
        Self { scene_id }
    }
}

/// Result from deleting a scene.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct DeleteSceneResult {
    /// Whether the operation succeeded
    success: bool,

    /// The scene ID that was deleted
    scene_id: String,

    /// Confirmation that the scene was deleted
    deleted: bool,
}

impl DeleteSceneResult {
    /// Create a new scene deletion result.
    #[tracing::instrument(skip(scene_id), fields(scene_id = %scene_id))]
    pub fn new(scene_id: String) -> Self {
        Self {
            success: true,
            scene_id,
            deleted: true,
        }
    }
}
