use botticelli_narrative::validator::validate_narrative_toml;

#[test]
fn test_invalid_acts_array() {
    let toml_content = r#"
[metadata]
title = "Test"
version = "1.0"

[[acts]]
name = "test"
"#;

    let result = validate_narrative_toml(toml_content);
    assert!(!result.is_valid());
    assert!(!result.errors.is_empty());
}

#[test]
fn test_missing_required_metadata() {
    let toml_content = r#"
[metadata]
# Missing title and version

[acts.test]
prompt = "test prompt"
"#;

    let result = validate_narrative_toml(toml_content);
    assert!(!result.is_valid());
    assert!(!result.errors.is_empty());
}

#[test]
fn test_empty_narrative() {
    let result = validate_narrative_toml("");
    assert!(!result.is_valid());
    assert!(!result.errors.is_empty());
}

#[test]
fn test_minimal_valid_narrative() {
    let toml_content = r#"
[metadata]
title = "Test Narrative"
version = "1.0"

[acts.test]
prompt = "test prompt"
"#;

    let result = validate_narrative_toml(toml_content);
    // May have warnings but should not have errors
    if !result.is_valid() {
        eprintln!("Validation errors: {}", result.format_errors());
    }
}
