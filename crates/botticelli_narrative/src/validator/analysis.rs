//! Analysis for unused resources and circular dependencies.

use super::resources::ResourceRegistry;
use botticelli_error::{
    ValidationError, ValidationErrorKind, ValidationLocation, ValidationResult, ValidationWarning,
    ValidationWarningKind,
};
use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use tracing::instrument;

/// Unit struct providing analysis methods.
///
/// Groups analysis-related validation functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct Analyzer;

impl Analyzer {
    /// Checks for unused resources and adds warnings.
    #[instrument(skip(resources, result), fields(
        unused_count = tracing::field::Empty
    ))]
    pub fn check_unused_resources(resources: &ResourceRegistry, result: &mut ValidationResult) {
        let used = resources.used_resources().borrow();
        let mut unused_count = 0;

        for bot in resources.bots() {
            let reference = format!("bots.{}", bot);
            if !used.contains(&reference) {
                unused_count += 1;
                tracing::debug!(bot = %bot, "Unused bot resource");
                result.add_warning(ValidationWarning::new(
                    ValidationWarningKind::UnusedResource,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(format!("bots.{}", bot)),
                    )),
                    format!("Bot '{}' is defined but never used", bot),
                ));
            }
        }

        for table in resources.tables() {
            let reference = format!("tables.{}", table);
            if !used.contains(&reference) {
                unused_count += 1;
                tracing::debug!(table = %table, "Unused table resource");
                result.add_warning(ValidationWarning::new(
                    ValidationWarningKind::UnusedResource,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(format!("tables.{}", table)),
                    )),
                    format!("Table '{}' is defined but never used", table),
                ));
            }
        }

        for media in resources.media() {
            let reference = format!("media.{}", media);
            if !used.contains(&reference) {
                unused_count += 1;
                tracing::debug!(media = %media, "Unused media resource");
                result.add_warning(ValidationWarning::new(
                    ValidationWarningKind::UnusedResource,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(format!("media.{}", media)),
                    )),
                    format!("Media '{}' is defined but never used", media),
                ));
            }
        }

        tracing::Span::current().record("unused_count", unused_count);
        tracing::debug!(unused_count, "Completed unused resource check");
    }

    /// Checks for circular dependencies in nested narrative references.
    #[instrument(skip(table, result), fields(
        narrative_count = tracing::field::Empty,
        edge_count = tracing::field::Empty,
        cycle_count = tracing::field::Empty
    ))]
    pub fn check_circular_dependencies(
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
                    Self::extract_narrative_refs(act_value)
                        .into_iter()
                        .for_each(|ref_name| {
                            let ref_node = get_node(&mut graph, &ref_name);
                            graph.add_edge(narrative_node, ref_node, ());
                        });
                }
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
                            Self::extract_narrative_refs(act_value)
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

        tracing::Span::current().record("narrative_count", node_map.len());
        tracing::Span::current().record("edge_count", graph.edge_count());
        tracing::debug!(
            narratives = node_map.len(),
            edges = graph.edge_count(),
            "Built dependency graph"
        );

        // Find strongly connected components (cycles)
        let sccs = kosaraju_scc(&graph);
        let mut cycle_count = 0;

        for scc in sccs {
            if scc.len() > 1 {
                // This is a cycle involving multiple nodes
                let cycle_names: Vec<String> = scc.iter().map(|&idx| graph[idx].clone()).collect();
                cycle_count += 1;
                tracing::error!(cycle = ?cycle_names, "Circular dependency detected");

                result.add_error(ValidationError::new(
                    ValidationErrorKind::CircularDependency,
                    None,
                    format!(
                        "Circular dependency detected: {}",
                        cycle_names.join(" → ")
                    ),
                    Some(
                        "Break the circular dependency by removing one of the narrative references or restructuring the acts.".to_string()
                    ),
                ));
            } else if scc.len() == 1 {
                // Check for self-reference
                let node = scc[0];
                if graph.neighbors(node).any(|n| n == node) {
                    cycle_count += 1;
                    tracing::error!(narrative = %graph[node], "Self-referencing circular dependency");
                    result.add_error(ValidationError::new(
                        ValidationErrorKind::CircularDependency,
                        None,
                        format!("Self-referencing circular dependency in '{}'", graph[node]),
                        Some("Remove the self-reference to break the cycle.".to_string()),
                    ));
                }
            }
        }

        tracing::Span::current().record("cycle_count", cycle_count);
        tracing::debug!(cycle_count, "Completed circular dependency check");
    }

    /// Extracts narrative references from an act value.
    #[instrument(skip(act_value), fields(ref_count = tracing::field::Empty))]
    fn extract_narrative_refs(act_value: &toml::Value) -> Vec<String> {
        let mut refs = Vec::new();

        // Check string values for narrative.* references
        if let Some(s) = act_value.as_str()
            && let Some(name) = s.strip_prefix("narrative.")
        {
            tracing::debug!(narrative = %name, "Found string narrative reference");
            refs.push(name.to_string());
        }

        // Check arrays for narrative references
        if let Some(arr) = act_value.as_array() {
            for item in arr {
                if let Some(s) = item.as_str()
                    && let Some(name) = s.strip_prefix("narrative.")
                {
                    tracing::debug!(narrative = %name, "Found array narrative reference");
                    refs.push(name.to_string());
                }
            }
        }

        // Check table format (act with prompt field)
        if let Some(table) = act_value.as_table() {
            if let Some(prompt) = table.get("prompt") {
                refs.extend(Self::extract_narrative_refs(prompt));
            }
            // Check inputs array
            if let Some(inputs) = table.get("inputs").and_then(|v| v.as_array()) {
                for input in inputs {
                    if let Some(s) = input.as_str()
                        && let Some(name) = s.strip_prefix("narrative.")
                    {
                        tracing::debug!(narrative = %name, "Found input narrative reference");
                        refs.push(name.to_string());
                    }
                }
            }
        }

        tracing::Span::current().record("ref_count", refs.len());
        refs
    }
}
