# TUI Event Loop Refactor - Clean Architecture

## Problem

Current event loop does too much, causing keyboard lag:
- HTTP requests in event loop
- State management blocking
- Tick processing blocking input
- Everything synchronous

## Solution: Actor-Based Architecture

### Thread 1: UI Loop (Main Thread)
**ONLY** handles:
- Keyboard input capture (instant)
- Rendering (60 FPS)
- Send user actions to background

```rust
loop {
    // Non-blocking event read
    if event::poll(Duration::from_millis(16))? {
        match event::read()? {
            Event::Key(key) => {
                // Send to background, don't process here
                tx.send(UserAction::KeyPress(key)).await?;
            }
        }
    }
    
    // Check for state updates from background
    if let Ok(new_state) = rx.try_recv() {
        state = new_state;
    }
    
    // Render with current state
    terminal.draw(|f| render(f, &state))?;
}
```

### Thread 2: State Manager
Handles:
- Processing user actions
- Updating AppState
- HTTP requests to MCP server
- Sending state updates back to UI

```rust
async fn state_manager(
    mut rx: Receiver<UserAction>,
    tx: Sender<AppState>,
) {
    let mut state = AppState::new();
    
    while let Some(action) = rx.recv().await {
        match action {
            UserAction::KeyPress(key) => {
                state.handle_key(key).await;
                tx.send(state.clone()).await?;
            }
            UserAction::SendMessage => {
                // HTTP request here, UI keeps responding
                client.send_message(&state.input).await?;
                state.clear_input();
                tx.send(state.clone()).await?;
            }
        }
    }
}
```

### Thread 3: Background Tick
Handles:
- Periodic tasks (spinner animation)
- Auto-refresh
- Status polling

```rust
async fn tick_manager(tx: Sender<TickEvent>) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;
        tx.send(TickEvent::Tick).await?;
    }
}
```

## Message Types

```rust
enum UserAction {
    KeyPress(KeyEvent),
    SendMessage,
    ChangeView(ViewMode),
    Quit,
}

enum TickEvent {
    Tick,
    Refresh,
}
```

## Benefits

1. **Instant keyboard response** - UI thread never blocks
2. **Clean separation** - Each task has single responsibility
3. **Scalable** - Can add more background tasks without affecting UI
4. **Testable** - Can test each component independently

## Implementation Steps

1. Create message types (UserAction, StateUpdate, TickEvent)
2. Split event loop into 3 tasks
3. Add channels for communication
4. Move HTTP client to state manager
5. Move tick logic to background task
6. Test keyboard latency

## Testing Strategy

Integration test that:
1. Spawns all 3 tasks
2. Sends rapid keyboard input
3. Measures time from input → state update
4. Should be < 16ms (60 FPS)
