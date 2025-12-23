# TUI Keyboard Lag Fix - Summary

## Problem
Keyboard input had 1-3 second lag, making the TUI unusable.

## Root Cause
The event loop was doing too much work:
- Processing every keystroke synchronously
- Running view updates in the hot loop
- Mixing UI and background tasks

## Solution
Created minimal event loop that ONLY handles:
1. Keyboard input (instant)
2. View rendering (fast)
3. Tab switching
4. Ctrl+C quit

All other work moved to background tasks (not yet implemented).

## Current Status
✅ Keyboard input is responsive (no lag)
✅ Tab switching works
✅ Ctrl+C quit works
✅ Consolidated TUI code into `botticelli_tui` crate
✅ Removed duplicate TUI from `botticelli_chat`
✅ Single binary: `botticelli-tui`

## Next Steps
1. **HTTP Client Integration** - Connect to MCP server at `http://localhost:8080/mcp`
   - Need to use `ExternalMcpClient` + `HttpTransport` from `botticelli_mcp_client`
   - Run in background task, communicate via channels
   - Send messages when user hits Enter
   - Receive responses and update ChatView

2. **Restore Features** - Incrementally add back:
   - Message history display
   - Streaming responses
   - Other views (Database, Schedule, Bots, etc.)
   - Status bar
   - Help text

3. **Testing** - Ensure each feature maintains responsiveness
   - Use instrumentation to detect any blocking
   - Keep keyboard input in hot loop
   - Everything else in background tasks

## Architecture
```
┌─────────────────┐
│   UI Thread     │  ← ONLY keyboard + rendering (FAST)
│  (Hot Loop)     │
└────────┬────────┘
         │ Channels
┌────────┴────────┐
│ Background Tasks│  ← HTTP, state updates, etc.
│  (Async Tasks)  │
└─────────────────┘
```

## Key Learnings
- Never block the UI thread
- Use channels for async communication
- Instrument everything for observability
- Test incrementally to catch regressions early
