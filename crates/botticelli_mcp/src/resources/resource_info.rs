//! Resource metadata.

/// Information about a resource.
#[derive(Debug, Clone, derive_getters::Getters)]
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

impl ResourceInfo {
    /// Creates a new resource info.
    #[tracing::instrument(skip(uri, name, description, mime_type), fields(uri_len = uri.len(), name_len = name.len()))]
    pub fn new(
        uri: String,
        name: String,
        description: String,
        mime_type: Option<String>,
    ) -> Self {
        Self {
            uri,
            name,
            description,
            mime_type,
        }
    }
}
