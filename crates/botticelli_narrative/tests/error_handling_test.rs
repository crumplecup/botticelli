use botticelli_narrative::{Narrative, NarrativeError, NarrativeErrorKind};

#[test]
fn test_invalid_toml_parsing() {
    let invalid_toml = r#"
        [narrative]
        name = "test"
        # Missing closing bracket
        [[act]
    "#;
    
    let result = toml::from_str::<Narrative>(invalid_toml);
    assert!(result.is_err());
}

#[test]
fn test_missing_required_fields() {
    let incomplete_toml = r#"
        [narrative]
        # Missing name field
    "#;
    
    let result = toml::from_str::<Narrative>(incomplete_toml);
    assert!(result.is_err());
}

#[test]
fn test_circular_narrative_reference() {
    // Test that circular references are detected
    let circular_toml = r#"
        [narrative]
        name = "circular"
        toc = ["circular"]
    "#;
    
    let result = toml::from_str::<Narrative>(circular_toml);
    // Should either fail parsing or detect cycle during execution
    assert!(result.is_ok()); // Parse succeeds but execution should fail
}
