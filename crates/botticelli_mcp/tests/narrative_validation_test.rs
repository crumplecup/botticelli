//! Tests for Phase 2 enhanced validation features.

use botticelli_mcp::tools::McpTool;
use botticelli_mcp::{CreateNarrativeTool, ModifyNarrativeTool};
use serde_json::json;

#[tokio::test]
async fn test_create_narrative_includes_auto_fixes() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Analyze data",
        "name": "test_narrative"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    // Check that auto_fixes_applied field exists
    assert!(
        result.get("auto_fixes_applied").is_some(),
        "Should include auto_fixes_applied"
    );

    let fixes = result.get("auto_fixes_applied").unwrap().as_array().unwrap();
    // Should have at least formatting improvements
    assert!(!fixes.is_empty(), "Should report auto-fixes applied");
}

#[tokio::test]
async fn test_create_narrative_includes_comments_version() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Fetch data and analyze it",
        "name": "analysis"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    // Check both TOML versions exist
    assert!(result.get("toml").is_some(), "Should have clean TOML");
    assert!(
        result.get("toml_with_comments").is_some(),
        "Should have commented TOML"
    );

    let toml = result.get("toml").unwrap().as_str().unwrap();
    let toml_with_comments = result
        .get("toml_with_comments")
        .unwrap()
        .as_str()
        .unwrap();

    // Clean version should not have comments
    assert!(
        !toml.contains("# Generated narrative"),
        "Clean TOML should not have header comments"
    );

    // Commented version should have comments
    assert!(
        toml_with_comments.contains("# Generated narrative"),
        "Commented TOML should have header"
    );
    assert!(
        toml_with_comments.contains("# Narrative metadata"),
        "Commented TOML should have section comments"
    );
}

#[tokio::test]
async fn test_validation_includes_priorities() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Test narrative",
        "name": "test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let validation = result.get("validation").unwrap();

    // Check enhanced validation structure
    assert!(validation.get("valid").is_some(), "Should have valid flag");
    assert!(
        validation.get("errors").is_some(),
        "Should have errors array"
    );
    assert!(
        validation.get("warnings").is_some(),
        "Should have warnings array"
    );
    assert!(
        validation.get("summary").is_some(),
        "Should have summary text"
    );
    assert!(
        validation.get("has_critical_errors").is_some(),
        "Should have critical errors flag"
    );
    assert!(
        validation.get("has_fixable_errors").is_some(),
        "Should have fixable errors flag"
    );
}

#[tokio::test]
async fn test_validation_summary_uses_emojis() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Simple test",
        "name": "emoji_test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let validation = result.get("validation").unwrap();
    let summary = validation.get("summary").unwrap().as_str().unwrap();

    // Valid narratives should have success emoji
    if validation.get("valid").unwrap().as_bool().unwrap() {
        assert!(
            summary.contains("✅"),
            "Valid narrative summary should have ✅"
        );
    }
}

#[tokio::test]
async fn test_toml_formatting_improves_structure() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Format test narrative",
        "name": "format_test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let toml = result.get("toml").unwrap().as_str().unwrap();

    // Should have all required sections
    assert!(
        toml.contains("[narrative]") && toml.contains("[toc]") && toml.contains("[acts]"),
        "TOML should have all required sections"
    );

    // Should be parseable (basic validation)
    assert!(
        !toml.is_empty() && toml.contains("name ="),
        "TOML should be well-formed"
    );

    // Shouldn't have excessive blank lines (more than 4 consecutive newlines)
    assert!(
        !toml.contains("\n\n\n\n\n"),
        "TOML should not have excessive blank lines"
    );
}

#[tokio::test]
async fn test_create_narrative_returns_act_count() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Fetch data, then analyze it, then report results",
        "name": "multi_act"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    // Should include act_count field
    assert!(
        result.get("act_count").is_some(),
        "Should include act_count"
    );

    let act_count = result.get("act_count").unwrap().as_u64().unwrap();
    assert!(
        act_count >= 2,
        "Should have at least 2 acts from description"
    );
}

#[tokio::test]
async fn test_modify_narrative_includes_auto_fixes_in_changes() {
    let tool = ModifyNarrativeTool;

    // Create a narrative with potential issues
    let existing_toml = r#"[narrative]
name = "test"

[toc]
order = ["act1",]

[acts]
act1 = "Test act""#;

    let input = json!({
        "narrative_toml": existing_toml,
        "modification": "Add act that does analysis"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let changes = result.get("changes").unwrap().as_array().unwrap();

    // Should include both the modification and any auto-fixes
    assert!(!changes.is_empty(), "Should have changes");

    // Check if any change mentions auto-fix or formatting
    let has_auto_fix_info = changes.iter().any(|change| {
        let change_str = change.as_str().unwrap();
        change_str.contains("Auto-fix") || change_str.contains("formatting")
    });

    // If trailing comma was present, should be mentioned
    if existing_toml.contains(",]") {
        assert!(
            has_auto_fix_info,
            "Should mention trailing comma auto-fix"
        );
    }
}

#[tokio::test]
async fn test_validation_errors_have_priorities() {
    // Test with intentionally broken TOML to trigger validation errors
    let tool = ModifyNarrativeTool;

    let broken_toml = r#"[narrative]
name = "broken"

[toc]
order = []

[acts]"#;

    let input = json!({
        "narrative_toml": broken_toml,
        "modification": "Set temperature to 0.5"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let validation = result.get("validation").unwrap();
    let errors = validation.get("errors").unwrap().as_array().unwrap();

    // If there are errors, they should have priority field
    if !errors.is_empty() {
        let first_error = &errors[0];
        assert!(
            first_error.get("priority").is_some(),
            "Errors should have priority field"
        );

        let priority = first_error.get("priority").unwrap().as_str().unwrap();
        assert!(
            ["critical", "high", "medium", "low"].contains(&priority),
            "Priority should be valid level"
        );
    }
}

#[tokio::test]
async fn test_validation_errors_include_suggestions() {
    let tool = ModifyNarrativeTool;

    // Empty TOC should trigger an error with suggestion
    let toml_with_empty_toc = r#"[narrative]
name = "test"

[toc]
order = []

[acts]
analyze = "Analyze data""#;

    let input = json!({
        "narrative_toml": toml_with_empty_toc,
        "modification": "Change model to Gemini"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let validation = result.get("validation").unwrap();
    let errors = validation.get("errors").unwrap().as_array().unwrap();

    // Should have error about empty TOC
    if !errors.is_empty() {
        let has_suggestion = errors.iter().any(|error| {
            error.get("suggestion").is_some()
                && !error.get("suggestion").unwrap().is_null()
        });

        assert!(
            has_suggestion,
            "At least one error should have a fix suggestion"
        );
    }
}

#[tokio::test]
async fn test_auto_fix_adds_missing_sections() {
    let tool = CreateNarrativeTool;

    // Create a simple narrative - auto-fix will ensure all sections exist
    let input = json!({
        "description": "Test auto-fix behavior",
        "name": "auto_fix_test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    let auto_fixes = result
        .get("auto_fixes_applied")
        .unwrap()
        .as_array()
        .unwrap();

    // Should have all required sections
    assert!(
        toml.contains("[narrative]"),
        "Should have [narrative] section"
    );
    assert!(toml.contains("[toc]"), "Should have [toc] section");
    assert!(toml.contains("[acts]"), "Should have [acts] section");

    // Should report fixes applied
    assert!(
        !auto_fixes.is_empty(),
        "Should report auto-fixes applied"
    );
}

#[tokio::test]
async fn test_auto_fix_generates_toc_from_acts() {
    let tool = ModifyNarrativeTool;

    // TOML with acts but no TOC
    let toml_no_toc = r#"[narrative]
name = "test"

[acts]
fetch = "Fetch data"
analyze = "Analyze data"
report = "Report findings""#;

    let input = json!({
        "narrative_toml": toml_no_toc,
        "modification": "Use Claude model"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let toml = result.get("toml").unwrap().as_str().unwrap();

    // Auto-fix should generate TOC from act names
    assert!(toml.contains("[toc]"), "Should have [toc] section");
    assert!(
        toml.contains("fetch") && toml.contains("analyze") && toml.contains("report"),
        "TOC should include all act names"
    );
}

#[tokio::test]
async fn test_auto_fix_removes_trailing_commas() {
    let tool = ModifyNarrativeTool;

    let toml_with_trailing = r#"[narrative]
name = "test"

[toc]
order = ["act1", "act2",]

[acts]
act1 = "First"
act2 = "Second""#;

    let input = json!({
        "narrative_toml": toml_with_trailing,
        "modification": "Set temperature to 0.7"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let toml = result.get("toml").unwrap().as_str().unwrap();
    let changes = result.get("changes").unwrap().as_array().unwrap();

    // Should not have trailing comma
    assert!(
        !toml.contains(",]"),
        "Auto-fix should remove trailing commas"
    );

    // Should mention the fix in changes
    let mentions_trailing_comma = changes.iter().any(|change| {
        change
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("trailing comma")
    });

    assert!(
        mentions_trailing_comma,
        "Changes should mention trailing comma fix"
    );
}

#[tokio::test]
async fn test_comments_include_all_section_types() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Test all comment types",
        "name": "comment_test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let toml_with_comments = result
        .get("toml_with_comments")
        .unwrap()
        .as_str()
        .unwrap();

    // Check for header comments
    assert!(
        toml_with_comments.contains("# Generated narrative TOML"),
        "Should have header comment"
    );

    // Check for section comments
    assert!(
        toml_with_comments.contains("# Narrative metadata"),
        "Should have [narrative] comment"
    );
    assert!(
        toml_with_comments.contains("# Table of contents"),
        "Should have [toc] comment"
    );
    assert!(
        toml_with_comments.contains("# Act definitions"),
        "Should have [acts] comment"
    );
}

#[tokio::test]
async fn test_validation_summary_indicates_fixable_errors() {
    let tool = ModifyNarrativeTool;

    // Create TOML with fixable error (empty TOC)
    let toml_with_fixable_error = r#"[narrative]
name = "test"

[toc]
order = []

[acts]
analyze = "Analyze""#;

    let input = json!({
        "narrative_toml": toml_with_fixable_error,
        "modification": "Add act that reports results"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let validation = result.get("validation").unwrap();
    let summary = validation.get("summary").unwrap().as_str().unwrap();

    // If there are errors, summary should mention if they're fixable
    if !validation.get("valid").unwrap().as_bool().unwrap() {
        assert!(
            summary.contains("error") || summary.contains("❌"),
            "Summary should indicate errors"
        );
    }
}

#[tokio::test]
async fn test_enhanced_summary_shows_fix_counts() {
    let tool = CreateNarrativeTool;

    let input = json!({
        "description": "Create narrative with multiple acts",
        "name": "fix_count_test"
    });

    let result = tool.execute(input).await.expect("Tool execution failed");

    let summary = result.get("summary").unwrap().as_str().unwrap();

    // Summary should mention acts and possibly fixes
    assert!(
        summary.contains("act") || summary.contains("Act"),
        "Summary should mention acts"
    );

    // If auto-fixes were applied, should be mentioned
    let auto_fixes = result
        .get("auto_fixes_applied")
        .unwrap()
        .as_array()
        .unwrap();
    if auto_fixes.len() > 1 {
        assert!(
            summary.contains("auto-fix") || summary.contains("fix"),
            "Summary should mention auto-fixes when multiple applied"
        );
    }
}
