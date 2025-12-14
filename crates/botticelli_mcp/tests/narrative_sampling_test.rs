use botticelli_mcp::NarrativeRegistry;
use serde_json::json;

/// Test basic narrative creation workflow
#[test]
fn test_create_narrative_workflow() {
    let registry = NarrativeRegistry::new();

    // Create a narrative session
    let initial_state = json!({
        "title": "Test Story",
        "description": "A test narrative",
        "acts": [],
        "inputs": []
    });

    let narrative_id = registry.create_session(initial_state.clone());

    // Verify we can retrieve it
    let retrieved = registry.get(&narrative_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap()["title"], "Test Story");

    // Verify active sessions
    let sessions = registry.active_sessions();
    assert_eq!(sessions.len(), 1);
    assert!(sessions.contains(&narrative_id));
}

/// Test adding acts to narrative
#[test]
fn test_add_act_workflow() {
    let registry = NarrativeRegistry::new();

    // Create narrative with empty acts array
    let initial_state = json!({
        "title": "Test",
        "description": "Test",
        "acts": []
    });

    let narrative_id = registry.create_session(initial_state);

    // Simulate adding an act
    let mut current_state = registry.get(&narrative_id).unwrap();
    let acts = current_state["acts"].as_array_mut().unwrap();
    acts.push(json!({
        "title": "Act 1",
        "description": "First act"
    }));

    registry.update(&narrative_id, current_state.clone());

    // Verify act was added
    let updated_state = registry.get(&narrative_id).unwrap();
    assert_eq!(updated_state["acts"].as_array().unwrap().len(), 1);
    assert_eq!(updated_state["acts"][0]["title"], "Act 1");
}

/// Test adding inputs to narrative
#[test]
fn test_add_input_workflow() {
    let registry = NarrativeRegistry::new();

    let initial_state = json!({
        "title": "Test",
        "description": "Test",
        "inputs": []
    });

    let narrative_id = registry.create_session(initial_state);

    // Simulate adding a text input
    let mut current_state = registry.get(&narrative_id).unwrap();
    let inputs = current_state["inputs"].as_array_mut().unwrap();
    inputs.push(json!({
        "type": "text",
        "content": "Hello world"
    }));

    registry.update(&narrative_id, current_state.clone());

    // Verify input was added
    let updated_state = registry.get(&narrative_id).unwrap();
    assert_eq!(updated_state["inputs"].as_array().unwrap().len(), 1);
    assert_eq!(updated_state["inputs"][0]["content"], "Hello world");
}

/// Test validation workflow
#[test]
fn test_validation_workflow() {
    let registry = NarrativeRegistry::new();

    // Create narrative with required components
    let complete_state = json!({
        "title": "Test",
        "description": "Test",
        "acts": [
            {"title": "Act 1", "description": "First"}
        ],
        "inputs": [
            {"type": "text", "content": "Test"}
        ]
    });

    let narrative_id = registry.create_session(complete_state);

    // Validate structure
    let state = registry.get(&narrative_id).unwrap();
    assert!(state.get("title").is_some());
    assert!(state.get("description").is_some());
    assert!(state["acts"].as_array().unwrap().len() > 0);
    assert!(state["inputs"].as_array().unwrap().len() > 0);
}

/// Test finalization workflow
#[test]
fn test_finalization_workflow() {
    let registry = NarrativeRegistry::new();

    // Create complete narrative ready for finalization
    let complete_state = json!({
        "title": "Complete Story",
        "description": "A complete test",
        "acts": [
            {"title": "Act 1", "description": "First"}
        ],
        "inputs": [
            {"type": "text", "content": "Opening"}
        ],
        "carousels": [
            {"title": "Scene 1", "prompt": "Describe"}
        ]
    });

    let narrative_id = registry.create_session(complete_state);

    // Simulate finalization by removing from registry
    let finalized_state = registry.remove(&narrative_id);
    assert!(finalized_state.is_some());

    // Verify it's no longer in active sessions
    assert!(!registry.active_sessions().contains(&narrative_id));

    // Verify the state has all required components
    let state = finalized_state.unwrap();
    assert_eq!(state["title"], "Complete Story");
    assert!(state["acts"].as_array().unwrap().len() > 0);
    assert!(state["inputs"].as_array().unwrap().len() > 0);
    assert!(state["carousels"].as_array().unwrap().len() > 0);
}

/// Test multi-session management
#[test]
fn test_multi_session_management() {
    let registry = NarrativeRegistry::new();

    let state1 = json!({"title": "Story 1"});
    let state2 = json!({"title": "Story 2"});
    let state3 = json!({"title": "Story 3"});

    let id1 = registry.create_session(state1);
    let id2 = registry.create_session(state2);
    let id3 = registry.create_session(state3);

    // Verify all sessions exist
    assert_eq!(registry.active_sessions().len(), 3);

    // Remove one session
    registry.remove(&id2);
    assert_eq!(registry.active_sessions().len(), 2);

    // Verify remaining sessions
    assert!(registry.get(&id1).is_some());
    assert!(registry.get(&id2).is_none());
    assert!(registry.get(&id3).is_some());

    // Clear all
    registry.clear();
    assert_eq!(registry.active_sessions().len(), 0);
}
