//! Shared utilities for narrative generation and elicitation.
//!
//! This module provides composable, reusable functions for:
//! - Extracting acts from natural language descriptions
//! - Validating narrative names
//! - Formatting and escaping TOML strings
//! - Counting acts in TOML documents
//!
//! These utilities are used by both the MCP create_narrative tool and
//! future elicitation systems.

use tracing::instrument;

/// Act definition extracted from description.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Act {
    /// Act name (used as identifier in TOML).
    pub name: String,
    /// Act prompt (the instruction text).
    pub prompt: String,
}

/// Extract acts from natural language description.
///
/// Uses heuristics to identify workflow steps:
/// - Split on common connectors (commas, semicolons)
/// - Look for "then" patterns
/// - Look for "and" patterns
/// - Extract action verbs as act names
#[instrument]
pub fn extract_acts_from_description(description: &str) -> Vec<Act> {
    // Simple heuristic: split on common connectors
    let parts: Vec<&str> = description
        .split([',', ';'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        // Fallback: single act
        return vec![Act {
            name: "process".to_string(),
            prompt: description.to_string(),
        }];
    }

    // Look for action verbs and create acts
    let mut acts = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let name = extract_act_name(part, i);
        acts.push(Act {
            name,
            prompt: (*part).to_string(),
        });
    }

    // Look for "then" patterns
    if description.contains(" then ") {
        let then_parts: Vec<&str> = description.split(" then ").collect();
        if then_parts.len() > 1 {
            acts.clear();
            for (i, part) in then_parts.iter().enumerate() {
                let name = extract_act_name(part, i);
                acts.push(Act {
                    name,
                    prompt: part.trim().to_string(),
                });
            }
        }
    }

    // Look for "and" patterns
    if acts.len() == 1 && description.contains(" and ") {
        let and_parts: Vec<&str> = description.split(" and ").collect();
        if and_parts.len() > 1 {
            acts.clear();
            for (i, part) in and_parts.iter().enumerate() {
                let name = extract_act_name(part, i);
                acts.push(Act {
                    name,
                    prompt: part.trim().to_string(),
                });
            }
        }
    }

    if acts.is_empty() {
        acts.push(Act {
            name: "process".to_string(),
            prompt: description.to_string(),
        });
    }

    acts
}

/// Extract act name from description part.
///
/// Looks for common action verbs at the start of the description.
/// Falls back to "actN" if no verb is found.
#[instrument]
fn extract_act_name(description: &str, fallback_index: usize) -> String {
    // Common action verbs
    let verbs = [
        "fetch", "get", "retrieve", "load", "read", "analyze", "process", "transform", "generate",
        "create", "build", "format", "send", "post", "publish", "write", "save", "store",
        "compare", "evaluate", "rank", "filter", "select", "choose", "extract", "parse",
    ];

    let lower = description.to_lowercase();

    // Find first verb
    for verb in &verbs {
        if lower.starts_with(verb) || lower.contains(&format!(" {} ", verb)) {
            return (*verb).to_string();
        }
    }

    // Fallback
    format!("act{}", fallback_index + 1)
}

/// Check if a narrative name is valid.
///
/// Valid names:
/// - Alphanumeric characters and underscores only
/// - Must start with a letter
/// - Length between 1 and 64 characters
#[instrument]
pub fn is_valid_narrative_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    // Must start with letter
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return false;
    }

    // All characters must be alphanumeric or underscore
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Escape string for TOML.
///
/// Handles special characters that need escaping in TOML strings.
#[instrument(skip(s))]
pub fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Count acts in TOML.
///
/// Counts either inline act definitions in [acts] section or
/// separate [acts.name] sections.
#[instrument(skip(toml))]
pub fn count_acts(toml: &str) -> usize {
    toml.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('[') && trimmed.contains("acts.")
        })
        .count()
        .max(
            toml.lines()
                .find(|line| line.trim() == "[acts]")
                .map(|_| {
                    toml.lines()
                        .skip_while(|line| !line.trim().starts_with("[acts]"))
                        .skip(1)
                        .take_while(|line| !line.trim().starts_with('['))
                        .filter(|line| line.contains('='))
                        .count()
                })
                .unwrap_or(0),
        )
}
