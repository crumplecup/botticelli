// TODO: Restore elicitation derive tests for elicitation 0.2 API
//
// The elicitation crate's 0.2 API changed from using ElicitationDialog trait
// to using rmcp::Peer<RoleClient>. This test needs to be updated to:
//
// 1. Create an rmcp client/server pair for testing
// 2. Use the new elicit() signature: fn elicit(client: &Peer<RoleClient>)
// 3. Test that #[derive(Elicit)] generates correct implementations
//
// The elicitation derive macros are still valuable and actively used in
// botticelli_chat. This is NOT obsolete - just needs updating to new API.
//
// See crates/botticelli_chat/tests/elicitation_test.rs for examples of
// testing elicitation with the current API (though that uses Elicitor trait,
// not the derived Elicitation trait).
//
// Key test cases to restore:
// - test_select_paradigm_with_enum: #[derive(Elicit)] on enum
// - test_survey_paradigm_with_struct: #[derive(Elicit)] on struct
// - test_enum_variants_mapped_to_options: Enum variant to choice mapping
// - test_bool_field_uses_confirmation: Bool field uses ask_confirmation
