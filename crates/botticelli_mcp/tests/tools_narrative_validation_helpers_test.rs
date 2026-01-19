//! Tests for narrative validation helpers.

use botticelli_mcp::tools::narrative_validation_helpers::{auto_fix_common_issues, format_toml};

#[test]
fn test_format_toml() {
    let toml =
        "[narrative]\nname = \"test\"\n\n\n[toc]\norder = [\"act1\"]\n[acts]\nact1 = \"test\"";
    let formatted = format_toml(toml);

    // Should not have triple blank lines
    assert!(!formatted.contains("\n\n\n"));
    // Should have blank line before [acts]
    assert!(formatted.contains("\n[acts]\n"));
}

#[test]
fn test_auto_fix_missing_toc() {
    let toml = "[narrative]\nname = \"test\"\n\n[acts]\nact1 = \"test\"\nact2 = \"test2\"";
    let (fixed, fixes) = auto_fix_common_issues(toml);

    assert!(fixed.contains("[toc]"));
    assert!(fixed.contains("order = [\"act1\", \"act2\"]"));
    assert!(!fixes.is_empty());
}
