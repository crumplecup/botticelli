use botticelli_mcp::BotticelliServer;

#[tokio::test]
async fn test_core_tools_registered() {
    let server = BotticelliServer::builder().build().expect("Server build");
    let tool_router = server.get_tool_router();

    let tool_names = tool_router.list_all();

    println!("\n🔧 Total tools registered: {}", tool_names.len());
    for tool in &tool_names {
        println!(
            "  - {} : {}",
            tool.name,
            tool.description.as_deref().unwrap_or("(no description)")
        );
    }

    // Should have at least echo and server_info from core
    assert!(
        tool_names.iter().any(|t| t.name == "echo"),
        "echo tool should be registered"
    );
    assert!(
        tool_names.iter().any(|t| t.name == "server_info"),
        "server_info tool should be registered"
    );

    println!("\n✅ Core tools verified!");
}
