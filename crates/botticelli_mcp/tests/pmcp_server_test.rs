#![cfg(ignore_obsolete_tests)]

// OBSOLETE: This test file tested the old pmcp server implementation which has
// been replaced by rmcp-based BotticelliServer.
//
// The pmcp protocol and server are no longer supported. The new architecture uses:
// - BotticelliServer (rmcp::ServerHandler)
// - rmcp protocol and transport layers
//
// See crates/botticelli_mcp/src/rmcp_server.rs for current implementation.
