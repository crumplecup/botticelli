//! Tests for content resource.

#[cfg(feature = "database")]
#[test]
fn test_content_resource_creation() {
    use botticelli_mcp::ContentResource;

    let resource = ContentResource::new();
    assert_eq!(resource.uri_pattern(), "content://");
}
