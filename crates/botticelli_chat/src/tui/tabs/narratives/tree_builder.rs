// Tree building for narrative navigation

use std::collections::HashMap;

#[cfg(feature = "tui")]
use tui_tree_widget::TreeItem;

use super::NarrativeEntry;

/// Builds tree items from discovered narratives
#[cfg(feature = "tui")]
pub fn build_tree_items(narratives: &[NarrativeEntry]) -> Vec<TreeItem<'static, String>> {
    use itertools::Itertools;

    let mut items = Vec::new();

    // Build "Recent" section (last 10 modified)
    let recent: Vec<_> = narratives
        .iter()
        .sorted_by_key(|n| std::cmp::Reverse(n.last_modified))
        .take(10)
        .map(|n| {
            let model_display = n
                .metadata
                .model
                .as_deref()
                .unwrap_or("default");
            let label = format!("{} ({})", n.name, model_display);
            TreeItem::new_leaf(n.path.to_string_lossy().to_string(), label)
        })
        .collect();

    if !recent.is_empty() {
        // Unwrap is safe because identifier and text are valid strings
        items.push(
            TreeItem::new("recent".to_string(), "📌 Recent".to_string(), recent)
                .unwrap_or_else(|_| TreeItem::new_leaf("error".to_string(), "Error creating Recent section".to_string()))
        );
    }

    // Group narratives by category
    let mut categories: HashMap<String, Vec<&NarrativeEntry>> = HashMap::new();
    for narrative in narratives {
        categories
            .entry(narrative.category.clone())
            .or_default()
            .push(narrative);
    }

    // Build category sections (sorted by name)
    for (category, mut cat_narratives) in categories.into_iter().sorted_by_key(|(cat, _)| cat.clone()) {
        // Sort narratives within category by name
        cat_narratives.sort_by(|a, b| a.name.cmp(&b.name));

        let children: Vec<_> = cat_narratives
            .into_iter()
            .map(|n| {
                TreeItem::new_leaf(n.path.to_string_lossy().to_string(), n.name.clone())
            })
            .collect();

        let count = children.len();
        let label = format!("📁 {} ({})", category, count);

        // Unwrap is safe because identifier and text are valid strings
        items.push(
            TreeItem::new(category.clone(), label, children)
                .unwrap_or_else(|_| TreeItem::new_leaf("error".to_string(), format!("Error creating {} section", category)))
        );
    }

    items
}

/// Finds a narrative by its path in the tree
pub fn find_narrative_by_path<'a>(
    narratives: &'a [NarrativeEntry],
    path_components: &[String],
) -> Option<&'a NarrativeEntry> {
    // Path components could be:
    // - ["recent", "<path>"]
    // - ["<category>", "<name>"]

    if path_components.is_empty() {
        return None;
    }

    // If in "recent" section, the second component is the full path
    if path_components[0] == "recent" && path_components.len() > 1 {
        let path_str = &path_components[1];
        return narratives.iter().find(|n| n.path.to_string_lossy() == path_str.as_str());
    }

    // Otherwise, match by category and name
    if path_components.len() > 1 {
        let category = &path_components[0];
        let name = &path_components[1];

        return narratives
            .iter()
            .find(|n| &n.category == category && &n.name == name);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn create_test_entry(name: &str, category: &str) -> NarrativeEntry {
        NarrativeEntry {
            path: PathBuf::from(format!("{}/{}.toml", category, name)),
            name: name.to_string(),
            metadata: super::super::NarrativeMetadata {
                name: name.to_string(),
                model: Some("claude-3-5-sonnet".to_string()),
                act_count: 3,
                description: Some("Test narrative".to_string()),
            },
            category: category.to_string(),
            last_modified: SystemTime::now(),
        }
    }

    #[test]
    fn test_find_narrative_by_category_and_name() {
        let narratives = vec![
            create_test_entry("showcase", "examples"),
            create_test_entry("welcome", "discord"),
        ];

        let found = find_narrative_by_path(&narratives, &["examples".to_string(), "showcase".to_string()]);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "showcase");

        let found = find_narrative_by_path(&narratives, &["discord".to_string(), "welcome".to_string()]);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "welcome");

        let not_found = find_narrative_by_path(&narratives, &["examples".to_string(), "nonexistent".to_string()]);
        assert!(not_found.is_none());
    }
}
