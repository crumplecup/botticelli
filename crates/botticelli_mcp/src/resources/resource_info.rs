//! Resource metadata.

/// Information about a resource.
#[derive(Debug, Clone, derive_getters::Getters, derive_new::new)]
pub struct ResourceInfo {
    /// Resource URI
    uri: String,
    /// Resource name
    name: String,
    /// Resource description
    description: String,
    /// MIME type (optional)
    mime_type: Option<String>,
}
