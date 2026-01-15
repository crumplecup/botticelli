//! Scene management tools.
//!
//! CRUD operations for narrative scenes.

use super::super::server::BotticelliServer;
use crate::{
    CreateSceneParams, CreateSceneResult, DeleteSceneParams, DeleteSceneResult, ListScenesParams,
    ListScenesResult, UpdateSceneParams, UpdateSceneResult,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::debug;

impl BotticelliServer {
    pub async fn create_scene(
        &self,
        Parameters(CreateSceneParams {
            narrative_id,
            scene_name,
            description,
        }): Parameters<CreateSceneParams>,
    ) -> Result<Json<CreateSceneResult>, rmcp::ErrorData> {
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
    pub async fn list_scenes(
        &self,
        Parameters(ListScenesParams { narrative_id }): Parameters<ListScenesParams>,
    ) -> Result<Json<ListScenesResult>, rmcp::ErrorData> {
        debug!(narrative_id = %narrative_id, "Listing scenes");

        let result = ListScenesResult::new(narrative_id, vec![]);
        Ok(Json(result))
    }
    pub async fn update_scene(
        &self,
        Parameters(UpdateSceneParams { scene_id, updates }): Parameters<UpdateSceneParams>,
    ) -> Result<Json<UpdateSceneResult>, rmcp::ErrorData> {
        debug!(scene_id = %scene_id, "Updating scene");

        let result = UpdateSceneResult::new(scene_id, updates);
        Ok(Json(result))
    }
    pub async fn delete_scene(
        &self,
        Parameters(DeleteSceneParams { scene_id }): Parameters<DeleteSceneParams>,
    ) -> Result<Json<DeleteSceneResult>, rmcp::ErrorData> {
        debug!(scene_id = %scene_id, "Deleting scene");

        let result = DeleteSceneResult::new(scene_id);
        Ok(Json(result))
    }
}
