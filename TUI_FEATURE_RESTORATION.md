# TUI Feature Restoration Progress

## Goal
Incrementally restore features to the minimal lag-free event loop while maintaining keyboard responsiveness.

## Architecture
- **UI Thread**: Handles keyboard input + rendering (instant response)
- **Background Task**: HTTP client communication with MCP server (non-blocking)
- **Message Passing**: `tokio::sync::mpsc` channels between threads

## Features Restored

### ✅ Phase 1: Core Input (COMPLETE)
- Keyboard input without lag (<100ms)
- Character append/delete
- Input buffer rendering

### ✅ Phase 2: View Management (COMPLETE)  
- Tab key to cycle through views
- BackTab for previous view
- Status bar showing current view

### ✅ Phase 3: Quit Handling (COMPLETE)
- Ctrl+C to quit
- Clean terminal restoration
- Proper async task cleanup

### ✅ Phase 4: HTTP Client Integration (COMPLETE)
- Background task spawned for HTTP communication
- MCP server connection established
- Non-blocking message sending via channels
- HTTP POST to `/chat` endpoint

### 🔄 Phase 5: Message Handling (IN PROGRESS)
- ✅ Send user messages through HTTP
- ⏳ Receive and display assistant responses
- ⏳ Update conversation state from responses
- ⏳ Handle streaming responses

## Current Status

**Working:**
- Typing is responsive with no lag
- Tab switching works
- Messages sent to MCP server via HTTP

**Next Steps:**
1. Parse HTTP response and extract assistant message
2. Add message to conversation state
3. Update UI to show assistant response
4. Add error handling for failed requests
5. Implement streaming response handling

## Test Protocol

For each feature addition:
1. Build with `just chat rebuild`
2. Test keyboard responsiveness
3. Check logs for timing info
4. Verify no regression in lag
5. Commit if successful

## Key Learnings

- **Minimal event loop**: Only keyboard + render in hot path
- **Background tasks**: All I/O must be non-blocking
- **Instrumentation**: Detailed logs essential for diagnosing issues
- **Incremental**: Add one feature at a time to isolate problems
