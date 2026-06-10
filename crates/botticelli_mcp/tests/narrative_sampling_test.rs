use botticelli_error::McpResult;
use botticelli_interface::RegistryOperations;
use botticelli_mcp::NarrativeRegistry;
use serde_json::{json, Value};

/// Test wrapper for JSON values that implements RegistryOperations.
#[derive(Debug, Clone)]
struct TestNarrativeState {
    id: String,
    data: Value,
}

impl RegistryOperations for TestNarrativeState {
    type Key = String;

    fn registry_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn from_json_args(args: Value) -> McpResult<Self> {
        let id = args.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("test")
            .to_string();
        Ok(Self { id, data: args })
    }

    fn to_json(&self) -> McpResult<Value> {
        Ok(self.data.clone())
    }

    fn update_from_json(&mut self, args: Value) -> McpResult<()> {
        // Merge args into data
        if let (Some(data_obj), Some(args_obj)) = (self.data.as_object_mut(), args.as_object()) {
            for (k, v) in args_obj {
                data_obj.insert(k.clone(), v.clone());
            }
        }
        Ok(())
    }
}

impl TestNarrativeState {
    fn new(id: String, data: Value) -> Self {
        Self { id, data }
    }
}

/// Test basic narrative creation workflow
#[test]
fn test_create_narrative_workflow() {
    let registry = NarrativeRegistry::new();

    // Create a narrative session
    let initial_state = TestNarrativeState::new(
        "test-1".to_string(),
        json!({
            "id": "test-1",
            "title": "Test Story",
            "description": "A test narrative",
            "acts": [],
            "inputs": []
        }),
    );

    let narrative_id = registry.create_session(initial_state.clone());

    // Verify we can retrieve it
    let retrieved = registry.get(&narrative_id);
    assert!(retrieved.is_ok());
    let retrieved_state = retrieved.unwrap();
    assert_eq!(retrieved_state.data["title"], "Test Story");

    // Verify active sessions
    let sessions = registry.list_keys();
    assert_eq!(sessions.len(), 1);
    assert!(sessions.contains(&narrative_id));
}

/// Test adding acts to narrative
#[test]
fn test_add_act_workflow() {
    let registry = NarrativeRegistry::new();

    // Create narrative with empty acts array
    let initial_data = json!({
        "title": "Test",
        "description": "Test",
        "acts": []
    });

    let initial_state = TestNarrativeState {
        id: "test-narrative".to_string(),
        data: initial_data,
    };

    let narrative_id = registry.create_session(initial_state);

    // Simulate adding an act
    let mut current_state = registry.get(&narrative_id).unwrap();
    let acts = current_state.data["acts"].as_array_mut().unwrap();
    acts.push(json!({
        "title": "Act 1",
        "description": "First act"
    }));

    registry
        .update(&narrative_id, current_state.to_json().expect("serialize state"))
        .expect("update succeeded");

    // Verify act was added
    let updated_state = registry.get(&narrative_id).unwrap();
    assert_eq!(updated_state.data["acts"].as_array().unwrap().len(), 1);
    assert_eq!(updated_state.data["acts"][0]["title"], "Act 1");
}

/// Test adding inputs to narrative
#[test]
fn test_add_input_workflow() {
    let registry = NarrativeRegistry::new();

    let initial_data = json!({
        "title": "Test",
        "description": "Test",
        "inputs": []
    });

    let initial_state = TestNarrativeState {
        id: "test-narrative".to_string(),
        data: initial_data,
    };

    let narrative_id = registry.create_session(initial_state);

    // Simulate adding a text input
    let mut current_state = registry.get(&narrative_id).unwrap();
    let inputs = current_state.data["inputs"].as_array_mut().unwrap();
    inputs.push(json!({
        "type": "text",
        "content": "Hello world"
    }));

    registry
        .update(&narrative_id, current_state.to_json().expect("serialize state"))
        .expect("update succeeded");

    // Verify input was added
    let updated_state = registry.get(&narrative_id).unwrap();
    assert_eq!(updated_state.data["inputs"].as_array().unwrap().len(), 1);
    assert_eq!(updated_state.data["inputs"][0]["content"], "Hello world");
}

/// Test validation workflow
#[test]
fn test_validation_workflow() {
    let registry = NarrativeRegistry::new();

    // Create narrative with required components
    let complete_data = json!({
        "title": "Test",
        "description": "Test",
        "acts": [
            {"title": "Act 1", "description": "First"}
        ],
        "inputs": [
            {"type": "text", "content": "Test"}
        ]
    });

    let complete_state = TestNarrativeState {
        id: "test-narrative".to_string(),
        data: complete_data,
    };

    let narrative_id = registry.create_session(complete_state);

    // Validate structure
    let state = registry.get(&narrative_id).unwrap();
    assert!(state.data.get("title").is_some());
    assert!(state.data.get("description").is_some());
    assert!(!state.data["acts"].as_array().unwrap().is_empty());
    assert!(!state.data["inputs"].as_array().unwrap().is_empty());
}

/// Test finalization workflow
#[test]
fn test_finalization_workflow() {
    let registry = NarrativeRegistry::new();

    let complete_state = TestNarrativeState::new(
        "complete".to_string(),
        json!({
            "title": "Complete Story",
            "description": "A complete test",
            "acts": [{"title": "Act 1", "description": "First"}],
            "inputs": [{"type": "text", "content": "Opening"}],
            "carousels": [{"title": "Scene 1", "prompt": "Describe"}]
        }),
    );

    let narrative_id = registry.create_session(complete_state);

    let finalized_state = registry.remove(&narrative_id);
    assert!(finalized_state.is_some());

    assert!(!registry.list_keys().contains(&narrative_id));

    let state = finalized_state.unwrap();
    assert_eq!(state.data["title"], "Complete Story");
    assert!(!state.data["acts"].as_array().unwrap().is_empty());
    assert!(!state.data["inputs"].as_array().unwrap().is_empty());
    assert!(!state.data["carousels"].as_array().unwrap().is_empty());
}

/// Test multi-session management
#[test]
fn test_multi_session_management() {
    let registry = NarrativeRegistry::new();

    let state1 = TestNarrativeState::new("s1".to_string(), json!({"title": "Story 1"}));
    let state2 = TestNarrativeState::new("s2".to_string(), json!({"title": "Story 2"}));
    let state3 = TestNarrativeState::new("s3".to_string(), json!({"title": "Story 3"}));

    let id1 = registry.create_session(state1);
    let id2 = registry.create_session(state2);
    let id3 = registry.create_session(state3);

    assert_eq!(registry.session_count(), 3);

    registry.remove(&id2);
    assert_eq!(registry.session_count(), 2);

    assert!(registry.get(&id1).is_ok());
    assert!(registry.get(&id2).is_err());
    assert!(registry.get(&id3).is_ok());

    registry.clear();
    assert_eq!(registry.session_count(), 0);
}
