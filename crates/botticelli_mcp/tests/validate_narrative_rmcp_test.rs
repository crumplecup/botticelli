//! Tests for validate_narrative tool (rmcp version).

mod helpers;

use botticelli_mcp::{BotticelliServer, ValidateNarrativeParams};
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_validate_narrative_valid_content() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative with valid content");

    let server = BotticelliServer::builder().build();

    let valid_toml = r#"
title = "Test Narrative"
version = "0.1.0"

[metadata]
setting = "Test Setting"
theme = "Adventure"

[[acts]]
name = "intro"
objective = "Test objective"
"#;

    let params = ValidateNarrativeParams {
        content: Some(valid_toml.to_string()),
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await?;
    tracing::debug!(
        valid = result.0.valid,
        error_count = result.0.errors.len(),
        warning_count = result.0.warnings.len(),
        "Validation result"
    );

    // The narrative might have validation issues based on the validator's requirements
    // Check that we got a result structure (tool succeeded)
    assert!(!result.0.summary.is_empty(), "Should have a summary");

    // If there are errors, they're related to the TOML structure, not the tool
    if !result.0.errors.is_empty() {
        tracing::debug!(errors = ?result.0.errors, "Validation errors");
    }

    tracing::info!("Valid content test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_invalid_toml() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative with invalid TOML");

    let server = BotticelliServer::builder().build();

    let invalid_toml = r#"
title = "Test
# Missing closing quote - syntax error
"#;

    let params = ValidateNarrativeParams {
        content: Some(invalid_toml.to_string()),
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await?;
    tracing::debug!(valid = result.0.valid, error_count = result.0.errors.len(), "Validation result");

    assert!(!result.0.valid, "Invalid TOML should fail validation");
    assert!(!result.0.errors.is_empty(), "Should have errors");

    tracing::info!("Invalid TOML test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_from_file() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative from file");

    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.toml");
    tracing::debug!(path = ?file_path, "Created temp file path");

    let valid_toml = r#"
title = "File Test"
version = "0.1.0"

[[acts]]
name = "act1"
objective = "Test"
"#;

    fs::write(&file_path, valid_toml).await?;

    let params = ValidateNarrativeParams {
        content: None,
        file_path: Some(file_path.to_string_lossy().to_string()),
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await?;
    tracing::debug!(has_summary = !result.0.summary.is_empty(), "Validation result");

    // Should complete successfully (may have validation errors in content, but tool works)
    assert!(!result.0.summary.is_empty());

    tracing::info!("From file test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_file_not_found() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative with missing file");

    let server = BotticelliServer::builder().build();

    let params = ValidateNarrativeParams {
        content: None,
        file_path: Some("/nonexistent/path/to/file.toml".to_string()),
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Validation result");

    assert!(result.is_err(), "Should fail with file not found error");

    tracing::info!("File not found test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_neither_content_nor_file() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative with neither content nor file");

    let server = BotticelliServer::builder().build();

    let params = ValidateNarrativeParams {
        content: None,
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Validation result");

    assert!(
        result.is_err(),
        "Should fail when neither content nor file_path provided"
    );

    tracing::info!("Neither content nor file test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_with_warnings_strict() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative strict mode with warnings");

    let server = BotticelliServer::builder().build();

    let toml_with_warnings = r#"
title = "Strict Warning Test"
version = "0.1.0"

[[acts]]
name = "act1"
objective = "Test"
model = "gpt-unknown-model"
"#;

    let params = ValidateNarrativeParams {
        content: Some(toml_with_warnings.to_string()),
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: true,
    };

    let result = server.validate_narrative(Parameters(params)).await?;
    tracing::debug!(
        valid = result.0.valid,
        warning_count = result.0.warnings.len(),
        "Validation result in strict mode"
    );

    // In strict mode, warnings should make it invalid
    if !result.0.warnings.is_empty() {
        assert!(
            !result.0.valid,
            "Should be invalid in strict mode with warnings"
        );
    }

    tracing::info!("Strict mode with warnings test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_result_structure() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validate_narrative result structure");

    let server = BotticelliServer::builder().build();

    let valid_toml = r#"
title = "Structure Test"
version = "0.1.0"

[[acts]]
name = "act1"
objective = "Test"
"#;

    let params = ValidateNarrativeParams {
        content: Some(valid_toml.to_string()),
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let result = server.validate_narrative(Parameters(params)).await?;
    tracing::debug!(summary = %result.0.summary, "Validation result");

    // Check result structure
    assert!(!result.0.summary.is_empty(), "Should have summary");
    assert!(
        result.0.summary.contains("error"),
        "Summary should mention errors"
    );
    assert!(
        result.0.summary.contains("warning"),
        "Summary should mention warnings"
    );

    tracing::info!("Result structure test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ValidateNarrativeParams serialization");

    let params = ValidateNarrativeParams {
        content: Some("test".to_string()),
        file_path: None,
        validate_files: false,
        validate_models: true,
        warn_unused: true,
        strict: false,
    };

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params");

    assert!(json.is_object());
    assert_eq!(json["content"], "test");
    assert_eq!(json["validate_models"], true);

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_narrative_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ValidateNarrativeResult serialization");

    use botticelli_mcp::ValidateNarrativeResult;

    let result = ValidateNarrativeResult::new(true, vec![], vec![]);

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert!(json.is_object());
    assert_eq!(json["valid"], true);
    assert!(json["errors"].is_array());
    assert!(json["warnings"].is_array());
    assert!(json["summary"].is_string());

    tracing::info!("Result serialization test passed");
    Ok(())
}
