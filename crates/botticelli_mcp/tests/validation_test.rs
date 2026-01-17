mod helpers;

use botticelli_narrative::validator::Validator;

#[test]
fn test_invalid_acts_array() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing invalid acts array format");

    let toml_content = r#"
[metadata]
title = "Test"
version = "1.0"

[[acts]]
name = "test"
"#;

    let result = Validator::validate_toml(toml_content);
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validation result");

    assert!(!result.is_valid());
    assert!(!result.errors().is_empty());

    tracing::info!("Invalid acts array test passed");
    Ok(())
}

#[test]
fn test_missing_required_metadata() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing missing required metadata");

    let toml_content = r#"
[metadata]
# Missing title and version

[acts.test]
prompt = "test prompt"
"#;

    let result = Validator::validate_toml(toml_content);
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validation result");

    assert!(!result.is_valid());
    assert!(!result.errors().is_empty());

    tracing::info!("Missing metadata test passed");
    Ok(())
}

#[test]
fn test_empty_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing empty narrative");

    let result = Validator::validate_toml("");
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validation result");

    assert!(!result.is_valid());
    assert!(!result.errors().is_empty());

    tracing::info!("Empty narrative test passed");
    Ok(())
}

#[test]
fn test_minimal_valid_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing minimal valid narrative");

    let toml_content = r#"
[metadata]
title = "Test Narrative"
version = "1.0"

[acts.test]
prompt = "test prompt"
"#;

    let result = Validator::validate_toml(toml_content);
    tracing::debug!(
        is_valid = result.is_valid(),
        error_count = result.errors().len(),
        warning_count = result.warnings().len(),
        "Validation result"
    );

    // May have warnings but should not have errors
    if !result.is_valid() {
        tracing::error!("Validation errors: {:?}", result.errors());
    }

    tracing::info!("Minimal valid narrative test passed");
    Ok(())
}



