//! Tests for narrative resource.

use botticelli_interface::McpResource;
use botticelli_mcp::NarrativeResource;

#[test]
fn test_narrative_resource_creation() {
    let resource = NarrativeResource::new();
    assert_eq!(resource.uri_pattern(), "narrative://");
}

#[test]
fn test_narrative_resource_with_directory() {
    let resource = NarrativeResource::with_directory("/custom/path");
    assert_eq!(resource.uri_pattern(), "narrative://");
}
