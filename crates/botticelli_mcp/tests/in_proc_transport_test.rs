#![cfg(ignore_obsolete_tests)]

// OBSOLETE: This test file used the old pmcp InProcessTransport which has been
// replaced by rmcp's transport layer.
//
// The InProcTransport type no longer exists. The new architecture uses:
// - rmcp::transport for transport layer abstractions
// - BotticelliServer (rmcp::ServerHandler)
//
// If in-process transport testing is needed, rewrite using rmcp APIs.
