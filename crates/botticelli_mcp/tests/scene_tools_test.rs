//! Tests for scene management tools.

mod helpers;

use botticelli_mcp::{
    BotticelliServer, CreateSceneParams, DeleteSceneParams, ListScenesParams, UpdateSceneParams,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

#[tokio::test]
async fn test_create_scene_with_description() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing scene creation with description");

    let server = BotticelliServer::builder().build();
    let params = CreateSceneParams::new("narrative_123".to_string(), "Opening Scene".to_string(), Some("The hero awakens in a strange land".to_string()));

    let result = server.create_scene(Parameters(params)).await?;
    let scene = result.0;
    tracing::debug!(
        scene_id = %scene.scene_id(),
        narrative_id = %scene.narrative_id(),
        "Created scene"
    );

    assert!(*scene.success());
    assert_eq!(scene.narrative_id(), "narrative_123");
    assert_eq!(scene.name(), "Opening Scene");
    assert_eq!(
        scene.description(),
        &Some("The hero awakens in a strange land".to_string())
    );
    assert!(scene.scene_id().starts_with("scene_"));

    tracing::info!("Scene creation with description test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_scene_without_description() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing scene creation without description");

    let server = BotticelliServer::builder().build();
    let params = CreateSceneParams::new("narrative_456".to_string(), "Battle Scene".to_string(), None);

    let result = server.create_scene(Parameters(params)).await?;
    let scene = result.0;
    tracing::debug!(scene_id = %scene.scene_id(), "Created scene without description");

    assert!(*scene.success());
    assert_eq!(scene.narrative_id(), "narrative_456");
    assert_eq!(scene.name(), "Battle Scene");
    assert_eq!(scene.description(), &None);

    tracing::info!("Scene creation without description test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_scene_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing CreateSceneParams serialization");

    let json_value = json!({
        "narrative_id": "narr_789",
        "scene_name": "Climax",
        "description": "The final confrontation"
    });

    let params: CreateSceneParams = serde_json::from_value(json_value)?;
    tracing::debug!(?params, "Deserialized params");

    assert_eq!(params.narrative_id(), "narr_789");
    assert_eq!(params.scene_name(), "Climax");
    assert_eq!(
        params.description(),
        &Some("The final confrontation".to_string())
    );

    tracing::info!("CreateSceneParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_list_scenes() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing list scenes");

    let server = BotticelliServer::builder().build();
    let params = ListScenesParams::new("narrative_999".to_string());

    let result = server.list_scenes(Parameters(params)).await?;
    let scenes = result.0;
    tracing::debug!(scene_count = scenes.scenes().len(), "Listed scenes");

    assert!(*scenes.success());
    assert_eq!(scenes.narrative_id(), "narrative_999");
    assert_eq!(scenes.scenes().len(), 0); // Placeholder returns empty list

    tracing::info!("List scenes test passed");
    Ok(())
}

#[tokio::test]
async fn test_list_scenes_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ListScenesParams serialization");

    let json_value = json!({
        "narrative_id": "narr_abc"
    });

    let params: ListScenesParams = serde_json::from_value(json_value)?;
    tracing::debug!(?params, "Deserialized params");

    assert_eq!(params.narrative_id(), "narr_abc");

    tracing::info!("ListScenesParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_update_scene() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing scene update");

    let server = BotticelliServer::builder().build();
    let updates = json!({
        "name": "Updated Scene Name",
        "description": "New description"
    });
    let params = UpdateSceneParams::new("scene_123".to_string(), updates.clone());

    let result = server.update_scene(Parameters(params)).await?;
    let updated = result.0;
    tracing::debug!(scene_id = %updated.scene_id(), "Updated scene");

    assert!(*updated.success());
    assert_eq!(updated.scene_id(), "scene_123");
    assert_eq!(updated.updates(), &updates);

    tracing::info!("Scene update test passed");
    Ok(())
}

#[tokio::test]
async fn test_update_scene_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing UpdateSceneParams serialization");

    let json_value = json!({
        "scene_id": "scene_xyz",
        "updates": {
            "characters": ["Alice", "Bob"],
            "location": "Forest"
        }
    });

    let params: UpdateSceneParams = serde_json::from_value(json_value)?;
    tracing::debug!(?params, "Deserialized params");

    assert_eq!(params.scene_id(), "scene_xyz");
    assert_eq!(&params.updates()["characters"][0], &"Alice");
    assert_eq!(&params.updates()["location"], &"Forest");

    tracing::info!("UpdateSceneParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_delete_scene() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing scene deletion");

    let server = BotticelliServer::builder().build();
    let params = DeleteSceneParams::new("scene_to_delete".to_string());

    let result = server.delete_scene(Parameters(params)).await?;
    let deleted = result.0;
    tracing::debug!(scene_id = %deleted.scene_id(), deleted = deleted.deleted(), "Deleted scene");

    assert!(*deleted.success());
    assert_eq!(deleted.scene_id(), "scene_to_delete");
    assert!(*deleted.deleted());

    tracing::info!("Scene deletion test passed");
    Ok(())
}

#[tokio::test]
async fn test_delete_scene_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing DeleteSceneParams serialization");

    let json_value = json!({
        "scene_id": "scene_del_123"
    });

    let params: DeleteSceneParams = serde_json::from_value(json_value)?;
    tracing::debug!(?params, "Deserialized params");

    assert_eq!(params.scene_id(), "scene_del_123");

    tracing::info!("DeleteSceneParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_scene_workflow() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing full scene workflow");

    let server = BotticelliServer::builder().build();

    // Create a scene
    let create_params = CreateSceneParams::new("workflow_narrative".to_string(), "Test Scene".to_string(), Some("Initial description".to_string()));
    let create_result = server.create_scene(Parameters(create_params)).await?;
    let created_scene = create_result.0;
    let scene_id = created_scene.scene_id().clone();
    tracing::debug!(scene_id = %scene_id, "Created scene");

    // Update the scene
    let update_params = UpdateSceneParams::new(scene_id.clone(), json!({"description": "Updated description"}));
    let update_result = server.update_scene(Parameters(update_params)).await?;
    tracing::debug!("Updated scene");
    assert!(*update_result.0.success());

    // Delete the scene
    let delete_params = DeleteSceneParams::new(scene_id);
    let delete_result = server.delete_scene(Parameters(delete_params)).await?;
    tracing::debug!("Deleted scene");
    assert!(*delete_result.0.success());

    tracing::info!("Scene workflow test passed");
    Ok(())
}
