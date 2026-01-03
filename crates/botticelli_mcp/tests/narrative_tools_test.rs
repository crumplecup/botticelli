//! Tests for narrative generation tools (create, modify, save).

use botticelli_mcp::{
    BotticelliServer, CreateNarrativeParams, CreateNarrativeResult, ModifyNarrativeParams,
    ModifyNarrativeResult, SaveNarrativeParams, SaveNarrativeResult,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ===== CreateNarrativeParams Tests =====

#[test]
fn test_create_narrative_params_serialization() {
    let params = CreateNarrativeParams {
        description: "Fetch data then analyze it".to_string(),
        name: "test_narrative".to_string(),
        default_model: Some("gemini-2.0-flash-exp".to_string()),
        default_temperature: Some(0.7),
    };

    let json = serde_json::to_value(&params).expect("Should serialize");

    assert_eq!(json["description"], "Fetch data then analyze it");
    assert_eq!(json["name"], "test_narrative");
    assert_eq!(json["default_model"], "gemini-2.0-flash-exp");
    assert_eq!(json["default_temperature"], 0.7);
}

#[test]
fn test_create_narrative_params_optional_fields() {
    let params = CreateNarrativeParams {
        description: "Process data".to_string(),
        name: "simple".to_string(),
        default_model: None,
        default_temperature: None,
    };

    let json = serde_json::to_value(&params).expect("Should serialize");

    assert_eq!(json["description"], "Process data");
    assert_eq!(json["name"], "simple");
    assert!(json.get("default_model").is_none() || json["default_model"].is_null());
}

#[test]
fn test_create_narrative_result_serialization() {
    let result = CreateNarrativeResult::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        "# Generated\n[narrative]\nname = \"test\"\n".to_string(),
        serde_json::json!({"valid": true, "errors": [], "warnings": []}),
        "Created narrative 'test' with 1 act(s)".to_string(),
        vec!["Applied formatting improvements".to_string()],
        1,
    );

    let json = serde_json::to_value(&result).expect("Should serialize");

    assert!(json["toml"].as_str().unwrap().contains("[narrative]"));
    assert!(
        json["toml_with_comments"]
            .as_str()
            .unwrap()
            .contains("# Generated")
    );
    assert_eq!(json["act_count"], 1);
    assert_eq!(json["auto_fixes_applied"].as_array().unwrap().len(), 1);
}

// ===== CreateNarrative Tool Tests =====

#[tokio::test]
async fn test_create_narrative_basic() {
    let server = BotticelliServer::builder().build();

    let params = CreateNarrativeParams {
        description: "Fetch data from API then analyze the results".to_string(),
        name: "data_analysis".to_string(),
        default_model: None,
        default_temperature: None,
    };

    let result = server
        .create_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: CreateNarrativeResult = result.0;

    // Check TOML structure
    assert!(result.toml.contains("[narrative]"));
    assert!(result.toml.contains("name = \"data_analysis\""));
    assert!(result.toml.contains("[toc]"));
    assert!(result.toml.contains("[acts]"));

    // Check comments version
    assert!(result.toml_with_comments.contains("# Generated"));

    // Check validation
    let validation: Value = result.validation;
    assert_eq!(validation["valid"], true);

    // Should have at least 2 acts (fetch, analyze)
    assert!(
        result.act_count >= 2,
        "Expected at least 2 acts, got {}",
        result.act_count
    );
}

#[tokio::test]
async fn test_create_narrative_with_defaults() {
    let server = BotticelliServer::builder().build();

    let params = CreateNarrativeParams {
        description: "Generate a report".to_string(),
        name: "report_generator".to_string(),
        default_model: Some("claude-3-5-sonnet-20241022".to_string()),
        default_temperature: Some(0.5),
    };

    let result = server
        .create_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let toml = &result.0.toml;

    // Check that defaults are in the TOML
    assert!(toml.contains("model = \"claude-3-5-sonnet-20241022\""));
    assert!(toml.contains("temperature = 0.5"));
}

#[tokio::test]
async fn test_create_narrative_invalid_name() {
    let server = BotticelliServer::builder().build();

    let params = CreateNarrativeParams {
        description: "Do something".to_string(),
        name: "123-invalid-name!".to_string(), // Invalid: starts with number, has hyphen and !
        default_model: None,
        default_temperature: None,
    };

    let result = server.create_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with invalid name");
    if let Err(err) = result {
        assert!(err.message.contains("Invalid narrative name"));
    }
}

#[tokio::test]
async fn test_create_narrative_complex_description() {
    let server = BotticelliServer::builder().build();

    let params = CreateNarrativeParams {
        description:
            "First fetch user data, then process the data, and finally generate a summary report"
                .to_string(),
        name: "user_report".to_string(),
        default_model: None,
        default_temperature: None,
    };

    let result = server
        .create_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    // Should extract multiple acts from "then" and "and" patterns
    assert!(
        result.0.act_count >= 2,
        "Should have extracted multiple acts"
    );

    // Check summary
    assert!(result.0.summary.contains("user_report"));
    assert!(result.0.summary.contains("act"));
}

// ===== ModifyNarrativeParams Tests =====

#[test]
fn test_modify_narrative_params_serialization() {
    let params = ModifyNarrativeParams {
        narrative_toml: "[narrative]\nname = \"test\"\n".to_string(),
        modification: "add act that validates the data".to_string(),
        save_to: Some("/tmp/narrative.toml".to_string()),
    };

    let json = serde_json::to_value(&params).expect("Should serialize");

    assert!(
        json["narrative_toml"]
            .as_str()
            .unwrap()
            .contains("[narrative]")
    );
    assert_eq!(json["modification"], "add act that validates the data");
    assert_eq!(json["save_to"], "/tmp/narrative.toml");
}

#[test]
fn test_modify_narrative_result_serialization() {
    let result = ModifyNarrativeResult::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        serde_json::json!({"valid": true}),
        vec!["Added act 'validate'".to_string()],
        Some("/tmp/narrative.toml".to_string()),
    );

    let json = serde_json::to_value(&result).expect("Should serialize");

    assert!(json["toml"].as_str().unwrap().contains("[narrative]"));
    assert_eq!(json["changes"].as_array().unwrap().len(), 1);
    assert_eq!(json["saved_to"], "/tmp/narrative.toml");
}

// ===== ModifyNarrative Tool Tests =====

#[tokio::test]
async fn test_modify_narrative_add_act() {
    let server = BotticelliServer::builder().build();

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams {
        narrative_toml: original_toml.to_string(),
        modification: "add act that validates the data".to_string(),
        save_to: None,
    };

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that act was added
    assert!(result.toml.contains("validates"));
    assert!(result.changes.iter().any(|c| c.contains("Added act")));

    // Should still be valid
    let validation: Value = result.validation;
    assert_eq!(validation["valid"], true);
}

#[tokio::test]
async fn test_modify_narrative_remove_act() {
    let server = BotticelliServer::builder().build();

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\", \"process\"]

[acts]
fetch = \"Get data\"
process = \"Process data\"
";

    let params = ModifyNarrativeParams {
        narrative_toml: original_toml.to_string(),
        modification: "remove act process".to_string(),
        save_to: None,
    };

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that act was removed
    assert!(!result.toml.contains("process = \"Process data\""));
    assert!(
        result
            .changes
            .iter()
            .any(|c| c.contains("Removed act 'process'"))
    );
}

#[tokio::test]
async fn test_modify_narrative_change_model() {
    let server = BotticelliServer::builder().build();

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams {
        narrative_toml: original_toml.to_string(),
        modification: "use claude model".to_string(),
        save_to: None,
    };

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that model was added/changed
    assert!(
        result
            .toml
            .contains("model = \"claude-3-5-sonnet-20241022\"")
    );
    assert!(result.changes.iter().any(|c| c.contains("Changed model")));
}

#[tokio::test]
async fn test_modify_narrative_change_temperature() {
    let server = BotticelliServer::builder().build();

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams {
        narrative_toml: original_toml.to_string(),
        modification: "set temperature to 0.8".to_string(),
        save_to: None,
    };

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that temperature was added/changed
    assert!(result.toml.contains("temperature = 0.8"));
    assert!(
        result
            .changes
            .iter()
            .any(|c| c.contains("Changed temperature"))
    );
}

#[tokio::test]
async fn test_modify_narrative_unknown_modification() {
    let server = BotticelliServer::builder().build();

    let params = ModifyNarrativeParams {
        narrative_toml: "[narrative]\nname = \"test\"\n".to_string(),
        modification: "do something completely unknown".to_string(),
        save_to: None,
    };

    let result = server.modify_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with unknown modification");
    if let Err(err) = result {
        assert!(err.message.contains("Could not understand modification"));
    }
}

#[tokio::test]
async fn test_modify_narrative_with_save() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let save_path = temp_dir.path().join("modified.toml");

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams {
        narrative_toml: original_toml.to_string(),
        modification: "use gemini model".to_string(),
        save_to: Some(save_path.to_string_lossy().to_string()),
    };

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that file was saved
    assert!(result.saved_to.is_some());
    assert!(save_path.exists(), "File should be created");

    // Verify file contents
    let contents = fs::read_to_string(&save_path).expect("Should read file");
    assert!(contents.contains("model = \"gemini-2.0-flash-exp\""));
}

// ===== SaveNarrativeParams Tests =====

#[test]
fn test_save_narrative_params_serialization() {
    let params = SaveNarrativeParams {
        narrative_toml: "[narrative]\nname = \"test\"\n".to_string(),
        file_path: "/tmp/test.toml".to_string(),
        overwrite: true,
    };

    let json = serde_json::to_value(&params).expect("Should serialize");

    assert!(
        json["narrative_toml"]
            .as_str()
            .unwrap()
            .contains("[narrative]")
    );
    assert_eq!(json["file_path"], "/tmp/test.toml");
    assert_eq!(json["overwrite"], true);
}

#[test]
fn test_save_narrative_params_default_overwrite() {
    let params = SaveNarrativeParams {
        narrative_toml: "content".to_string(),
        file_path: "/tmp/test.toml".to_string(),
        overwrite: false,
    };

    let json = serde_json::to_value(&params).expect("Should serialize");

    // Default should be false
    assert_eq!(json["overwrite"], false);
}

#[test]
fn test_save_narrative_result_serialization() {
    let result = SaveNarrativeResult::new("/tmp/test.toml".to_string(), 1234, false);

    let json = serde_json::to_value(&result).expect("Should serialize");

    assert_eq!(json["status"], "saved");
    assert_eq!(json["file_path"], "/tmp/test.toml");
    assert_eq!(json["size_bytes"], 1234);
    assert_eq!(json["overwritten"], false);
}

// ===== SaveNarrative Tool Tests =====

#[tokio::test]
async fn test_save_narrative_basic() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("test.toml");

    let narrative_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = SaveNarrativeParams {
        narrative_toml: narrative_toml.to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        overwrite: false,
    };

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: SaveNarrativeResult = result.0;

    // Check result
    assert_eq!(result.status, "saved");
    assert_eq!(result.size_bytes, narrative_toml.len());
    assert!(!result.overwritten);

    // Verify file was created
    assert!(file_path.exists(), "File should exist");

    // Verify contents
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, narrative_toml);
}

#[tokio::test]
async fn test_save_narrative_invalid_extension() {
    let server = BotticelliServer::builder().build();

    let params = SaveNarrativeParams {
        narrative_toml: "content".to_string(),
        file_path: "/tmp/test.txt".to_string(), // Wrong extension
        overwrite: false,
    };

    let result = server.save_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with invalid extension");
    if let Err(err) = result {
        assert!(err.message.contains(".toml extension"));
    }
}

#[tokio::test]
async fn test_save_narrative_overwrite_protection() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("existing.toml");

    // Create existing file
    fs::write(&file_path, "old content").expect("Should write initial file");

    let params = SaveNarrativeParams {
        narrative_toml: "new content".to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        overwrite: false, // Should fail
    };

    let result = server.save_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail without overwrite flag");
    if let Err(err) = result {
        assert!(err.message.contains("already exists"));
    }

    // Verify original file unchanged
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, "old content");
}

#[tokio::test]
async fn test_save_narrative_overwrite_allowed() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("existing.toml");

    // Create existing file
    fs::write(&file_path, "old content").expect("Should write initial file");

    let params = SaveNarrativeParams {
        narrative_toml: "new content".to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        overwrite: true, // Allow overwrite
    };

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed with overwrite=true");

    let result: SaveNarrativeResult = result.0;

    // Check that overwritten flag is set
    assert!(result.overwritten);

    // Verify file was overwritten
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, "new content");
}

#[tokio::test]
async fn test_save_narrative_creates_directories() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("nested/directories/test.toml");

    // Ensure parent directories don't exist
    assert!(!file_path.parent().unwrap().exists());

    let params = SaveNarrativeParams {
        narrative_toml: "content".to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        overwrite: false,
    };

    let _result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed and create directories");

    // Verify directories were created
    assert!(
        file_path.parent().unwrap().exists(),
        "Parent directories should exist"
    );
    assert!(file_path.exists(), "File should exist");
}

#[tokio::test]
async fn test_save_narrative_absolute_path() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("test.toml");

    let params = SaveNarrativeParams {
        narrative_toml: "content".to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        overwrite: false,
    };

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: SaveNarrativeResult = result.0;

    // Result should contain absolute path
    let result_path = Path::new(&result.file_path);
    assert!(
        result_path.is_absolute(),
        "Returned path should be absolute"
    );
}

// ===== Integration Test: Full Workflow =====

#[tokio::test]
async fn test_narrative_workflow_create_modify_save() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().expect("Should create temp dir");

    // 1. Create a narrative
    let create_params = CreateNarrativeParams {
        description: "Fetch user data then generate a report".to_string(),
        name: "user_report".to_string(),
        default_model: Some("gemini-2.0-flash-exp".to_string()),
        default_temperature: Some(0.7),
    };

    let created = server
        .create_narrative(Parameters(create_params))
        .await
        .expect("Should create narrative")
        .0;

    assert!(created.validation["valid"].as_bool().unwrap());
    let toml = created.toml;

    // 2. Modify the narrative
    let modify_params = ModifyNarrativeParams {
        narrative_toml: toml,
        modification: "add act that validates the data".to_string(),
        save_to: None,
    };

    let modified = server
        .modify_narrative(Parameters(modify_params))
        .await
        .expect("Should modify narrative")
        .0;

    assert!(modified.validation["valid"].as_bool().unwrap());
    assert!(modified.changes.iter().any(|c| c.contains("Added act")));

    // 3. Save the narrative
    let save_path = temp_dir.path().join("final_narrative.toml");
    let save_params = SaveNarrativeParams {
        narrative_toml: modified.toml.clone(),
        file_path: save_path.to_string_lossy().to_string(),
        overwrite: false,
    };

    let saved = server
        .save_narrative(Parameters(save_params))
        .await
        .expect("Should save narrative")
        .0;

    assert_eq!(saved.status, "saved");
    assert!(save_path.exists());

    // Verify final file contains all expected elements
    let final_contents = fs::read_to_string(&save_path).expect("Should read final file");
    assert!(final_contents.contains("name = \"user_report\""));
    assert!(final_contents.contains("model = \"gemini-2.0-flash-exp\""));
    assert!(final_contents.contains("temperature = 0.7"));
    assert!(final_contents.contains("validates")); // Modified act
}
