// OBSOLETE: This test file tested the old pmcp HTTP server which has been replaced
// by the new rmcp-based server architecture.
//
// The function `run_pmcp_http_server` no longer exists. The new architecture uses:
// - BotticelliServer (rmcp::ServerHandler)
// - rmcp's serve() method with stdio/HTTP transports
//
// If HTTP server lifecycle testing is needed in the future, this should be rewritten to:
// 1. Use BotticelliServer::builder().build()?
// 2. Use rmcp's HTTP transport layer
// 3. Test through rmcp protocol, not direct HTTP calls
//
// See crates/botticelli_mcp/src/rmcp_server.rs for current server implementation.
