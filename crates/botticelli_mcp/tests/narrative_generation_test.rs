//! Integration tests for narrative generation tools.

mod helpers;

use botticelli_mcp::ToolRegistry;
use serde_json::json;

#[tokio::test]
async fn test_create_simple_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing simple narrative creation");

    let registry = ToolRegistry::default();

    let input = json!({
        "description": "Analyze user feedback and generate a summary",
        "name": "feedback_analysis"
    });

    let result = registry.execute("create_narrative", input).await?;
    tracing::debug!(?result, "Received result");

    // Check structure
    assert!(result.get("toml").is_some(), "Missing TOML output");
    assert!(result.get("validation").is_some(), "Missing validation");
    assert!(result.get("summary").is_some(), "Missing summary");

    // Check validation
    let validation = result.get("validation").unwrap();
    assert!(
        validation.get("valid").unwrap().as_bool().unwrap(),
        "Generated narrative should be valid"
    );

    // Check TOML contains required sections
    let toml = result.get("toml").unwrap().as_str().unwrap();
    assert!(toml.contains("[narrative]"), "Missing [narrative] section");
    assert!(toml.contains("[toc]"), "Missing [toc] section");
    assert!(toml.contains("[acts]"), "Missing [acts] section");
    assert!(
        toml.contains("name = \"feedback_analysis\""),
        "Missing narrative name"
    );

    tracing::info!("Simple narrative creation test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_with_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative creation with custom model");

    let registry = ToolRegistry::default();

    let input = json!({
        "description": "Fetch data, analyze it, and post results",
        "name": "data_pipeline",
        "default_model": "gemini-2.0-flash-exp",
        "default_temperature": 0.7
    });

    let result = registry.execute("create_narrative", input).await?;
    tracing::debug!(?result, "Received result with custom model");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    assert!(
        toml.contains("model = \"gemini-2.0-flash-exp\""),
        "Missing model configuration"
    );
    assert!(
        toml.contains("temperature = 0.7"),
        "Missing temperature configuration"
    );

    tracing::info!("Custom model configuration test passed");
    Ok(())
}

#[tokio::test]
async fn test_create_multiple_acts() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative with multiple acts");

    let registry = ToolRegistry::default();

    let input = json!({
        "description": "Fetch data, then analyze it, then generate report, then send email",
        "name": "reporting_workflow"
    });

    let result = registry.execute("create_narrative", input).await?;
    tracing::debug!(?result, "Received multi-act narrative");

    let toml = result.get("toml").unwrap().as_str().unwrap();

    // Should detect "then" pattern and create multiple acts
    assert!(toml.contains("fetch"), "Missing fetch act");
    assert!(toml.contains("analyze"), "Missing analyze act");
    assert!(
        toml.contains("generate") || toml.contains("report"),
        "Missing report act"
    );

    tracing::info!("Multiple acts test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_change_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative modification - change model");

    let registry = ToolRegistry::default();

    let existing_toml = r#"[narrative]
name = "test"
description = "Test narrative"
model = "gemini-2.0-flash-exp"

[toc]
order = ["analyze"]

[acts]
analyze = "Analyze the data"
"#;

    let input = json!({
        "narrative_toml": existing_toml,
        "modification": "Change model to Claude"
    });

    let result = registry.execute("modify_narrative", input).await?;
    tracing::debug!(?result, "Received modified narrative");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    assert!(
        toml.contains("claude-3-5-sonnet"),
        "Model should be changed to Claude"
    );

    let changes = result.get("changes").unwrap().as_array().unwrap();
    assert!(!changes.is_empty(), "Should report changes");

    tracing::info!("Model change test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_add_act() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative modification - add act");

    let registry = ToolRegistry::default();

    let existing_toml = r#"[narrative]
name = "test"
description = "Test narrative"

[toc]
order = ["analyze"]

[acts]
analyze = "Analyze the data"
"#;

    let input = json!({
        "narrative_toml": existing_toml,
        "modification": "Add act that summarizes the results"
    });

    let result = registry.execute("modify_narrative", input).await?;
    tracing::debug!(?result, "Received modified narrative with new act");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    assert!(toml.contains("summarizes"), "New act should be added");

    let changes = result.get("changes").unwrap().as_array().unwrap();
    assert!(!changes.is_empty(), "Should report changes");

    // Check that at least one change is about adding the act
    let has_add_act = changes
        .iter()
        .any(|change| change.as_str().unwrap().contains("Added act"));
    assert!(has_add_act, "Should indicate act was added");

    tracing::info!("Add act test passed");
    Ok(())
}

#[tokio::test]
async fn test_modify_set_temperature() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative modification - set temperature");

    let registry = ToolRegistry::default();

    let existing_toml = r#"[narrative]
name = "test"
description = "Test narrative"

[toc]
order = ["analyze"]

[acts]
analyze = "Analyze the data"
"#;

    let input = json!({
        "narrative_toml": existing_toml,
        "modification": "Set temperature to 0.3"
    });

    let result = registry.execute("modify_narrative", input).await?;
    tracing::debug!(?result, "Received modified narrative with temperature");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    assert!(
        toml.contains("temperature = 0.3"),
        "Temperature should be set"
    );

    tracing::info!("Set temperature test passed");
    Ok(())
}

#[tokio::test]
async fn test_save_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative save");

    let registry = ToolRegistry::default();

    let toml_content = r#"[narrative]
name = "test_save"
description = "Test saving"

[toc]
order = ["analyze"]

[acts]
analyze = "Analyze"
"#;

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_narrative.toml");
    let file_path_str = file_path.to_str().unwrap();

    // Clean up if exists
    let _ = std::fs::remove_file(&file_path);

    let input = json!({
        "narrative_toml": toml_content,
        "file_path": file_path_str,
        "overwrite": false
    });

    let result = registry.execute("save_narrative", input).await?;
    tracing::debug!(?result, "Received save result");

    assert_eq!(
        result.get("status").unwrap().as_str().unwrap(),
        "saved",
        "Should indicate file was saved"
    );

    // Verify file exists and contains correct content
    let saved_content = std::fs::read_to_string(&file_path).expect("File should exist");
    assert_eq!(saved_content, toml_content, "Saved content should match");

    // Clean up
    std::fs::remove_file(&file_path).ok();

    tracing::info!("Save narrative test passed");
    Ok(())
}

#[tokio::test]
async fn test_save_rejects_non_toml_extension() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing save rejection of non-TOML extension");

    let registry = ToolRegistry::default();

    let input = json!({
        "narrative_toml": "[narrative]\nname = \"test\"",
        "file_path": "/tmp/test.txt"
    });

    let result = registry.execute("save_narrative", input).await;
    assert!(result.is_err(), "Should reject non-.toml extension");

    tracing::info!("Non-TOML extension rejection test passed");
    Ok(())
}

#[tokio::test]
async fn test_save_prevents_overwrite_by_default() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing save prevents overwrite by default");

    let registry = ToolRegistry::default();

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_overwrite.toml");
    let file_path_str = file_path.to_str().unwrap();

    // Create initial file
    std::fs::write(&file_path, "existing content").expect("Failed to create test file");

    let input = json!({
        "narrative_toml": "[narrative]\nname = \"new\"",
        "file_path": file_path_str,
        "overwrite": false
    });

    let result = registry.execute("save_narrative", input).await;
    assert!(
        result.is_err(),
        "Should prevent overwriting existing file without permission"
    );

    // Clean up
    std::fs::remove_file(&file_path).ok();

    tracing::info!("Overwrite prevention test passed");
    Ok(())
}

#[tokio::test]
async fn test_full_workflow() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing full workflow: create → modify → save");

    let registry = ToolRegistry::default();

    // Step 1: Create narrative
    let create_input = json!({
        "description": "Fetch user data and analyze trends",
        "name": "user_trends"
    });

    let create_result = registry.execute("create_narrative", create_input).await?;
    tracing::debug!(?create_result, "Created narrative");
    let narrative_v1 = create_result.get("toml").unwrap().as_str().unwrap();

    // Step 2: Modify - add model
    let modify_input = json!({
        "narrative_toml": narrative_v1,
        "modification": "Use Gemini model"
    });

    let modify_result = registry.execute("modify_narrative", modify_input).await?;
    tracing::debug!(?modify_result, "Modified narrative");
    let narrative_v2 = modify_result.get("toml").unwrap().as_str().unwrap();

    // Verify modification applied
    assert!(
        narrative_v2.contains("gemini"),
        "Model should be set to Gemini"
    );

    // Step 3: Save
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("user_trends.toml");
    let file_path_str = file_path.to_str().unwrap();

    let _ = std::fs::remove_file(&file_path); // Clean up if exists

    let save_input = json!({
        "narrative_toml": narrative_v2,
        "file_path": file_path_str
    });

    registry.execute("save_narrative", save_input).await?;
    tracing::debug!("Saved narrative to file");

    // Verify file exists and is valid
    assert!(file_path.exists(), "File should be created");
    let saved = std::fs::read_to_string(&file_path).expect("Should read saved file");
    assert!(
        saved.contains("user_trends"),
        "Should contain narrative name"
    );

    // Clean up
    std::fs::remove_file(&file_path).ok();

    tracing::info!("Full workflow test passed");
    Ok(())
}
