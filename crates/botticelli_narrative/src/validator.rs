//! Narrative TOML validation with actionable error messages.
//!
//! This module provides comprehensive validation for narrative TOML files,
//! catching common syntax errors and providing specific fix suggestions.

use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Result of validating a narrative TOML file.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ValidationResult {
    /// Validation errors (must be fixed)
    errors: Vec<ValidationError>,
    /// Validation warnings (should be reviewed)
    warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Creates a new validation result with no errors or warnings.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Returns true if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Adds an error to the result.
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Adds a warning to the result.
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Formats errors as a human-readable string.
    pub fn format_errors(&self) -> String {
        let mut output = String::new();

        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&format!("Error {}: {}", i + 1, error.message));

            if let Some(suggestion) = &error.suggestion {
                output.push_str(&format!("\n\n  Suggestion: {}", suggestion));
            }
        }

        output
    }

    /// Formats warnings as a human-readable string.
    pub fn format_warnings(&self) -> String {
        let mut output = String::new();

        for (i, warning) in self.warnings.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&format!("Warning {}: {}", i + 1, warning.message));
        }

        output
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A validation error with location and fix suggestion.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ValidationError {
    /// Type of validation error
    kind: ValidationErrorKind,
    /// Location in the TOML file (if available)
    location: Option<ValidationLocation>,
    /// Human-readable error message
    message: String,
    /// Suggestion on how to fix the error
    suggestion: Option<String>,
}

/// A validation warning that should be reviewed.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ValidationWarning {
    /// Type of validation warning
    kind: ValidationWarningKind,
    /// Location in the TOML file (if available)
    location: Option<ValidationLocation>,
    /// Human-readable warning message
    message: String,
}

/// Location information for validation messages.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ValidationLocation {
    /// Line number (1-indexed)
    line: usize,
    /// Column number (1-indexed)
    column: usize,
    /// Section name (e.g., "acts.fetch_data")
    section: Option<String>,
}

/// Types of validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrorKind {
    /// Invalid TOML syntax pattern
    InvalidSyntax,
    /// Missing required section
    MissingSection,
    /// Undefined resource reference
    UndefinedReference,
    /// Empty table of contents
    EmptyToc,
    /// Act referenced in toc but not defined
    MissingAct,
    /// Act has no inputs
    EmptyPrompt,
    /// File not found
    FileNotFound,
    /// Circular dependency detected
    CircularDependency,
}

/// Types of validation warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationWarningKind {
    /// Unknown model name (possible typo)
    UnknownModel,
    /// Defined resource never used
    UnusedResource,
    /// Direct table reference without [tables] definition
    DirectTableReference,
    /// Large media file (may impact performance)
    LargeMediaFile,
}

/// Configuration for validation behavior.
#[derive(Debug, Clone, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct ValidationConfig {
    /// Check that nested narrative files exist
    validate_nested_narratives: bool,
    /// Check that media files exist
    validate_media_files: bool,
    /// Warn on unknown model names
    warn_unknown_models: bool,
    /// Warn on unused resources
    warn_unused_resources: bool,
    /// Base directory for relative path resolution
    base_dir: Option<PathBuf>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_nested_narratives: true,
            validate_media_files: true,
            warn_unknown_models: true,
            warn_unused_resources: true,
            base_dir: None,
        }
    }
}

/// Known model names for validation.
const KNOWN_MODELS: &[&str] = &[
    // Gemini models
    "gemini-2.0-flash-exp",
    "gemini-1.5-flash",
    "gemini-1.5-flash-8b",
    "gemini-1.5-pro",
    "gemini-exp-1206",
    // OpenAI models
    "gpt-4",
    "gpt-4-turbo",
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-3.5-turbo",
    // Anthropic models
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    // Groq models
    "llama-3.3-70b-versatile",
    "llama-3.1-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
    // HuggingFace (common examples)
    "meta-llama/Meta-Llama-3-8B-Instruct",
    "mistralai/Mistral-7B-Instruct-v0.2",
    // Ollama (common models)
    "llama3.2",
    "llama3.1",
    "mistral",
    "phi3",
];

/// Validates a narrative TOML string.
///
/// # Arguments
///
/// * `toml` - The TOML content to validate
///
/// # Returns
///
/// A `ValidationResult` containing any errors or warnings found.
///
/// # Examples
///
/// ```
/// use botticelli_narrative::validator::validate_narrative_toml;
///
/// let toml = r#"
///     [narrative]
///     name = "test"
///     description = "Test narrative"
///     
///     [toc]
///     order = ["act1"]
///     
///     [acts]
///     act1 = "Hello world"
/// "#;
///
/// let result = validate_narrative_toml(toml);
/// assert!(result.is_valid());
/// ```
#[instrument(skip(toml), fields(toml_len = toml.len()))]
pub fn validate_narrative_toml(toml: &str) -> ValidationResult {
    validate_narrative_toml_with_config(toml, &ValidationConfig::default())
}

/// Validates a narrative TOML string with custom configuration.
#[instrument(skip(toml, config), fields(toml_len = toml.len()))]
pub fn validate_narrative_toml_with_config(
    toml: &str,
    config: &ValidationConfig,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Phase 1: Parse TOML to detect syntax issues
    let parsed = match toml::from_str::<toml::Value>(toml) {
        Ok(value) => value,
        Err(e) => {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::InvalidSyntax,
                location: None,
                message: format!("Failed to parse TOML: {}", e),
                suggestion: Some("Check for syntax errors like missing quotes, unmatched brackets, or invalid escape sequences.".to_string()),
            });
            return result;
        }
    };

    // Phase 2: Check for common syntax patterns
    detect_syntax_patterns(&parsed, &mut result);

    // Phase 3: Validate structure (sections, references, etc.)
    validate_structure(&parsed, config, &mut result);

    result
}

/// Validates a narrative file.
///
/// # Arguments
///
/// * `path` - Path to the narrative TOML file
///
/// # Returns
///
/// A `ValidationResult` containing any errors or warnings found.
#[instrument(skip(path), fields(path = %path.as_ref().display()))]
pub fn validate_narrative_file(path: impl AsRef<Path>) -> ValidationResult {
    let config = ValidationConfig {
        base_dir: path.as_ref().parent().map(|p| p.to_path_buf()),
        ..Default::default()
    };

    validate_narrative_file_with_config(path, &config)
}

/// Validates a narrative file with custom configuration.
#[instrument(skip(path, config), fields(path = %path.as_ref().display()))]
pub fn validate_narrative_file_with_config(
    path: impl AsRef<Path>,
    config: &ValidationConfig,
) -> ValidationResult {
    let path = path.as_ref();
    let mut result = ValidationResult::new();

    // Read file
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::FileNotFound,
                location: None,
                message: format!("Failed to read file '{}': {}", path.display(), e),
                suggestion: Some("Check that the file exists and is readable.".to_string()),
            });
            return result;
        }
    };

    validate_narrative_toml_with_config(&content, config)
}

/// Detects common TOML syntax patterns that indicate mistakes.
fn detect_syntax_patterns(parsed: &toml::Value, result: &mut ValidationResult) {
    if let Some(table) = parsed.as_table() {
        // Check for [[acts]] (array of tables instead of table of tables)
        if let Some(acts_value) = table.get("acts")
            && acts_value.is_array()
        {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::InvalidSyntax,
                location: None,
                message: "Found [[acts]] but acts should be a table of tables, not an array"
                    .to_string(),
                suggestion: Some(
                    "Use one of these formats:\n\n\
                    1. Inline table syntax (recommended for simple prompts):\n\
                       [acts]\n\
                       fetch = \"Get data\"\n\n\
                    2. Table syntax (for acts with configuration):\n\
                       [acts.fetch]\n\
                       prompt = \"Get data\"\n\
                       model = \"gemini-2.0-flash-exp\""
                        .to_string(),
                ),
            });
        }

        // Check for [[narrative]] (multiple narrative sections)
        if let Some(narrative_value) = table.get("narrative")
            && narrative_value.is_array()
        {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::InvalidSyntax,
                location: None,
                message: "Found [[narrative]] but only a single [narrative] section is allowed".to_string(),
                suggestion: Some("Use a single [narrative] section:\n\n[narrative]\nname = \"my_narrative\"\ndescription = \"...\"".to_string()),
            });
        }
    }
}

/// Validates narrative structure (sections, references, etc.).
fn validate_structure(
    parsed: &toml::Value,
    config: &ValidationConfig,
    result: &mut ValidationResult,
) {
    let table = match parsed.as_table() {
        Some(t) => t,
        None => {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::InvalidSyntax,
                location: None,
                message: "TOML root must be a table".to_string(),
                suggestion: None,
            });
            return;
        }
    };

    // Check for [narrative] section
    let has_narrative = table.contains_key("narrative");
    let has_narratives = table.contains_key("narratives");

    if !has_narrative && !has_narratives {
        result.add_error(ValidationError {
            kind: ValidationErrorKind::MissingSection,
            location: None,
            message: "Missing [narrative] or [narratives] section".to_string(),
            suggestion: Some("Add a [narrative] section:\n\n[narrative]\nname = \"my_narrative\"\ndescription = \"...\"".to_string()),
        });
        return;
    }

    // Validate model names if enabled
    if config.warn_unknown_models {
        if has_narrative && let Some(narrative) = table.get("narrative").and_then(|v| v.as_table())
        {
            validate_model_name(narrative, "narrative", result);
        }
        if has_narratives
            && let Some(narratives) = table.get("narratives").and_then(|v| v.as_table())
        {
            for (name, narrative_value) in narratives {
                if let Some(narrative_table) = narrative_value.as_table() {
                    validate_model_name(narrative_table, &format!("narratives.{}", name), result);
                }
            }
        }
        // Check acts for model overrides
        if let Some(acts) = table.get("acts").and_then(|v| v.as_table()) {
            for (act_name, act_value) in acts {
                if let Some(act_table) = act_value.as_table() {
                    validate_model_name(act_table, &format!("acts.{}", act_name), result);
                }
            }
        }
    }

    // Collect resources for reference validation and unused detection
    let resources = collect_resources(table);

    // For single narrative files, validate toc and acts
    if has_narrative {
        validate_single_narrative(table, &resources, result);
    }

    // For multi-narrative files, each narrative has its own toc
    if has_narratives {
        validate_multi_narratives(table, &resources, result);
    }

    // Check for unused resources if enabled
    if config.warn_unused_resources {
        check_unused_resources(&resources, result);
    }

    // Check for circular dependencies in narrative references
    check_circular_dependencies(table, result);
}

/// Validates a single narrative structure.
fn validate_single_narrative(
    table: &toml::map::Map<String, toml::Value>,
    resources: &ResourceRegistry,
    result: &mut ValidationResult,
) {
    // Check for toc
    if !table.contains_key("toc") {
        result.add_error(ValidationError {
            kind: ValidationErrorKind::MissingSection,
            location: None,
            message: "Missing [toc] section".to_string(),
            suggestion: Some(
                "Add a table of contents:\n\n[toc]\norder = [\"act1\", \"act2\"]".to_string(),
            ),
        });
        return;
    }

    // Get toc.order
    let toc_order = extract_toc_order(table.get("toc"));
    if toc_order.is_empty() {
        result.add_error(ValidationError {
            kind: ValidationErrorKind::EmptyToc,
            location: None,
            message: "Table of contents is empty".to_string(),
            suggestion: Some(
                "Add at least one act to toc.order:\n\n[toc]\norder = [\"act1\"]".to_string(),
            ),
        });
        return;
    }

    // Get acts
    let acts = extract_acts(table.get("acts"));

    // Validate each act in toc.order exists and has valid references
    for act_name in &toc_order {
        if !acts.contains_key(act_name.as_str()) {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::MissingAct,
                location: None,
                message: format!("Act '{}' referenced in toc.order does not exist", act_name),
                suggestion: Some(format!(
                    "Add the act:\n\n[acts.{}]\nprompt = \"...\"",
                    act_name
                )),
            });
        } else {
            // Validate references in act
            if let Some(act_value) = acts.get(act_name.as_str()) {
                validate_act_references(act_name, act_value, resources, result);
            }
        }
    }
}

/// Validates multi-narrative structure.
fn validate_multi_narratives(
    table: &toml::map::Map<String, toml::Value>,
    resources: &ResourceRegistry,
    result: &mut ValidationResult,
) {
    let narratives = match table.get("narratives").and_then(|v| v.as_table()) {
        Some(n) => n,
        None => return,
    };

    let shared_acts = extract_acts(table.get("acts"));

    for (narrative_name, narrative_value) in narratives {
        let narrative_table = match narrative_value.as_table() {
            Some(t) => t,
            None => continue,
        };

        // Each narrative must have a toc
        let toc_order = extract_toc_order(narrative_table.get("toc"));
        if toc_order.is_empty() {
            result.add_error(ValidationError {
                kind: ValidationErrorKind::EmptyToc,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(format!("narratives.{}", narrative_name)),
                }),
                message: format!("Narrative '{}' has empty table of contents", narrative_name),
                suggestion: Some(format!(
                    "Add toc to narrative:\n\n[narratives.{}]\ntoc = [\"act1\"]",
                    narrative_name
                )),
            });
            continue;
        }

        // Get narrative-specific acts
        let narrative_acts = extract_acts(narrative_table.get("acts"));

        // Validate each act exists (either in shared or narrative-specific acts)
        for act_name in &toc_order {
            let act_value = narrative_acts
                .get(act_name.as_str())
                .or_else(|| shared_acts.get(act_name.as_str()));

            if act_value.is_none() {
                result.add_error(ValidationError {
                    kind: ValidationErrorKind::MissingAct,
                    location: Some(ValidationLocation {
                        line: 0,
                        column: 0,
                        section: Some(format!("narratives.{}", narrative_name)),
                    }),
                    message: format!(
                        "Act '{}' in narrative '{}' does not exist",
                        act_name, narrative_name
                    ),
                    suggestion: Some(format!(
                        "Add the act under [acts] or [narratives.{}.acts]",
                        narrative_name
                    )),
                });
            } else if let Some(value) = act_value {
                validate_act_references(act_name, value, resources, result);
            }
        }
    }
}

/// Extracts toc.order from a toc value.
fn extract_toc_order(toc_value: Option<&toml::Value>) -> Vec<String> {
    let toc_value = match toc_value {
        Some(v) => v,
        None => return Vec::new(),
    };

    // Handle array format: toc = ["act1", "act2"]
    if let Some(arr) = toc_value.as_array() {
        return arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    // Handle table format: [toc] with order field
    if let Some(table) = toc_value.as_table()
        && let Some(order) = table.get("order").and_then(|v| v.as_array())
    {
        return order
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    Vec::new()
}

/// Extracts acts map from an acts value.
fn extract_acts(acts_value: Option<&toml::Value>) -> HashMap<String, toml::Value> {
    let acts_table = match acts_value.and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return HashMap::new(),
    };

    acts_table
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Collects all defined resources (bots, tables, media) for reference validation.
fn collect_resources(table: &toml::map::Map<String, toml::Value>) -> ResourceRegistry {
    let mut registry = ResourceRegistry::default();

    // Collect bots
    if let Some(bots) = table.get("bots").and_then(|v| v.as_table()) {
        registry.bots = bots.keys().cloned().collect();
    }

    // Collect tables
    if let Some(tables) = table.get("tables").and_then(|v| v.as_table()) {
        registry.tables = tables.keys().cloned().collect();
    }

    // Collect media
    if let Some(media) = table.get("media").and_then(|v| v.as_table()) {
        registry.media = media.keys().cloned().collect();
    }

    registry
}

/// Registry of defined resources.
#[derive(Debug, Default)]
struct ResourceRegistry {
    bots: Vec<String>,
    tables: Vec<String>,
    media: Vec<String>,
    used_resources: std::cell::RefCell<std::collections::HashSet<String>>,
}

impl Clone for ResourceRegistry {
    fn clone(&self) -> Self {
        Self {
            bots: self.bots.clone(),
            tables: self.tables.clone(),
            media: self.media.clone(),
            used_resources: std::cell::RefCell::new(self.used_resources.borrow().clone()),
        }
    }
}

/// Validates references in an act value.
fn validate_act_references(
    act_name: &str,
    act_value: &toml::Value,
    resources: &ResourceRegistry,
    result: &mut ValidationResult,
) {
    // Check if act is a string reference
    if let Some(reference) = act_value.as_str() {
        validate_reference(act_name, reference, resources, result);
        return;
    }

    // Check if act is an array of references
    if let Some(arr) = act_value.as_array() {
        for item in arr {
            if let Some(reference) = item.as_str() {
                validate_reference(act_name, reference, resources, result);
            }
        }
    }
}

/// Validates a single resource reference.
fn validate_reference(
    act_name: &str,
    reference: &str,
    resources: &ResourceRegistry,
    result: &mut ValidationResult,
) {
    // Parse reference format: "type.name"
    if let Some((resource_type, resource_name)) = reference.split_once('.') {
        let exists = match resource_type {
            "bots" => resources.bots.contains(&resource_name.to_string()),
            "tables" => resources.tables.contains(&resource_name.to_string()),
            "media" => resources.media.contains(&resource_name.to_string()),
            "narrative" => true, // Narrative references validated separately
            _ => return,         // Unknown prefix, might be valid
        };

        if exists {
            // Track usage
            resources
                .used_resources
                .borrow_mut()
                .insert(reference.to_string());
        } else {
            let available = match resource_type {
                "bots" => &resources.bots,
                "tables" => &resources.tables,
                "media" => &resources.media,
                _ => return,
            };

            let suggestion = if available.is_empty() {
                format!(
                    "Define the {} resource:\n\n[{}.{}]\n...",
                    resource_type, resource_type, resource_name
                )
            } else {
                format!(
                    "Available {}:\n  - {}\n\nDid you mean one of these? Or define '{}.{}'",
                    resource_type,
                    available.join("\n  - "),
                    resource_type,
                    resource_name
                )
            };

            result.add_error(ValidationError {
                kind: ValidationErrorKind::UndefinedReference,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(format!("acts.{}", act_name)),
                }),
                message: format!(
                    "Undefined reference '{}.{}' in act '{}'",
                    resource_type, resource_name, act_name
                ),
                suggestion: Some(suggestion),
            });
        }
    }
}

/// Validates a model name against known models.
fn validate_model_name(
    section: &toml::map::Map<String, toml::Value>,
    section_name: &str,
    result: &mut ValidationResult,
) {
    if let Some(model) = section.get("model").and_then(|v| v.as_str())
        && !KNOWN_MODELS.contains(&model)
    {
        // Try to find a close match for suggestions
        let suggestion = find_closest_model(model);

        result.add_warning(ValidationWarning {
                kind: ValidationWarningKind::UnknownModel,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(section_name.to_string()),
                }),
                message: if let Some(closest) = suggestion {
                    format!("Unknown model '{}'. Did you mean '{}'?", model, closest)
                } else {
                    format!(
                        "Unknown model '{}'. This may be a typo or a newer model not in the validator's list.",
                        model
                    )
                },
            });
    }
}

/// Finds the closest matching model name using simple string distance.
fn find_closest_model(model: &str) -> Option<&'static str> {
    let model_lower = model.to_lowercase();

    // First try exact substring match
    for known in KNOWN_MODELS {
        if known.to_lowercase().contains(&model_lower)
            || model_lower.contains(&known.to_lowercase())
        {
            return Some(known);
        }
    }

    // Try Levenshtein distance for close matches
    let mut best_match: Option<(&str, usize)> = None;
    for known in KNOWN_MODELS {
        let distance = levenshtein_distance(&model_lower, &known.to_lowercase());
        if distance <= 3 {
            // Allow up to 3 character differences
            if let Some((_, best_dist)) = best_match {
                if distance < best_dist {
                    best_match = Some((known, distance));
                }
            } else {
                best_match = Some((known, distance));
            }
        }
    }

    best_match.map(|(model, _)| model)
}

/// Simple Levenshtein distance calculation.
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(len2 + 1) {
        *val = j;
    }

    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[len1][len2]
}

/// Checks for unused resources and adds warnings.
fn check_unused_resources(resources: &ResourceRegistry, result: &mut ValidationResult) {
    let used = resources.used_resources.borrow();

    for bot in &resources.bots {
        let reference = format!("bots.{}", bot);
        if !used.contains(&reference) {
            result.add_warning(ValidationWarning {
                kind: ValidationWarningKind::UnusedResource,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(format!("bots.{}", bot)),
                }),
                message: format!("Bot '{}' is defined but never used", bot),
            });
        }
    }

    for table in &resources.tables {
        let reference = format!("tables.{}", table);
        if !used.contains(&reference) {
            result.add_warning(ValidationWarning {
                kind: ValidationWarningKind::UnusedResource,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(format!("tables.{}", table)),
                }),
                message: format!("Table '{}' is defined but never used", table),
            });
        }
    }

    for media in &resources.media {
        let reference = format!("media.{}", media);
        if !used.contains(&reference) {
            result.add_warning(ValidationWarning {
                kind: ValidationWarningKind::UnusedResource,
                location: Some(ValidationLocation {
                    line: 0,
                    column: 0,
                    section: Some(format!("media.{}", media)),
                }),
                message: format!("Media '{}' is defined but never used", media),
            });
        }
    }
}

/// Checks for circular dependencies in nested narrative references.
fn check_circular_dependencies(
    table: &toml::map::Map<String, toml::Value>,
    result: &mut ValidationResult,
) {
    // Build a dependency graph of narrative references
    let mut graph = DiGraph::<String, ()>::new();
    let mut node_map = HashMap::<String, NodeIndex>::new();

    // Helper to get or create node
    let mut get_node = |graph: &mut DiGraph<String, ()>, name: &str| -> NodeIndex {
        if let Some(&idx) = node_map.get(name) {
            idx
        } else {
            let idx = graph.add_node(name.to_string());
            node_map.insert(name.to_string(), idx);
            idx
        }
    };

    // For single narrative files, check if it references itself
    if let Some(narrative) = table.get("narrative").and_then(|v| v.as_table())
        && let Some(name) = narrative.get("name").and_then(|v| v.as_str())
    {
        let narrative_node = get_node(&mut graph, name);

        // Check all acts for self-references
        if let Some(acts) = table.get("acts").and_then(|v| v.as_table()) {
            for act_value in acts.values() {
                extract_narrative_refs(act_value)
                    .into_iter()
                    .for_each(|ref_name| {
                        let ref_node = get_node(&mut graph, &ref_name);
                        graph.add_edge(narrative_node, ref_node, ());
                    });
            }
        }
    }

    // Collect all narrative references from top-level acts (for multi-narrative or nested)
    if let Some(acts) = table.get("acts").and_then(|v| v.as_table()) {
        for (act_name, act_value) in acts {
            let act_node = get_node(&mut graph, act_name);

            // Check for narrative.* references in the act
            extract_narrative_refs(act_value)
                .into_iter()
                .for_each(|ref_name| {
                    let ref_node = get_node(&mut graph, &ref_name);
                    graph.add_edge(act_node, ref_node, ());
                });
        }
    }

    // Check for multi-narrative structure
    if let Some(narratives) = table.get("narratives").and_then(|v| v.as_table()) {
        for (narrative_name, narrative_value) in narratives {
            let narrative_node = get_node(&mut graph, narrative_name);

            if let Some(narrative_table) = narrative_value.as_table() {
                // Check acts within this narrative
                if let Some(acts) = narrative_table.get("acts").and_then(|v| v.as_table()) {
                    for act_value in acts.values() {
                        extract_narrative_refs(act_value)
                            .into_iter()
                            .for_each(|ref_name| {
                                let ref_node = get_node(&mut graph, &ref_name);
                                graph.add_edge(narrative_node, ref_node, ());
                            });
                    }
                }
            }
        }
    }

    // Find strongly connected components (cycles)
    let sccs = kosaraju_scc(&graph);

    for scc in sccs {
        if scc.len() > 1 {
            // This is a cycle involving multiple nodes
            let cycle_names: Vec<String> = scc.iter().map(|&idx| graph[idx].clone()).collect();

            result.add_error(ValidationError {
                kind: ValidationErrorKind::CircularDependency,
                location: None,
                message: format!(
                    "Circular dependency detected: {}",
                    cycle_names.join(" → ")
                ),
                suggestion: Some(
                    "Break the circular dependency by removing one of the narrative references or restructuring the acts.".to_string()
                ),
            });
        } else if scc.len() == 1 {
            // Check for self-reference
            let node = scc[0];
            if graph.neighbors(node).any(|n| n == node) {
                result.add_error(ValidationError {
                    kind: ValidationErrorKind::CircularDependency,
                    location: None,
                    message: format!("Self-referencing circular dependency in '{}'", graph[node]),
                    suggestion: Some("Remove the self-reference to break the cycle.".to_string()),
                });
            }
        }
    }
}

/// Extracts narrative references from an act value.
fn extract_narrative_refs(act_value: &toml::Value) -> Vec<String> {
    let mut refs = Vec::new();

    // Check string values for narrative.* references
    if let Some(s) = act_value.as_str()
        && let Some(name) = s.strip_prefix("narrative.")
    {
        refs.push(name.to_string());
    }

    // Check arrays for narrative references
    if let Some(arr) = act_value.as_array() {
        for item in arr {
            if let Some(s) = item.as_str()
                && let Some(name) = s.strip_prefix("narrative.")
            {
                refs.push(name.to_string());
            }
        }
    }

    // Check table format (act with prompt field)
    if let Some(table) = act_value.as_table() {
        if let Some(prompt) = table.get("prompt") {
            refs.extend(extract_narrative_refs(prompt));
        }
        // Check inputs array
        if let Some(inputs) = table.get("inputs").and_then(|v| v.as_array()) {
            for input in inputs {
                if let Some(s) = input.as_str()
                    && let Some(name) = s.strip_prefix("narrative.")
                {
                    refs.push(name.to_string());
                }
            }
        }
    }

    refs
}
