//! Tests for ContentResource URI parsing and resource interface.

#[cfg(feature = "database")]
mod database_tests {
    use botticelli_database::RedbStorage;
    use botticelli_interface::BotStorage;
    use botticelli_mcp::{ContentResource, McpResource};
    use std::sync::Arc;

    fn make_resource() -> ContentResource {
        let storage: Arc<dyn BotStorage> =
            Arc::new(RedbStorage::in_memory().expect("in-memory redb"));
        ContentResource::new(storage)
    }

    #[tokio::test]
    async fn test_read_invalid_scheme() {
        let resource = make_resource();
        assert!(resource.read("invalid://uri").await.is_err());
    }

    #[tokio::test]
    async fn test_read_missing_id() {
        let resource = make_resource();
        assert!(resource.read("content://table").await.is_err());
    }

    #[tokio::test]
    async fn test_read_nonnumeric_id_is_valid_uuid() {
        let resource = make_resource();
        // UUIDs are strings — a valid UUID should parse and then return not-found
        let result = resource
            .read("content://content/550e8400-e29b-41d4-a716-446655440000")
            .await;
        // No content stored, so expect resource_not_found error, not a parse error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{}", err).contains("not found")
                || format!("{}", err).contains("Content not found"),
            "Expected resource-not-found error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_list_empty_storage() {
        let resource = make_resource();
        let resources = resource.list().await.expect("list should succeed");
        assert!(resources.is_empty());
    }
}
