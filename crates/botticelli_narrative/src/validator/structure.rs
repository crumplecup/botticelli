//! Narrative structure validation.

use super::{
    extraction::DataExtractor,
    resources::{ResourceRegistry, ResourceValidator},
};
use botticelli_error::{
    ValidationError, ValidationErrorKind, ValidationLocation, ValidationResult,
};
use rmcp::tool;
use tracing::instrument;

/// Unit struct providing structure validation methods.
///
/// Groups structure-related validation functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct StructureValidator;

impl StructureValidator {
    /// Validates a single narrative structure.
    #[instrument(skip(table, resources, result), fields(
        toc_length = tracing::field::Empty,
        act_count = tracing::field::Empty
    ))]
    #[tool]
    pub fn validate_single(
        table: &toml::map::Map<String, toml::Value>,
        resources: &ResourceRegistry,
        result: &mut ValidationResult,
    ) {
        // Check for toc
        if !table.contains_key("toc") {
            tracing::error!("Missing [toc] section");
            result.add_error(ValidationError::new(
                ValidationErrorKind::MissingSection,
                None,
                "Missing [toc] section".to_string(),
                Some("Add a table of contents:\n\n[toc]\norder = [\"act1\", \"act2\"]".to_string()),
            ));
            return;
        }

        // Get toc.order
        let toc_order = DataExtractor::toc_order(table.get("toc"));
        tracing::Span::current().record("toc_length", toc_order.len());

        if toc_order.is_empty() {
            tracing::error!("Empty table of contents");
            result.add_error(ValidationError::new(
                ValidationErrorKind::EmptyToc,
                None,
                "Table of contents is empty".to_string(),
                Some("Add at least one act to toc.order:\n\n[toc]\norder = [\"act1\"]".to_string()),
            ));
            return;
        }

        // Get acts
        let acts = DataExtractor::acts(table.get("acts"));
        tracing::Span::current().record("act_count", acts.len());
        tracing::debug!(
            toc_length = toc_order.len(),
            act_count = acts.len(),
            "Validating single narrative"
        );

        // Validate each act in toc.order exists and has valid references
        for act_name in &toc_order {
            if !acts.contains_key(act_name.as_str()) {
                tracing::error!(act = %act_name, "Act in toc not found in [acts]");
                result.add_error(ValidationError::new(
                    ValidationErrorKind::MissingAct,
                    None,
                    format!("Act '{}' referenced in toc.order does not exist", act_name),
                    Some(format!(
                        "Add the act:\n\n[acts.{}]\nprompt = \"...\"",
                        act_name
                    )),
                ));
            } else {
                // Validate references in act
                if let Some(act_value) = acts.get(act_name.as_str()) {
                    ResourceValidator::validate_act_references(
                        act_name, act_value, resources, result,
                    );
                }
            }
        }
    }

    /// Validates multi-narrative structure.
    #[instrument(skip(table, resources, result), fields(
        narrative_count = tracing::field::Empty
    ))]
    #[tool]
    pub fn validate_multi(
        table: &toml::map::Map<String, toml::Value>,
        resources: &ResourceRegistry,
        result: &mut ValidationResult,
    ) {
        let narratives = match table.get("narratives").and_then(|v| v.as_table()) {
            Some(n) => n,
            None => {
                tracing::debug!("No [narratives] table found");
                return;
            }
        };

        tracing::Span::current().record("narrative_count", narratives.len());
        tracing::debug!(count = narratives.len(), "Validating multi-narrative file");

        let shared_acts = DataExtractor::acts(table.get("acts"));

        for (narrative_name, narrative_value) in narratives {
            let narrative_table = match narrative_value.as_table() {
                Some(t) => t,
                None => continue,
            };

            // Each narrative must have a toc
            let toc_order = DataExtractor::toc_order(narrative_table.get("toc"));
            if toc_order.is_empty() {
                result.add_error(ValidationError::new(
                    ValidationErrorKind::EmptyToc,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(format!("narratives.{}", narrative_name)),
                    )),
                    format!("Narrative '{}' has empty table of contents", narrative_name),
                    Some(format!(
                        "Add toc to narrative:\n\n[narratives.{}]\ntoc = [\"act1\"]",
                        narrative_name
                    )),
                ));
                continue;
            }

            // Get narrative-specific acts
            let narrative_acts = DataExtractor::acts(narrative_table.get("acts"));

            // Validate each act exists (either in shared or narrative-specific acts)
            for act_name in &toc_order {
                let act_value = narrative_acts
                    .get(act_name.as_str())
                    .or_else(|| shared_acts.get(act_name.as_str()));

                if act_value.is_none() {
                    result.add_error(ValidationError::new(
                        ValidationErrorKind::MissingAct,
                        Some(ValidationLocation::new(
                            0,
                            0,
                            Some(format!("narratives.{}", narrative_name)),
                        )),
                        format!(
                            "Act '{}' in narrative '{}' does not exist",
                            act_name, narrative_name
                        ),
                        Some(format!(
                            "Add the act under [acts] or [narratives.{}.acts]",
                            narrative_name
                        )),
                    ));
                } else if let Some(value) = act_value {
                    ResourceValidator::validate_act_references(act_name, value, resources, result);
                }
            }
        }
    }
}
