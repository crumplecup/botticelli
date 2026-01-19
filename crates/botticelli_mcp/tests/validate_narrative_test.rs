//! Tests for validate_narrative tool.

mod helpers;

use botticelli_mcp::ToolRegistry;
use serde_json::json;

#[tokio::test]
async fn test_validate_valid_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation of valid narrative");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narrative]
name = "test"
description = "Test narrative"

[toc]
order = ["act1"]

[acts]
act1 = "Hello world"
        "#
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    assert_eq!(result["valid"], true);
    assert_eq!(result["errors"].as_array().unwrap().len(), 0);
    assert_eq!(result["warnings"].as_array().unwrap().len(), 0);

    tracing::info!("Valid narrative test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_invalid_syntax() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation of invalid syntax");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narrative]
name = "test"

[toc]
order = ["act1"]

[[acts]]
name = "act1"
prompt = "Hello"
        "#
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    assert_eq!(result["valid"], false);
    assert!(!result["errors"].as_array().unwrap().is_empty());

    let error_msg = result["errors"][0]["message"].as_str().unwrap();
    assert!(error_msg.contains("[[acts]]"));

    tracing::info!("Invalid syntax test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_unknown_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation of unknown model");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narrative]
name = "test"
description = "Test"
model = "gpt-5-turbo"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
        "#,
        "validate_models": true
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    assert_eq!(result["valid"], true); // Warnings don't fail validation
    assert!(!result["warnings"].as_array().unwrap().is_empty());

    let warning_msg = result["warnings"][0]["message"].as_str().unwrap();
    assert!(warning_msg.contains("gpt-5-turbo"));
    assert!(warning_msg.contains("gpt-4-turbo"));

    tracing::info!("Unknown model test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_unused_resources() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation of unused resources");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narrative]
name = "test"

[bots.unused]
platform = "discord"
command = "test"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
        "#,
        "warn_unused": true
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    assert_eq!(result["valid"], true);
    assert!(!result["warnings"].as_array().unwrap().is_empty());

    let warning_msg = result["warnings"][0]["message"].as_str().unwrap();
    assert!(warning_msg.contains("unused"));

    tracing::info!("Unused resources test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_circular_dependency() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation of circular dependency");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narratives.first]
description = "First"
toc = ["step1"]

[narratives.first.acts]
step1 = "narrative.second"

[narratives.second]
description = "Second"
toc = ["step2"]

[narratives.second.acts]
step2 = "narrative.first"
        "#
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    assert_eq!(result["valid"], false);
    assert!(!result["errors"].as_array().unwrap().is_empty());

    let error_msg = result["errors"][0]["message"].as_str().unwrap();
    assert!(error_msg.contains("Circular dependency"));

    tracing::info!("Circular dependency test passed");
    Ok(())
}

#[tokio::test]
async fn test_validate_strict_mode() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation strict mode");

    let registry = ToolRegistry::default();

    let input = json!({
        "content": r#"
[narrative]
name = "test"
model = "unknown-model"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
        "#,
        "strict": true
    });

    let result = registry.execute("validate_narrative", input).await?;
    tracing::debug!(?result, "Validation result");

    // Strict mode treats warnings as errors
    assert_eq!(result["valid"], false);
    assert!(!result["warnings"].as_array().unwrap().is_empty());

    tracing::info!("Strict mode test passed");
    Ok(())
}

#[tokio::test]
async fn test_tool_registry_includes_validator() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing tool registry includes validator");

    let registry = ToolRegistry::default();

    // Try calling the tool to verify it exists
    let input = json!({"content": "[narrative]\nname = \"test\"\n[acts]\nact1 = \"hello\""});
    let result = registry.execute("validate_narrative", input).await;
    tracing::debug!(has_tool = result.is_ok(), "Checked registry");

    assert!(
        result.is_ok(),
        "validate_narrative tool should be available"
    );

    tracing::info!("Tool registry test passed");
    Ok(())
}
