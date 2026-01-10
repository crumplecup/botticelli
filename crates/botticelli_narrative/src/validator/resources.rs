//! Resource collection and reference validation.

use botticelli_error::{ValidationError, ValidationErrorKind, ValidationLocation, ValidationResult};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

/// Registry of defined resources.
#[derive(Debug, Default)]
pub struct ResourceRegistry {
    pub(super) bots: Vec<String>,
    pub(super) tables: Vec<String>,
    pub(super) media: Vec<String>,
    pub(super) used_resources: std::cell::RefCell<HashSet<String>>,
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

/// Unit struct providing resource validation methods.
///
/// Groups resource-related validation functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct ResourceValidator;

impl ResourceValidator {
    /// Collects all defined resources (bots, tables, media) for reference validation.
    #[instrument(skip(table), fields(
        bots = tracing::field::Empty,
        tables = tracing::field::Empty,
        media = tracing::field::Empty
    ))]
    pub fn collect(table: &toml::map::Map<String, toml::Value>) -> ResourceRegistry {
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

        tracing::Span::current().record("bots", registry.bots.len());
        tracing::Span::current().record("tables", registry.tables.len());
        tracing::Span::current().record("media", registry.media.len());
        tracing::debug!(
            bots = registry.bots.len(),
            tables = registry.tables.len(),
            media = registry.media.len(),
            "Resources collected"
        );

        registry
    }

    /// Validates references in an act value.
    #[instrument(skip(act_value, resources, result), fields(act = %act_name))]
    pub fn validate_act_references(
        act_name: &str,
        act_value: &toml::Value,
        resources: &ResourceRegistry,
        result: &mut ValidationResult,
    ) {
        // Check if act is a string reference
        if let Some(reference) = act_value.as_str() {
            tracing::debug!(reference = %reference, "Validating string reference");
            Self::validate_reference(act_name, reference, resources, result);
            return;
        }

        // Check if act is an array of references
        if let Some(arr) = act_value.as_array() {
            tracing::debug!(count = arr.len(), "Validating array of references");
            for item in arr {
                if let Some(reference) = item.as_str() {
                    Self::validate_reference(act_name, reference, resources, result);
                }
            }
        }
    }

    /// Validates a single resource reference.
    #[instrument(skip(resources, result), fields(act = %act_name, reference = %reference))]
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
                _ => {
                    tracing::debug!(resource_type, "Unknown resource type, skipping");
                    return;
                }
            };

            if exists {
                tracing::debug!(resource_type, resource_name, "Valid reference");
                // Track usage
                resources
                    .used_resources
                    .borrow_mut()
                    .insert(reference.to_string());
            } else {
                tracing::error!(resource_type, resource_name, "Undefined reference");
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

                result.add_error(ValidationError::new(
                    ValidationErrorKind::UndefinedReference,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(format!("acts.{}", act_name)),
                    )),
                    format!(
                        "Undefined reference '{}.{}' in act '{}'",
                        resource_type, resource_name, act_name
                    ),
                    Some(suggestion),
                ));
            }
        }
    }
}
