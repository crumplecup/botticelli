//! Test elicitation-derived tools are discoverable.

use elicitation::collect_all_elicit_tools;

// Import types to ensure they're linked into the test binary
use botticelli_cache::CacheEntry;
use botticelli_core::{BotState, BotStats, HealthStatus, Output};
use botticelli_storage::FileSystemStorage;

#[test]
fn test_elicit_tools_registered() {
    // Force types to be included in binary
    let _ = std::any::type_name::<BotState>();
    let _ = std::any::type_name::<CacheEntry>();
    let _ = std::any::type_name::<FileSystemStorage>();
    
    // Collect all elicit tools registered via inventory
    let tools = collect_all_elicit_tools();
    
    println!("\n🔧 Total elicit tools discovered: {}", tools.len());
    
    for tool in &tools {
        println!("  - {} (from {})", tool.type_name, tool.module_path);
    }
    
    // We expect at least 8 types from core, cache, storage, error
    // (BotState, BotStats, BotStatus, HealthStatus, Output, CacheEntry, 
    //  CommandCache, FileSystemStorage, plus error types with Elicit)
    assert!(
        tools.len() >= 8,
        "Expected at least 8 elicit tools, found {}",
        tools.len()
    );
    
    println!("\n✅ Elicit tools verified!");
}
