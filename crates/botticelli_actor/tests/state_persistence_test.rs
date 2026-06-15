//! Tests for BotStorageStatePersistence.

use botticelli_actor::BotStorageStatePersistence;
use botticelli_database::RedbStorage;
use botticelli_interface::BotStorage;
use botticelli_server::StatePersistence;
use std::sync::Arc;

fn make_storage() -> Arc<dyn BotStorage> {
    Arc::new(RedbStorage::in_memory().expect("in-memory redb"))
}

#[tokio::test]
async fn test_state_persistence_interface() {
    let persistence = BotStorageStatePersistence::new(make_storage());

    // Trait methods are available
    let result = persistence.load_state().await;
    assert!(result.is_ok(), "load_state should not error on empty store");
    assert!(result.unwrap().is_none(), "no state initially");
}

#[test]
fn test_state_persistence_construction() {
    let _persistence = BotStorageStatePersistence::new(make_storage());
}
