//! Scene management tools.
//!
//! CRUD operations for narrative scenes.

use crate::rmcp_server::BotticelliServer;
use crate::{
    CreateSceneParams, CreateSceneResult, DeleteSceneParams, DeleteSceneResult, ListScenesParams,
    ListScenesResult, UpdateSceneParams, UpdateSceneResult,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

#[tool_router(router = scene_tool_router, vis = "pub")]
impl BotticelliServer {
    /// Create a new scene in a narrative.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), scene_name = params.scene_name(), has_description = params.description().is_some()))]
    pub async fn create_scene(
        &self,
        Parameters(params): Parameters<CreateSceneParams>,
    ) -> Result<Json<CreateSceneResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let scene_name = params.scene_name().clone();
        let description = params.description().clone();

        debug!(
            narrative_id = %narrative_id,
            scene_name = %scene_name,
            has_description = description.is_some(),
            "Creating scene in narrative"
        );

        // Generate a new scene ID
        let scene_id = format!("scene_{}", uuid::Uuid::new_v4());

        debug!(scene_id = %scene_id, "Generated scene ID");

        let result = CreateSceneResult::new(scene_id, narrative_id, scene_name, description);
        Ok(Json(result))
    }

    /// List all scenes in a narrative.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id()))]
    pub async fn list_scenes(
        &self,
        Parameters(params): Parameters<ListScenesParams>,
    ) -> Result<Json<ListScenesResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        debug!(narrative_id = %narrative_id, "Listing scenes");

        let result = ListScenesResult::new(narrative_id, vec![]);
        Ok(Json(result))
    }

    /// Update an existing scene.
    #[instrument(skip(self, params), fields(scene_id = params.scene_id()))]
    pub async fn update_scene(
        &self,
        Parameters(params): Parameters<UpdateSceneParams>,
    ) -> Result<Json<UpdateSceneResult>, rmcp::ErrorData> {
        let scene_id = params.scene_id().clone();
        let updates = params.updates().clone();

        debug!(scene_id = %scene_id, "Updating scene");

        let result = UpdateSceneResult::new(scene_id, updates);
        Ok(Json(result))
    }

    /// Delete a scene from a narrative.
    #[instrument(skip(self, params), fields(scene_id = params.scene_id()))]
    pub async fn delete_scene(
        &self,
        Parameters(params): Parameters<DeleteSceneParams>,
    ) -> Result<Json<DeleteSceneResult>, rmcp::ErrorData> {
        let scene_id = params.scene_id().clone();
        debug!(scene_id = %scene_id, "Deleting scene");

        let result = DeleteSceneResult::new(scene_id);
        Ok(Json(result))
    }
}
