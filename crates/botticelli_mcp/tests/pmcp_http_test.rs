#![cfg(ignore_obsolete_tests)]

// OBSOLETE: This test file tested the old pmcp HTTP server which has been
// replaced by rmcp HTTP transport.
//
// The old run_pmcp_http_server() function no longer exists. The new architecture uses:
// - BotticelliServer with rmcp HTTP transport
// - rmcp::transport::http module
//
// See crates/botticelli_mcp/src/rmcp_server.rs for current implementation.
