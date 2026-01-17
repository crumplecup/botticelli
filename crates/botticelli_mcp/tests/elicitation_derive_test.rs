#![cfg(ignore_obsolete_tests)]

// OBSOLETE: This test file tested old derive macros for elicitation tools which
// have been replaced by rmcp handler patterns.
//
// The old trait-based tool pattern with derives has been replaced by:
// - Direct rmcp handler methods on BotticelliServer
// - Parameter/Result structs with standard derives
// - ToolRegistry delegating to server methods
//
// No custom derives are needed in the new architecture.
