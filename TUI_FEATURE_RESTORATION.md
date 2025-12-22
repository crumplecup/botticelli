# TUI Feature Restoration Plan

## Success! Minimal Loop Works Without Lag

The minimal event loop in `minimal_loop.rs` successfully achieves instant keyboard response with zero lag.

## Current State

✅ **Working:**
- Instant keyboard input with tokio::select!
- 60 FPS rendering without blocking input
- ChatView rendering with proper UI layout
- Input buffer display
- Basic text entry (typing characters)
- Backspace support
- Ctrl+C to quit
- **View switching (Tab key)** - cycles through all views
- **Status bar** - shows current view and key bindings

❌ **Missing Features:**
1. Message sending (Enter key clears input but doesn't send)
2. HTTP client connection to MCP server
3. Actual message display from conversations
4. Proper rendering for non-Chat views (Bots, Database, Schedule, etc.)
5. Background tasks for server communication
6. Error handling and display
7. Conversation management

## Architecture Principles (DO NOT VIOLATE)

1. **Hot loop ONLY handles:**
   - Keyboard events (instant)
   - Rendering (60 FPS)
   
2. **Background tasks handle:**
   - HTTP requests
   - State updates
   - Server communication
   - Any I/O operations

3. **Communication:**
   - Channels for async messaging
   - Never block the hot loop
   - Use tokio::spawn for background work

## Restoration Steps

### Step 1: Message Sending ✓ (Current)
- [x] Input buffer working
- [x] Enter key detected
- [ ] Send to HTTP client (next step)

### Step 2: HTTP Client Integration
- [ ] Create HTTP client connection
- [ ] Background task for sending messages
- [ ] Channel-based request/response
- [ ] Update state with responses
- [ ] Maintain lag-free input during requests

### Step 3: Message Display
- [ ] Fetch conversation messages from server
- [ ] Display in ChatView
- [ ] Scroll support
- [ ] Message formatting

### Step 4: View Switching
- [ ] Implement tab navigation
- [ ] Switch between Chat/Bots/Database/Schedule
- [ ] Maintain state per view
- [ ] Keep input responsive during switches

### Step 5: Background Tasks
- [ ] Periodic server polling
- [ ] Status updates
- [ ] Error notifications
- [ ] All using channels, never blocking input

## Testing Strategy

For each step:
1. Run `just chat rebuild`
2. Type rapidly in the input box
3. Verify ZERO LAG in keyboard response
4. Check logs for timing information
5. If lag appears, STOP and fix before continuing

## Key Files

- `crates/botticelli_tui/src/minimal_loop.rs` - The working event loop
- `crates/botticelli_tui/src/view.rs` - ChatView and other views
- `crates/botticelli_tui/src/state.rs` - AppState management
- `crates/botticelli_tui/src/bin/chat.rs` - Binary entry point

## Success Criteria

- Keyboard input remains instant (<10ms response) at all times
- Features work without degrading input responsiveness
- Logs show clear separation between hot loop and background tasks
- No blocking operations in the hot loop
