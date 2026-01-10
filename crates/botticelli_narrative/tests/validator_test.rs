//! Tests for narrative TOML validation.

use botticelli_narrative::validator::{ValidationErrorKind, Validator};

#[test]
fn test_valid_narrative() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test narrative"
        
        [toc]
        order = ["act1"]
        
        [acts]
        act1 = "Hello world"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors
    );
    assert!(result.warnings().is_empty());
}

#[test]
fn test_array_of_tables_acts_error() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [toc]
        order = ["act1"]
        
        [[acts]]
        name = "act1"
        prompt = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    // Should have at least one error for [[acts]]
    assert!(!result.errors().is_empty());

    // Find the InvalidSyntax error about [[acts]]
    let syntax_error = result.errors().iter().find(|e| {
        matches!(e.kind, ValidationErrorKind::InvalidSyntax) && e.message.contains("[[acts]]")
    });
    assert!(
        syntax_error.is_some(),
        "Expected InvalidSyntax error for [[acts]], got: {:?}",
        result.errors
    );
    assert!(syntax_error.unwrap().suggestion.is_some());
}

#[test]
fn test_missing_toc_error() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [acts]
        act1 = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::MissingSection
    ));
    assert!(result.errors[0].message.contains("toc"));
}

#[test]
fn test_empty_toc_error() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [toc]
        order = []
        
        [acts]
        act1 = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::EmptyToc
    ));
}

#[test]
fn test_missing_act_error() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [toc]
        order = ["act1", "act2"]
        
        [acts]
        act1 = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::MissingAct
    ));
    assert!(result.errors[0].message.contains("act2"));
}

#[test]
fn test_undefined_bot_reference() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [bots.get_stats]
        platform = "discord"
        command = "server.get_stats"
        
        [toc]
        order = ["fetch"]
        
        [acts]
        fetch = "bots.undefined"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::UndefinedReference
    ));
    assert!(result.errors[0].message.contains("bots.undefined"));
    assert!(result.errors[0].suggestion.is_some());
    assert!(
        result.errors[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("get_stats")
    );
}

#[test]
fn test_valid_bot_reference() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [bots.get_stats]
        platform = "discord"
        command = "server.get_stats"
        
        [toc]
        order = ["fetch"]
        
        [acts]
        fetch = "bots.get_stats"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_valid_array_act() {
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [bots.get_stats]
        platform = "discord"
        command = "server.get_stats"
        
        [media.logo]
        file = "./logo.png"
        
        [toc]
        order = ["analyze"]
        
        [acts]
        analyze = ["bots.get_stats", "media.logo", "Analyze this"]
    "#;

    let result = Validator::validate_toml(toml);
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_multi_narrative_valid() {
    let toml = r#"
        [narratives.first]
        description = "First narrative"
        toc = ["act1"]
        
        [narratives.first.acts]
        act1 = "Hello"
        
        [narratives.second]
        description = "Second narrative"
        toc = ["act2"]
        
        [acts]
        act2 = "World"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_multi_narrative_empty_toc() {
    let toml = r#"
        [narratives.first]
        description = "First narrative"
        toc = []
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::EmptyToc
    ));
    assert!(result.errors[0].message.contains("first"));
}

#[test]
fn test_multi_narrative_missing_act() {
    let toml = r#"
        [narratives.first]
        description = "First narrative"
        toc = ["missing_act"]
        
        [acts]
        other_act = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors[0].kind,
        ValidationErrorKind::MissingAct
    ));
    assert!(result.errors[0].message.contains("missing_act"));
    assert!(result.errors[0].message.contains("first"));
}
