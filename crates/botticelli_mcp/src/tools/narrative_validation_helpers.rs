//! Enhanced validation helpers for narrative generation tools.
//!
//! Provides better error formatting, suggestions, and TOML quality improvements.

use botticelli_narrative::validator::{ValidationError, ValidationResult, ValidationWarning};
use serde_json::{Value, json};

/// Format validation result as structured JSON with enhanced information.
pub fn format_validation_result(validation: &ValidationResult) -> Value {
    json!({
        "valid": validation.is_valid(),
        "errors": format_errors(&validation.errors),
        "warnings": format_warnings(&validation.warnings),
        "summary": create_summary(validation),
        "has_critical_errors": has_critical_errors(&validation.errors),
        "has_fixable_errors": has_fixable_errors(&validation.errors)
    })
}

/// Format errors with priority and fix suggestions.
fn format_errors(errors: &[ValidationError]) -> Vec<Value> {
    errors
        .iter()
        .map(|error| {
            let priority = categorize_error_priority(&error.kind);
            let fix = generate_fix_suggestion(error);

            json!({
                "message": error.message,
                "kind": format!("{:?}", error.kind),
                "priority": priority,
                "location": error.location.as_ref().map(|loc| json!({
                    "line": loc.line,
                    "column": loc.column,
                    "section": loc.section,
                })),
                "suggestion": error.suggestion.as_ref().or(fix.as_ref()),
                "fixable": error.suggestion.is_some() || fix.is_some(),
            })
        })
        .collect()
}

/// Format warnings with additional context.
fn format_warnings(warnings: &[ValidationWarning]) -> Vec<Value> {
    warnings
        .iter()
        .map(|warning| {
            json!({
                "message": warning.message,
                "kind": format!("{:?}", warning.kind),
                "location": warning.location.as_ref().map(|loc| json!({
                    "line": loc.line,
                    "column": loc.column,
                    "section": loc.section,
                })),
                "severity": categorize_warning_severity(&warning.kind)
            })
        })
        .collect()
}

/// Create a summary of validation results.
fn create_summary(validation: &ValidationResult) -> String {
    let error_count = validation.errors.len();
    let warning_count = validation.warnings.len();
    let fixable_count = validation
        .errors
        .iter()
        .filter(|e| e.suggestion.is_some())
        .count();

    if validation.is_valid() {
        if warning_count == 0 {
            "✅ Valid narrative with no issues".to_string()
        } else {
            format!("✅ Valid narrative with {} warning(s)", warning_count)
        }
    } else if fixable_count == error_count {
        format!("❌ {} error(s) - all have fix suggestions", error_count)
    } else if fixable_count > 0 {
        format!(
            "❌ {} error(s) ({} fixable, {} need manual review)",
            error_count,
            fixable_count,
            error_count - fixable_count
        )
    } else {
        format!("❌ {} error(s) - manual review required", error_count)
    }
}

/// Categorize error priority.
fn categorize_error_priority(kind: &botticelli_narrative::validator::ValidationErrorKind) -> &str {
    use botticelli_narrative::validator::ValidationErrorKind;

    match kind {
        ValidationErrorKind::MissingSection => "critical",
        ValidationErrorKind::InvalidSyntax => "critical",
        ValidationErrorKind::CircularDependency => "critical",
        ValidationErrorKind::UndefinedReference => "high",
        ValidationErrorKind::MissingAct => "high",
        ValidationErrorKind::EmptyToc => "medium",
        ValidationErrorKind::EmptyPrompt => "medium",
        ValidationErrorKind::FileNotFound => "medium",
    }
}

/// Categorize warning severity.
fn categorize_warning_severity(
    kind: &botticelli_narrative::validator::ValidationWarningKind,
) -> &str {
    use botticelli_narrative::validator::ValidationWarningKind;

    match kind {
        ValidationWarningKind::UnusedResource => "low",
        ValidationWarningKind::DirectTableReference => "low",
        ValidationWarningKind::UnknownModel => "medium",
        ValidationWarningKind::LargeMediaFile => "medium",
    }
}

/// Check if there are critical errors.
fn has_critical_errors(errors: &[ValidationError]) -> bool {
    errors
        .iter()
        .any(|e| categorize_error_priority(&e.kind) == "critical")
}

/// Check if errors have fix suggestions.
fn has_fixable_errors(errors: &[ValidationError]) -> bool {
    errors.iter().any(|e| e.suggestion.is_some())
}

/// Generate fix suggestion for errors without one.
fn generate_fix_suggestion(error: &ValidationError) -> Option<String> {
    use botticelli_narrative::validator::ValidationErrorKind;

    match &error.kind {
        ValidationErrorKind::EmptyToc => Some(
            "Add at least one act name to the [toc] order array. Example: order = [\"act1\"]"
                .to_string(),
        ),
        ValidationErrorKind::MissingSection => Some(
            "Add the required section to your narrative TOML. Required: [narrative], [toc], [acts]"
                .to_string(),
        ),
        ValidationErrorKind::EmptyPrompt => Some(
            "Add a prompt to the act. Example: act_name = \"Describe what this act should do\""
                .to_string(),
        ),
        _ => None,
    }
}

/// Improve TOML formatting with better structure.
pub fn format_toml(toml: &str) -> String {
    let mut formatted = String::new();
    let mut in_section = false;
    let mut last_was_blank = false;

    for line in toml.lines() {
        let trimmed = line.trim();

        // Skip multiple consecutive empty lines
        if trimmed.is_empty() {
            if !last_was_blank && !formatted.is_empty() {
                formatted.push('\n');
                last_was_blank = true;
            }
            continue;
        }

        // Add blank line before sections (except first)
        if trimmed.starts_with('[') {
            if in_section && !last_was_blank {
                formatted.push('\n');
            }
            in_section = true;
        }

        last_was_blank = false;
        formatted.push_str(line);
        formatted.push('\n');
    }

    formatted.trim_end().to_string()
}

/// Add helpful comments to generated TOML.
pub fn add_helpful_comments(toml: &str) -> String {
    let mut output = String::new();

    // Add header comment
    output.push_str("# Generated narrative TOML\n");
    output.push_str("# Edit as needed and validate with: botticelli validate <file>\n\n");

    for line in toml.lines() {
        let trimmed = line.trim();

        // Add section comments
        if trimmed == "[narrative]" {
            output.push_str("# Narrative metadata\n");
        } else if trimmed == "[toc]" {
            output.push_str("\n# Table of contents - execution order\n");
        } else if trimmed == "[acts]" {
            output.push_str("\n# Act definitions\n");
        } else if trimmed.starts_with("[bots.") {
            output.push_str("\n# Bot command definition\n");
        } else if trimmed.starts_with("[tables.") {
            output.push_str("\n# Database table query\n");
        } else if trimmed.starts_with("[media.") {
            output.push_str("\n# Media resource\n");
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

/// Validate and auto-fix common TOML issues.
pub fn auto_fix_common_issues(toml: &str) -> (String, Vec<String>) {
    let mut fixed = toml.to_string();
    let mut fixes_applied = Vec::new();

    // Fix 1: Ensure all sections are present
    if !fixed.contains("[narrative]") {
        fixed = format!("[narrative]\nname = \"unnamed\"\n\n{}", fixed);
        fixes_applied.push("Added missing [narrative] section".to_string());
    }

    if !fixed.contains("[toc]") {
        // Find all act names
        let act_names = extract_act_names(&fixed);
        if !act_names.is_empty() {
            let toc = format!(
                "[toc]\norder = [{}]\n\n",
                act_names
                    .iter()
                    .map(|name| format!("\"{}\"", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            // Insert before [acts] if present
            if let Some(acts_pos) = fixed.find("[acts]") {
                fixed.insert_str(acts_pos, &toc);
                fixes_applied.push("Added missing [toc] section with discovered acts".to_string());
            }
        }
    }

    if !fixed.contains("[acts]") {
        fixed.push_str("\n[acts]\n");
        fixes_applied.push("Added missing [acts] section".to_string());
    }

    // Fix 2: Remove trailing commas in arrays
    if fixed.contains(",]") {
        fixed = fixed.replace(",]", "]");
        fixes_applied.push("Removed trailing commas from arrays".to_string());
    }

    // Fix 3: Ensure proper spacing
    fixed = format_toml(&fixed);
    if fixes_applied.is_empty() {
        // Only mention formatting if no other fixes
        fixes_applied.push("Applied formatting improvements".to_string());
    }

    (fixed, fixes_applied)
}

/// Extract act names from TOML.
fn extract_act_names(toml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_acts_section = false;

    for line in toml.lines() {
        let trimmed = line.trim();

        if trimmed == "[acts]" {
            in_acts_section = true;
            continue;
        }

        if trimmed.starts_with('[') && in_acts_section {
            break;
        }

        if in_acts_section && trimmed.contains('=') {
            if let Some(name) = trimmed.split('=').next() {
                names.push(name.trim().to_string());
            }
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_extract_act_names() {
        let toml = "[acts]\nfetch = \"Get data\"\nanalyze = \"Process\"";
        let names = extract_act_names(toml);

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"fetch".to_string()));
        assert!(names.contains(&"analyze".to_string()));
    }
}
