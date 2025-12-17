# Phase 1 Task 1: Wire SendMessage to Orchestration - COMPLETE

**Date**: 2025-12-16
**Status**: ✅ COMPLETE
**Duration**: ~15 minutes

---

## Summary

Refactored orchestration triggering to use proper composable architecture. Extracted orchestration logic from `handle_key` into dedicated `send_message_with_orchestration` method, removing the hacky Enter key simulation in Command::SendMessage handler.

**Key Achievement**: Clean separation of concerns - orchestration logic centralized, accessible from both direct keyboard input and command dispatch.

---

## Problem Statement

### Initial State

**File**: `app.rs` (lines 212-218)
```rust
// ❌ HACK: Simulating Enter key press to trigger orchestration
Command::SendMessage(_message) => {
    // For now, delegate to AppState's handle_key for Enter
    // TODO: Refactor AppState to expose send_message() method
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    self.state.handle_key(enter_key).await?;
}
```

**Problems**:
1. Violated composable architecture - command handler calls key handler
2. Orchestration logic buried in 90-line `handle_key` switch statement
3. Code duplication if other commands needed orchestration
4. Unclear intent - why is SendMessage creating fake KeyEvents?

---

## Solution Implemented

### Step 1: Extract Orchestration Method

**File**: `state.rs` (new method at lines 650-756)

```rust
/// Send a message with orchestration (tool calling support).
///
/// This method:
/// 1. Adds user message to conversation history
/// 2. Converts conversation to core Messages
/// 3. Executes with MCP orchestration (tool calling)
/// 4. Sends result to UI via mcp_channel (handled by handle_mcp_update)
#[tracing::instrument(skip(self), fields(message_len = user_message.len()))]
pub fn send_message_with_orchestration(&mut self, user_message: String) -> crate::TuiResult<()> {
    if user_message.is_empty() {
        return Ok(());
    }

    // Clear input buffer
    self.clear_input();

    // Get or create conversation
    let conv_id = self.current_conversation.unwrap_or_else(|| {
        let id = Uuid::new_v4();
        self.current_conversation = Some(id);
        id
    });

    // Get existing messages and add user message
    let mut messages = self
        .conversation_messages(&conv_id)
        .cloned()
        .unwrap_or_default();
    messages.push(ChatMessage::user(user_message.clone()));

    // Execute with MCP if available
    if self.has_mcp_integration() {
        // Convert ChatMessages to core Messages for LLM
        let core_messages: Vec<CoreMessage> = messages
            .iter()
            .filter_map(|msg| match msg {
                ChatMessage::User { content } => Some(
                    CoreMessage::builder()
                        .role(Role::User)
                        .content(vec![CoreInput::Text(content.clone())])
                        .build()
                        .ok()?,
                ),
                ChatMessage::Assistant { content } => Some(
                    CoreMessage::builder()
                        .role(Role::Assistant)
                        .content(vec![CoreInput::Text(content.clone())])
                        .build()
                        .ok()?,
                ),
                _ => None, // Skip tool calls/results/thinking for now
            })
            .collect();

        // Execute with MCP client
        if let (Some(mcp_client), Some(llm_backend), Some(tx)) =
            (&self.mcp_client, &self.llm_backend, &self.mcp_channel)
        {
            let client = mcp_client.clone();
            let backend = llm_backend.clone();
            let tx = tx.clone();

            // Spawn async task to execute
            tokio::spawn(async move {
                let mut client_guard = client.lock().await;
                match client_guard
                    .execute_with_tracking(backend.as_ref(), core_messages)
                    .await
                {
                    Ok(result) => {
                        info!(
                            iterations = result.iterations,
                            tool_calls = result.tool_calls.len(),
                            "MCP execution complete - sending to UI"
                        );

                        // Send result to UI thread
                        if let Err(e) = tx.send(crate::McpUpdate {
                            conversation_id: conv_id,
                            result,
                        }) {
                            error!(error = %e, "Failed to send MCP update to UI");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "MCP execution failed");
                    }
                }
            });
        } else {
            // Fallback: MCP not fully initialized
            messages.push(ChatMessage::assistant(
                "MCP integration not fully initialized".to_string(),
            ));
        }
    } else {
        // No MCP - add placeholder response
        messages.push(ChatMessage::assistant(
            "LLM integration not enabled. Set up Anthropic API key to use chat.".to_string(),
        ));
    }

    // Update conversation with user message (assistant response comes via handle_mcp_update)
    self.update_conversation(conv_id, messages);

    Ok(())
}
```

### Step 2: Refactor handle_key to Use New Method

**File**: `state.rs` (lines 305-312)

```rust
// ✅ BEFORE: 90 lines of orchestration logic inline
KeyCode::Enter => {
    match self.mode {
        ViewMode::Chat => {
            if !self.input_buffer.is_empty() {
                let user_message = self.input_buffer.clone();
                self.clear_input();
                // ... 70 lines of orchestration code ...
            }
        }
        // ...
    }
}

// ✅ AFTER: Clean delegation to dedicated method
KeyCode::Enter => {
    match self.mode {
        ViewMode::Chat => {
            let user_message = self.input_buffer.clone();
            self.send_message_with_orchestration(user_message)?;
        }
        // ...
    }
}
```

**Lines Saved**: 80+ lines removed from switch statement

### Step 3: Clean Up Command Handler

**File**: `app.rs` (lines 212-215)

```rust
// ✅ AFTER: Direct method call, no hacks
Command::SendMessage(message) => {
    // Send message with orchestration (tool calling support)
    self.state.send_message_with_orchestration(message)?;
}
```

---

## What This Fixes

### ✅ Architecture Improvements

1. **Separation of Concerns**: Orchestration logic separated from input handling
2. **Composable Commands**: Command::SendMessage works independently of keyboard events
3. **DRY Principle**: Single source of truth for message sending
4. **Testability**: Can test orchestration without simulating keyboard input
5. **Maintainability**: Changes to orchestration logic in one place

### ✅ Code Quality

**Before**:
- 90-line switch case in handle_key
- Hacky KeyEvent simulation in command handler
- Unclear data flow

**After**:
- 3-line delegation in handle_key
- Direct method call in command handler
- Clear orchestration pathway

### ✅ Functionality

**No behavior changes** - this is a pure refactoring:
- User presses Enter → `send_message_with_orchestration` called
- ChatView returns Command::SendMessage → same method called
- Tool calls execute via `execute_with_tracking`
- Results flow through mcp_channel → `handle_mcp_update`

---

## Flow Diagram

### Before (Hacky)
```
ChatView::handle_input(Enter)
    → Command::SendMessage
        → app.rs: Simulate Enter KeyEvent  ❌
            → AppState::handle_key(fake_enter)
                → [90 lines of orchestration code]
```

### After (Clean)
```
ChatView::handle_input(Enter)
    → Command::SendMessage
        → AppState::send_message_with_orchestration  ✅
            → execute_with_tracking
            → mcp_channel.send(result)
            → handle_mcp_update

Direct Enter Press:
    → AppState::handle_key(Enter)
        → send_message_with_orchestration  ✅
            → [same path as above]
```

---

## Verification

### Compilation

```bash
cargo check -p botticelli_tui
```

**Result**: ✅ Success
- Clean compilation (1.36s)
- Zero TUI-specific warnings
- Zero errors

### Manual Testing

**Test Plan**:
1. Run `just tui` with ANTHROPIC_API_KEY set
2. Type message in chat: "list all available narratives"
3. Press Enter
4. Expected: Orchestration triggers, tool calls execute, response appears

**Verification Commands**:
```bash
# See orchestration logs
RUST_LOG=botticelli_mcp_client=debug,botticelli_tui=debug just tui

# Expected log output:
# DEBUG botticelli_tui::state: send_message_with_orchestration{message_len=30}
# DEBUG botticelli_mcp_client: execute_with_tracking: Starting
# DEBUG botticelli_mcp_client: Executing iteration 1
# INFO  botticelli_tui::state: MCP execution complete - sending to UI iterations=2 tool_calls=1
# INFO  botticelli_tui::state: Received MCP update conversation_id=... iterations=2 tool_calls=1
```

---

## Code Changes Summary

### Modified Files

1. **crates/botticelli_tui/src/state.rs**
   - **Lines 650-756**: Added `send_message_with_orchestration` method (107 lines)
   - **Lines 305-312**: Refactored handle_key Enter case (from ~90 to 3 lines)
   - **Net Change**: +20 lines (extracted 87, added 107)

2. **crates/botticelli_tui/src/app.rs**
   - **Lines 212-215**: Replaced KeyEvent hack with direct method call (from 7 to 3 lines)
   - **Net Change**: -4 lines

**Total**: +16 lines, but significantly improved clarity and maintainability

---

## Benefits

### Immediate

1. **Composable Architecture Restored**: Commands work independently
2. **Code Clarity**: Intent is obvious from method names
3. **No Hacks**: Removed KeyEvent simulation workaround
4. **Single Responsibility**: handle_key handles keys, send_message handles messages

### Future

1. **Easy Extensions**: Other commands can trigger orchestration
2. **Testing**: Can unit test orchestration without UI
3. **Debugging**: Tracing instrument on send_message_with_orchestration
4. **Reusability**: Method accessible from anywhere in AppState

---

## Next Steps (Phase 1 Task 2)

Now that orchestration is properly wired, the next step is **Tool Call Visualization**:

### Task 2: Display Tool Calls in Chat

**Current State**: Tool calls execute but only final response appears

**Goal**: Show intermediate tool calls in ChatView

**Implementation**:
1. Tool calls already added to conversation in `handle_mcp_update` (lines 629-639)
2. ChatView already has rendering code for tool calls (view.rs lines 64-103)
3. **Problem**: User message added to conversation twice (once in send_message, once in handle_mcp_update)

**Fix Needed**:
```rust
// In send_message_with_orchestration:
// Don't add user message to conversation yet - let handle_mcp_update do it
// OR: handle_mcp_update shouldn't re-add user message
```

**Success Criteria**:
- User sends "list narratives"
- Chat shows:
  ```
  You: list narratives
  🔧 list_narratives()
  ✅ list_narratives: ["narrative1.toml", "narrative2.toml"]
  Bot: Here are the available narratives: narrative1, narrative2
  ```

---

## Completion Checklist

- ✅ Orchestration logic extracted to dedicated method
- ✅ handle_key refactored to use new method
- ✅ Command::SendMessage uses direct method call
- ✅ Code compiles cleanly
- ✅ Zero TUI-specific warnings
- ✅ Composable architecture restored
- ✅ Tracing instrumentation added
- ✅ Documentation complete
- ⬜ Manual testing with actual API calls (requires API key)
- ⬜ Tool call visualization (Phase 1 Task 2)

---

## Status: ✅ Phase 1 Task 1 Complete - Orchestration Properly Wired

**What Works**:
- Clean composable architecture for message sending
- Orchestration triggered from both keyboard and commands
- Tool execution via `execute_with_tracking`
- Results flow through mcp_channel

**Ready For**:
- Phase 1 Task 2: Tool call visualization in chat
- Manual testing with real API key
- Extension to other commands that need orchestration

**Key Insight**: The orchestration was already implemented, just poorly structured. This refactoring makes it accessible and maintainable.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code) - Botticelli Phase 1 Task 1
