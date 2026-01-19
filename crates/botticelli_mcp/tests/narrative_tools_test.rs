//! Tests for narrative generation tools (create, modify, save).

mod helpers;

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
fn test_create_narrative_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing CreateNarrativeParams serialization");

    let params = CreateNarrativeParams::new(
        "Fetch data then analyze it".to_string(),
        "test_narrative".to_string(),
        Some("gemini-2.0-flash-exp".to_string()),
        Some(0.7),
    );

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params");

    assert_eq!(json["description"], "Fetch data then analyze it");
    assert_eq!(json["name"], "test_narrative");
    assert_eq!(json["default_model"], "gemini-2.0-flash-exp");
    assert_eq!(json["default_temperature"], 0.7);

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[test]
fn test_create_narrative_params_optional_fields() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing CreateNarrativeParams with optional fields");
    let params =
        CreateNarrativeParams::new("Process data".to_string(), "simple".to_string(), None, None);

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params with optional fields");

    assert_eq!(json["description"], "Process data");
    assert_eq!(json["name"], "simple");
    assert!(json.get("default_model").is_none() || json["default_model"].is_null());

    tracing::info!("Optional fields test passed");
    Ok(())
}

#[test]
fn test_create_narrative_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing CreateNarrativeResult serialization");
    let result = CreateNarrativeResult::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        "# Generated\n[narrative]\nname = \"test\"\n".to_string(),
        serde_json::json!({"valid": true, "errors": [], "warnings": []}),
        "Created narrative 'test' with 1 act(s)".to_string(),
        vec!["Applied formatting improvements".to_string()],
        1,
    );

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert!(json["toml"].as_str().unwrap().contains("[narrative]"));
    assert!(
        json["toml_with_comments"]
            .as_str()
            .unwrap()
            .contains("# Generated")
    );
    assert_eq!(json["act_count"], 1);
    assert_eq!(json["auto_fixes_applied"].as_array().unwrap().len(), 1);

    tracing::info!("Result serialization test passed");
    Ok(())
}

// ===== CreateNarrative Tool Tests =====

#[tokio::test]
async fn test_create_narrative_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing create narrative basic");

    let server = BotticelliServer::builder().build()?;

    let params = CreateNarrativeParams::new(
        "Fetch data from API then analyze the results".to_string(),
        "data_analysis".to_string(),
        None,
        None,
    );

    let result = server.create_narrative(Parameters(params)).await?;
    let result: CreateNarrativeResult = result.0;

    // Check TOML structure
    assert!(result.toml().contains("[narrative]"));
    assert!(result.toml().contains("name = \"data_analysis\""));
    assert!(result.toml().contains("[toc]"));
    assert!(result.toml().contains("[acts]"));

    // Check comments version
    assert!(result.toml_with_comments().contains("# Generated"));

    // Check validation
    let validation: Value = result.validation().clone();
    assert_eq!(validation["valid"], true);

    // Should have at least 2 acts (fetch, analyze)
    assert!(
        *result.act_count() >= 2,
        "Expected at least 2 acts, got {}",
        result.act_count()
    );

    tracing::info!("Basic create narrative test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_narrative_with_defaults() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing create narrative with defaults");

    let server = BotticelliServer::builder().build()?;

    let params = CreateNarrativeParams::new(
        "Generate a report".to_string(),
        "report_generator".to_string(),
        Some("claude-3-5-sonnet-20241022".to_string()),
        Some(0.5),
    );

    let result = server.create_narrative(Parameters(params)).await?;
    let result: CreateNarrativeResult = result.0;

    let toml = result.toml();

    // Check that defaults are in the TOML
    assert!(toml.contains("model = \"claude-3-5-sonnet-20241022\""));
    assert!(toml.contains("temperature = 0.5"));

    tracing::info!("Create with defaults test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_narrative_invalid_name() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing create narrative with invalid name");

    let server = BotticelliServer::builder().build()?;

    let params = CreateNarrativeParams::new(
        "Do something".to_string(),
        "123-invalid-name!".to_string(),
        None,
        None,
    );

    let result = server.create_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with invalid name");
    if let Err(err) = result {
        assert!(err.message.contains("Invalid narrative name"));
    }

    tracing::info!("Invalid name test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_narrative_complex_description() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing create narrative with complex description");

    let server = BotticelliServer::builder().build()?;

    let params = CreateNarrativeParams::new(
        "First fetch user data, then process the data, and finally generate a summary report"
            .to_string(),
        "user_report".to_string(),
        None,
        None,
    );

    let result = server.create_narrative(Parameters(params)).await?;
    let result: CreateNarrativeResult = result.0;

    // Should extract multiple acts from "then" and "and" patterns
    assert!(
        *result.act_count() >= 2,
        "Should have extracted multiple acts"
    );

    // Check summary
    assert!(result.summary().contains("user_report"));
    assert!(result.summary().contains("act"));

    tracing::info!("Complex description test passed");
    Ok(())
}

// ===== ModifyNarrativeParams Tests =====

#[test]
fn test_modify_narrative_params_serialization() {
    let params = ModifyNarrativeParams::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        "add act that validates the data".to_string(),
        Some("/tmp/narrative.toml".to_string()),
    );

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
async fn test_modify_narrative_add_act() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative add act");

    let server = BotticelliServer::builder().build()?;

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams::new(
        original_toml.to_string(),
        "add act that validates the data".to_string(),
        None,
    );

    let result = server.modify_narrative(Parameters(params)).await?;

    let result: ModifyNarrativeResult = result.0;

    // Check that act was added
    assert!(result.toml().contains("validates"));
    assert!(
        result
            .changes()
            .iter()
            .any(|c: &String| c.contains("Added act"))
    );

    // Should still be valid
    let validation: Value = result.validation().clone();
    assert_eq!(validation["valid"], true);

    tracing::info!("Add act test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_narrative_remove_act() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative remove act");

    let server = BotticelliServer::builder().build()?;

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\", \"process\"]

[acts]
fetch = \"Get data\"
process = \"Process data\"
";

    let params = ModifyNarrativeParams::new(
        original_toml.to_string(),
        "remove act process".to_string(),
        None,
    );

    let result = server.modify_narrative(Parameters(params)).await?;

    let result: ModifyNarrativeResult = result.0;

    // Check that act was removed
    assert!(!result.toml().contains("process = \"Process data\""));
    assert!(
        result
            .changes()
            .iter()
            .any(|c: &String| c.contains("Removed act 'process'"))
    );

    tracing::info!("Remove act test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_narrative_change_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative change model");

    let server = BotticelliServer::builder().build()?;

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams::new(
        original_toml.to_string(),
        "use claude model".to_string(),
        None,
    );

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that model was added/changed
    assert!(
        result
            .toml()
            .contains("model = \"claude-3-5-sonnet-20241022\"")
    );
    assert!(
        result
            .changes()
            .iter()
            .any(|c: &String| c.contains("Changed model"))
    );

    tracing::info!("Change model test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_narrative_change_temperature() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative change temperature");

    let server = BotticelliServer::builder().build()?;

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams::new(
        original_toml.to_string(),
        "set temperature to 0.8".to_string(),
        None,
    );

    let result = server.modify_narrative(Parameters(params)).await?;

    let result: ModifyNarrativeResult = result.0;

    // Check that temperature was added/changed
    assert!(result.toml().contains("temperature = 0.8"));
    assert!(
        result
            .changes()
            .iter()
            .any(|c: &String| c.contains("Changed temperature"))
    );

    tracing::info!("Change temperature test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_narrative_unknown_modification() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative with unknown modification");

    let server = BotticelliServer::builder().build()?;

    let params = ModifyNarrativeParams::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        "do something completely unknown".to_string(),
        None,
    );

    let result = server.modify_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with unknown modification");
    if let Err(err) = result {
        assert!(err.message.contains("Could not understand modification"));
    }

    tracing::info!("Unknown modification test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_narrative_with_save() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing modify narrative with save");

    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new()?;
    let save_path = temp_dir.path().join("modified.toml");

    let original_toml = "\
[narrative]
name = \"test\"

[toc]
order = [\"fetch\"]

[acts]
fetch = \"Get data\"
";

    let params = ModifyNarrativeParams::new(
        original_toml.to_string(),
        "use gemini model".to_string(),
        Some(save_path.to_string_lossy().to_string()),
    );

    let result = server
        .modify_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: ModifyNarrativeResult = result.0;

    // Check that file was saved
    assert!(result.saved_to().is_some());
    assert!(save_path.exists(), "File should be created");

    // Verify file contents
    let contents = fs::read_to_string(&save_path)?;
    assert!(contents.contains("model = \"gemini-2.0-flash-exp\""));

    tracing::info!("Modify with save test passed");
    Ok(())
}

// ===== SaveNarrativeParams Tests =====

#[test]
fn test_save_narrative_params_serialization() {
    let params = SaveNarrativeParams::new(
        "[narrative]\nname = \"test\"\n".to_string(),
        "/tmp/test.toml".to_string(),
        true,
    );

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
    let params =
        SaveNarrativeParams::new("content".to_string(), "/tmp/test.toml".to_string(), false);

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
async fn test_save_narrative_basic() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
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

    let params = SaveNarrativeParams::new(
        narrative_toml.to_string(),
        file_path.to_string_lossy().to_string(),
        false,
    );

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: SaveNarrativeResult = result.0;

    // Check result
    assert_eq!(result.status(), "saved");
    assert_eq!(*result.size_bytes(), narrative_toml.len());
    assert!(!*result.overwritten());

    // Verify file was created
    assert!(file_path.exists(), "File should exist");

    // Verify contents
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, narrative_toml);
    Ok(())
}

#[tokio::test]
async fn test_save_narrative_invalid_extension() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;

    let params = SaveNarrativeParams::new(
        "content".to_string(),
        "/tmp/test.txt".to_string(), // Wrong extension
        false,
    );

    let result = server.save_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with invalid extension");
    if let Err(err) = result {
        assert!(err.message.contains(".toml extension"));
    }
    Ok(())
}

#[tokio::test]
async fn test_save_narrative_overwrite_protection() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("existing.toml");

    // Create existing file
    fs::write(&file_path, "old content").expect("Should write initial file");

    let params = SaveNarrativeParams::new(
        "new content".to_string(),
        file_path.to_string_lossy().to_string(),
        false, // Should fail
    );

    let result = server.save_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail without overwrite flag");
    if let Err(err) = result {
        assert!(err.message.contains("already exists"));
    }

    // Verify original file unchanged
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, "old content");
    Ok(())
}

#[tokio::test]
async fn test_save_narrative_overwrite_allowed() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("existing.toml");

    // Create existing file
    fs::write(&file_path, "old content").expect("Should write initial file");

    let params = SaveNarrativeParams::new(
        "new content".to_string(),
        file_path.to_string_lossy().to_string(),
        true, // Allow overwrite
    );

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed with overwrite=true");

    let result: SaveNarrativeResult = result.0;

    // Check that overwritten flag is set
    assert!(*result.overwritten());

    // Verify file was overwritten
    let contents = fs::read_to_string(&file_path).expect("Should read file");
    assert_eq!(contents, "new content");
    Ok(())
}

#[tokio::test]
async fn test_save_narrative_creates_directories() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("nested/directories/test.toml");

    // Ensure parent directories don't exist
    assert!(!file_path.parent().unwrap().exists());

    let params = SaveNarrativeParams::new(
        "content".to_string(),
        file_path.to_string_lossy().to_string(),
        false,
    );

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
    Ok(())
}

#[tokio::test]
async fn test_save_narrative_absolute_path() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file_path = temp_dir.path().join("test.toml");

    let params = SaveNarrativeParams::new(
        "content".to_string(),
        file_path.to_string_lossy().to_string(),
        false,
    );

    let result = server
        .save_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    let result: SaveNarrativeResult = result.0;

    // Result should contain absolute path
    let result_path = Path::new(result.file_path());
    assert!(
        result_path.is_absolute(),
        "Returned path should be absolute"
    );
    Ok(())
}

// ===== Integration Test: Full Workflow =====

#[tokio::test]
async fn test_narrative_workflow_create_modify_save() -> anyhow::Result<()> {
    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new().expect("Should create temp dir");

    // 1. Create a narrative
    let create_params = CreateNarrativeParams::new(
        "Fetch user data then generate a report".to_string(),
        "user_report".to_string(),
        Some("gemini-2.0-flash-exp".to_string()),
        Some(0.7),
    );

    let created = server
        .create_narrative(Parameters(create_params))
        .await
        .expect("Should create narrative")
        .0;

    assert!(created.validation()["valid"].as_bool().unwrap());
    let toml = created.toml().to_string();

    // 2. Modify the narrative
    let modify_params =
        ModifyNarrativeParams::new(toml, "add act that validates the data".to_string(), None);

    let modified = server
        .modify_narrative(Parameters(modify_params))
        .await
        .expect("Should modify narrative")
        .0;

    assert!(modified.validation()["valid"].as_bool().unwrap());
    assert!(
        modified
            .changes()
            .iter()
            .any(|c: &String| c.contains("Added act"))
    );

    // 3. Save the narrative
    let save_path = temp_dir.path().join("final_narrative.toml");
    let save_params = SaveNarrativeParams::new(
        modified.toml().clone(),
        save_path.to_string_lossy().to_string(),
        false,
    );

    let saved = server
        .save_narrative(Parameters(save_params))
        .await
        .expect("Should save narrative")
        .0;

    assert_eq!(saved.status(), "saved");
    assert!(save_path.exists());

    // Verify final file contains all expected elements
    let final_contents = fs::read_to_string(&save_path).expect("Should read final file");
    assert!(final_contents.contains("name = \"user_report\""));
    assert!(final_contents.contains("model = \"gemini-2.0-flash-exp\""));
    assert!(final_contents.contains("temperature = 0.7"));
    assert!(final_contents.contains("validates")); // Modified act
    Ok(())
}
