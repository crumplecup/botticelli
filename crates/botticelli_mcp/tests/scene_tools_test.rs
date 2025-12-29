//! Tests for scene management tools.

use botticelli_mcp::{
    BotticelliServer, CreateSceneParams, DeleteSceneParams, ListScenesParams, UpdateSceneParams,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

#[tokio::test]
async fn test_create_scene_with_description() {
    let server = BotticelliServer::builder().build();
    let params = CreateSceneParams {
        narrative_id: "narrative_123".to_string(),
        scene_name: "Opening Scene".to_string(),
        description: Some("The hero awakens in a strange land".to_string()),
    };

    let result = server.create_scene(Parameters(params)).await;

    assert!(result.is_ok(), "create_scene should succeed");
    let scene = result.unwrap().0;
    assert!(scene.success);
    assert_eq!(scene.narrative_id, "narrative_123");
    assert_eq!(scene.name, "Opening Scene");
    assert_eq!(
        scene.description,
        Some("The hero awakens in a strange land".to_string())
    );
    assert!(scene.scene_id.starts_with("scene_"));
}

#[tokio::test]
async fn test_create_scene_without_description() {
    let server = BotticelliServer::builder().build();
    let params = CreateSceneParams {
        narrative_id: "narrative_456".to_string(),
        scene_name: "Battle Scene".to_string(),
        description: None,
    };

    let result = server.create_scene(Parameters(params)).await;

    assert!(result.is_ok(), "create_scene should succeed");
    let scene = result.unwrap().0;
    assert!(scene.success);
    assert_eq!(scene.narrative_id, "narrative_456");
    assert_eq!(scene.name, "Battle Scene");
    assert_eq!(scene.description, None);
}

#[tokio::test]
async fn test_create_scene_params_serialization() {
    let json_value = json!({
        "narrative_id": "narr_789",
        "scene_name": "Climax",
        "description": "The final confrontation"
    });

    let params: CreateSceneParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.narrative_id, "narr_789");
    assert_eq!(params.scene_name, "Climax");
    assert_eq!(
        params.description,
        Some("The final confrontation".to_string())
    );
}

#[tokio::test]
async fn test_list_scenes() {
    let server = BotticelliServer::builder().build();
    let params = ListScenesParams {
        narrative_id: "narrative_999".to_string(),
    };

    let result = server.list_scenes(Parameters(params)).await;

    assert!(result.is_ok(), "list_scenes should succeed");
    let scenes = result.unwrap().0;
    assert!(scenes.success);
    assert_eq!(scenes.narrative_id, "narrative_999");
    assert_eq!(scenes.scenes.len(), 0); // Placeholder returns empty list
}

#[tokio::test]
async fn test_list_scenes_params_serialization() {
    let json_value = json!({
        "narrative_id": "narr_abc"
    });

    let params: ListScenesParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.narrative_id, "narr_abc");
}

#[tokio::test]
async fn test_update_scene() {
    let server = BotticelliServer::builder().build();
    let updates = json!({
        "name": "Updated Scene Name",
        "description": "New description"
    });
    let params = UpdateSceneParams {
        scene_id: "scene_123".to_string(),
        updates: updates.clone(),
    };

    let result = server.update_scene(Parameters(params)).await;

    assert!(result.is_ok(), "update_scene should succeed");
    let updated = result.unwrap().0;
    assert!(updated.success);
    assert_eq!(updated.scene_id, "scene_123");
    assert_eq!(updated.updates, updates);
}

#[tokio::test]
async fn test_update_scene_params_serialization() {
    let json_value = json!({
        "scene_id": "scene_xyz",
        "updates": {
            "characters": ["Alice", "Bob"],
            "location": "Forest"
        }
    });

    let params: UpdateSceneParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.scene_id, "scene_xyz");
    assert_eq!(params.updates["characters"][0], "Alice");
    assert_eq!(params.updates["location"], "Forest");
}

#[tokio::test]
async fn test_delete_scene() {
    let server = BotticelliServer::builder().build();
    let params = DeleteSceneParams {
        scene_id: "scene_to_delete".to_string(),
    };

    let result = server.delete_scene(Parameters(params)).await;

    assert!(result.is_ok(), "delete_scene should succeed");
    let deleted = result.unwrap().0;
    assert!(deleted.success);
    assert_eq!(deleted.scene_id, "scene_to_delete");
    assert!(deleted.deleted);
}

#[tokio::test]
async fn test_delete_scene_params_serialization() {
    let json_value = json!({
        "scene_id": "scene_del_123"
    });

    let params: DeleteSceneParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.scene_id, "scene_del_123");
}

#[tokio::test]
async fn test_scene_workflow() {
    let server = BotticelliServer::builder().build();

    // Create a scene
    let create_params = CreateSceneParams {
        narrative_id: "workflow_narrative".to_string(),
        scene_name: "Test Scene".to_string(),
        description: Some("Initial description".to_string()),
    };
    let create_result = server.create_scene(Parameters(create_params)).await;
    assert!(create_result.is_ok());
    let created_scene = create_result.unwrap().0;
    let scene_id = created_scene.scene_id.clone();

    // Update the scene
    let update_params = UpdateSceneParams {
        scene_id: scene_id.clone(),
        updates: json!({"description": "Updated description"}),
    };
    let update_result = server.update_scene(Parameters(update_params)).await;
    assert!(update_result.is_ok());

    // Delete the scene
    let delete_params = DeleteSceneParams { scene_id };
    let delete_result = server.delete_scene(Parameters(delete_params)).await;
    assert!(delete_result.is_ok());
}
