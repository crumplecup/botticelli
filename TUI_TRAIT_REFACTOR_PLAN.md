# TUI Trait-Based Refactor Plan

## Current State
- TUI has business logic mixed with UI
- Chat crate has MCP hosting but also UI concerns
- Need clean separation via traits

## Architecture Goals

### botticelli_interface (traits)
- `ChatHost` trait - MCP hosting interface
- `ConversationManager` trait - message/conversation management  
- `ToolExecutor` trait - tool execution interface

### botticelli_chat (business logic)
- Implements `ChatHost` trait
- Manages MCP client connections
- Coordinates LLM + tool execution loop
- NO UI CODE

### botticelli_tui (UI only)
- Depends on traits from `_interface`
- Takes `impl ChatHost` as dependency
- ONLY rendering and input handling
- NO business logic

## Implementation Steps

### Step 1: Complete trait definitions in _interface
- [x] Define `ChatHost` trait
- [ ] Define `ConversationManager` trait  
- [ ] Define `ToolExecutor` trait

### Step 2: Implement traits in _chat
- [ ] Implement `ChatHost` for chat backend
- [ ] Implement `ConversationManager`
- [ ] Implement `ToolExecutor`
- [ ] Wire up LLM + MCP integration

### Step 3: Refactor _tui to use traits
- [ ] Remove all business logic from TuiState
- [ ] Accept `Box<dyn ChatHost>` in TuiState::new()
- [ ] Update UI to call trait methods
- [ ] Preserve all working UI functionality

### Step 4: Wire it all together
- [ ] Update main.rs to instantiate chat backend
- [ ] Pass backend to TUI
- [ ] Test end-to-end

## Success Criteria
- [ ] `just check` passes
- [ ] `just chat` launches and works
- [ ] All tools visible to LLM
- [ ] Tool execution works
- [ ] No TODOs in implementation
- [ ] Clean separation of concerns
