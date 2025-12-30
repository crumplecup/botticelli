# RMCP Migration Status

## Completed ✅

### Phase 1: Infrastructure
- [x] Add rmcp dependencies to Cargo.toml
- [x] Create error types with derive_more
- [x] Create Echo tool with rmcp macros
- [x] Create ServerInfo tool
- [x] Implement BotticelliServer with builder pattern
- [x] Set up test infrastructure

### Phase 2: Tool Type Definitions
- [x] Migrate elicitation param/result types (ElicitBool, ElicitText, ElicitNumber, ElicitSelect)
- [x] Migrate narrative param/result types (CreateNarrative, ModifyNarrative, SaveNarrative)
- [x] Migrate scene param/result types  
- [x] Migrate validation param/result types
- [x] Migrate execution param/result types

### Phase 3: Tool Stubs
- [x] Create tools/narrative.rs with stubs for create/modify/save
- [x] Create tools/elicitation_tools.rs with stubs for all elicit_* tools
- [x] Implement save_narrative() with full filesystem operations
- [x] All tools compile and follow rmcp patterns

## In Progress 🚧

### Phase 4: Remove pmcp Dependencies
- [ ] Remove pmcp from Cargo.toml (blocked - need to migrate all tools first)
- [ ] Update transport layer to use rmcp exclusively

## Not Started 📋

### Phase 5: LLM Backend Integration
- [ ] Design trait-based LLM backend system
- [ ] Implement create_narrative with LLM backend
- [ ] Implement modify_narrative with LLM backend  
- [ ] Add backend selection logic

### Phase 6: Elicitation Session Management
- [ ] Design elicitation session architecture
- [ ] Implement elicit_bool with session management
- [ ] Implement elicit_text with session management
- [ ] Implement elicit_number with session management
- [ ] Implement elicit_select with session management

### Phase 7: Database Tool Migration
- [ ] Migrate query_content tool
- [ ] Update database integration patterns

### Phase 8: Testing & Validation
- [ ] Update tests for new rmcp patterns
- [ ] Integration tests for tool execution
- [ ] End-to-end narrative workflows
- [ ] Performance benchmarks

## Key Design Decisions

### Error Handling
✅ Using `rmcp::ErrorData` with `rmcp::model::ErrorCode`
✅ String literals work directly (no `.into()` needed)
✅ Format strings for dynamic messages work correctly

### Tool Patterns
✅ All tools use `#[tool]` macro
✅ All tools use `#[instrument]` for observability  
✅ Parameters use derive(Serialize, Deserialize, JsonSchema)
✅ Results use derive(Serialize, JsonSchema)

### Builder Patterns
✅ BotticelliServer uses builder pattern
✅ All types use derives (Getters, Setters, derive_new) where appropriate
✅ No public fields, proper encapsulation

## Next Immediate Steps

1. **LLM Backend Traits** - Design the trait sandwich for backend abstraction
2. **Narrative Generation** - Wire up create_narrative to use LLM backend trait
3. **Elicitation Sessions** - Design session management architecture
4. **Remove pmcp** - Once all tools migrated, remove pmcp dependency completely

## Blockers

None currently - all infrastructure in place for incremental migration.

## Notes

- The migration follows standards from CLAUDE.md throughout
- All code compiles cleanly (1 dead_code warning on unused metrics field)
- save_narrative is fully functional as a reference implementation
- Tool stubs clearly indicate what's needed for full implementation
- Ready for parallel work on LLM backends and elicitation sessions
