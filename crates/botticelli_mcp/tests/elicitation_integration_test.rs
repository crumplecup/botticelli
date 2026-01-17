#![cfg(ignore_obsolete_tests)]

// OBSOLETE: This test file tested old pmcp-based elicitation integration which
// has been replaced by rmcp tool handlers.
//
// All elicitation tools are now rmcp handlers in BotticelliServer.
// Integration tests should use ToolRegistry.execute() to test tools.
//
// See narrative_generation_test.rs and narrative_tools_test.rs for current
// testing patterns with the new architecture.
