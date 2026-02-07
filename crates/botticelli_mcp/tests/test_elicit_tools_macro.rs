//! Test that elicit_tools! macro works with our types

use botticelli_mcp::BotticelliServer;

#[tokio::test]
async fn test_elicit_tools_generated() {
    // Build server
    let server = BotticelliServer::builder().build().expect("Server build");
    
    // Get tool router
    let tool_router = server.get_tool_router();
    let tools = tool_router.list_all();
    
    println!("\n🔧 Total tools: {}", tools.len());
    
    // Look for elicit tools
    let elicit_tools: Vec<_> = tools
        .iter()
        .filter(|t| t.name.starts_with("elicit_"))
        .collect();
    
    println!("📝 Elicit tools found: {}", elicit_tools.len());
    for tool in &elicit_tools {
        println!("  - {}", tool.name);
    }
    
    // Should have enum types (Role, StopReason, FinishReason)
    assert!(
        tools.iter().any(|t| t.name == "elicit_role"),
        "elicit_role tool should be registered (enum type)"
    );
    
    assert!(
        tools.iter().any(|t| t.name == "elicit_stop_reason"),
        "elicit_stop_reason tool should be registered (enum type)"
    );
    
    // Should have struct types (Message, CacheKey)
    assert!(
        tools.iter().any(|t| t.name == "elicit_message"),
        "elicit_message tool should be registered (struct type)"
    );
    
    assert!(
        tools.iter().any(|t| t.name == "elicit_cache_key"),
        "elicit_cache_key tool should be registered (struct type)"
    );
    
    println!("\n✅ Elicit tools verified (enums and structs)!");
}
