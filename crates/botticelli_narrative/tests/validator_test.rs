//! Tests for narrative TOML validation.

mod helpers;

use botticelli_narrative::validator::{ValidationErrorKind, Validator};

#[test]
fn test_valid_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing valid narrative TOML");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML");
    
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors()
    );
    assert!(result.warnings().is_empty());
    
    tracing::info!("Valid narrative test passed");
    Ok(())
}

#[test]
fn test_array_of_tables_acts_error() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing array of tables [[acts]] error detection");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with [[acts]]");
    
    assert!(!result.is_valid());
    // Should have at least one error for [[acts]]
    assert!(!result.errors().is_empty());

    // Find the InvalidSyntax error about [[acts]]
    let syntax_error = result.errors().iter().find(|e| {
        matches!(e.kind(), ValidationErrorKind::InvalidSyntax) && e.message().contains("[[acts]]")
    });
    assert!(
        syntax_error.is_some(),
        "Expected InvalidSyntax error for [[acts]], got: {:?}",
        result.errors()
    );
    assert!(syntax_error.unwrap().suggestion().is_some());
    
    tracing::info!("Array of tables [[acts]] error test passed");
    Ok(())
}

#[test]
fn test_missing_toc_error() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing missing [toc] section error");
    
    let toml = r#"
        [narrative]
        name = "test"
        description = "Test"
        
        [acts]
        act1 = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML without [toc]");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::MissingSection
    ));
    assert!(result.errors()[0].message().contains("toc"));
    
    tracing::info!("Missing [toc] error test passed");
    Ok(())
}

#[test]
fn test_empty_toc_error() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing empty toc.order error");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with empty order");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::EmptyToc
    ));
    
    tracing::info!("Empty toc error test passed");
    Ok(())
}

#[test]
fn test_missing_act_error() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing missing act definition error");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with missing act");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::MissingAct
    ));
    assert!(result.errors()[0].message().contains("act2"));
    
    tracing::info!("Missing act error test passed");
    Ok(())
}

#[test]
fn test_undefined_bot_reference() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing undefined bot reference error");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with undefined bot");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::UndefinedReference
    ));
    assert!(result.errors()[0].message().contains("bots.undefined"));
    assert!(result.errors()[0].suggestion().is_some());
    assert!(
        result.errors()[0]
            .suggestion()
            .as_ref()
            .unwrap()
            .contains("get_stats")
    );
    
    tracing::info!("Undefined bot reference error test passed");
    Ok(())
}

#[test]
fn test_valid_bot_reference() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing valid bot reference");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with valid bot ref");
    
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors()
    );
    
    tracing::info!("Valid bot reference test passed");
    Ok(())
}

#[test]
fn test_valid_array_act() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing valid array act with multiple inputs");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated TOML with array act");
    
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors()
    );
    
    tracing::info!("Valid array act test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_valid() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing valid multi-narrative TOML");
    
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
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated multi-narrative TOML");
    
    assert!(
        result.is_valid(),
        "Expected valid narrative, got errors: {:?}",
        result.errors()
    );
    
    tracing::info!("Multi-narrative valid test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_empty_toc() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing multi-narrative with empty toc error");
    
    let toml = r#"
        [narratives.first]
        description = "First narrative"
        toc = []
    "#;

    let result = Validator::validate_toml(toml);
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated multi-narrative with empty toc");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::EmptyToc
    ));
    assert!(result.errors()[0].message().contains("first"));
    
    tracing::info!("Multi-narrative empty toc error test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_missing_act() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing multi-narrative with missing act error");
    
    let toml = r#"
        [narratives.first]
        description = "First narrative"
        toc = ["missing_act"]
        
        [acts]
        other_act = "Hello"
    "#;

    let result = Validator::validate_toml(toml);
    tracing::debug!(is_valid = result.is_valid(), error_count = result.errors().len(), "Validated multi-narrative with missing act");
    
    assert!(!result.is_valid());
    assert_eq!(result.errors().len(), 1);
    assert!(matches!(
        result.errors()[0].kind(),
        ValidationErrorKind::MissingAct
    ));
    assert!(result.errors()[0].message().contains("missing_act"));
    assert!(result.errors()[0].message().contains("first"));
    
    tracing::info!("Multi-narrative missing act error test passed");
    Ok(())
}
