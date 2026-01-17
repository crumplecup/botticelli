// OBSOLETE: This test file tested the old generic NarrativeRegistry<T> API
// which has been replaced by:
// 1. The non-generic NarrativeRegistry in narrative_creation.rs (for PartialNarrative)
// 2. Direct tool execution through ToolRegistry
//
// The generic registry pattern is no longer used in the current architecture.
// All narrative creation/modification now goes through:
// - create_narrative tool
// - modify_narrative tool
// - save_narrative tool
//
// These are tested in:
// - narrative_generation_test.rs
// - narrative_tools_test.rs
// - narrative_validation_test.rs
//
// If generic registry functionality is needed in the future, this file can be
// revived and updated to use the new ToolRegistry-based API.
