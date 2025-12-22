# TUI Event Loop Architecture - Root Cause Analysis

**Date:** 2025-12-22
**Status:** Diagnosis Complete - Ready for Implementation

## Executive Summary

**Problem:** User experiences severe keyboard input lag despite proper async architecture.

**Root Cause:** `terminal.draw()` is synchronous terminal I/O that blocks the event loop on every keystroke.

**Solution:** Decouple input processing from rendering - update state immediately, render at fixed 60fps independently.

---

## Current Architecture (CORRECT but SLOW)

### What We Have ✅

```rust
// Separate event reader (good!)
tokio::spawn(async move {
    let mut reader = EventStream::new();
    loop {
        if let Some(Ok(event)) = reader.next().await {
            event_tx.send(Event::Key(key)).unwrap();
        }
    }
});

// Main loop with biased select (good!)
loop {
    tokio::select! {
        biased;
        Some(event) = event_rx.recv() => {
            self.state.handle_key(event)?;  // Fast (<1ms)
            self.render()?;  // BLOCKS HERE!
        }
    }
}
```

### The Bottleneck

```rust
fn render(&mut self) -> Result<()> {
    self.terminal.draw(|frame| {
        // Layout calculations
        // String formatting
        // Widget construction
        // THEN: Terminal I/O (writes escape codes to stdout)
    })?;  // <-- This is SYNCHRONOUS I/O
    Ok(())
}
```

**Why this is slow:**
- Terminal I/O is syscalls (`write()`)
- Can be 10-50ms depending on terminal emulator
- Over SSH: can be 100ms+
- **Every keystroke waits for previous render to complete**

---

## Why Tests Didn't Catch It

Our tests used `TestBackend` (in-memory buffer):

```rust
let backend = TestBackend::new(80, 24);  // RAM, not real terminal
let mut terminal = Terminal::new(backend)?;
terminal.draw(/* instant */)?;  // No actual I/O!
```

**Real terminal:**
```rust
let backend = CrosstermBackend::new(io::stdout());  // Real I/O
let mut terminal = Terminal::new(backend)?;
terminal.draw(/* 10-50ms */)?;  // Syscalls!
```

---

## Solution: Decouple Input from Rendering

### New Architecture

```
Input Events ────▶ Update State ────▶ Set dirty flag
     │                                      │
     │                                      │
     └──────────────────────────────────────┘
                (instant, no I/O)


Ticker (60fps) ──▶ Check dirty ──▶ Render ──▶ Clear dirty
                       │                         
                       └─ Skip if clean
```

### Implementation

```rust
pub async fn run(&mut self) -> TuiResult<()> {
    let mut ticker = tokio::time::interval(Duration::from_millis(16)); // 60fps
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    loop {
        tokio::select! {
            biased;
            
            // Priority 1: Input events (instant state update, NO render)
            Some(event) = self.event_rx.recv() => {
                let should_continue = self.handle_event(event).await?;
                // State updated, dirty flag set
                // NO terminal.draw() here!
                if !should_continue {
                    break;
                }
            }
            
            // Priority 2: MCP updates (instant, NO render)
            Some(msg) = self.mcp_rx.recv() => {
                self.state.handle_mcp_update(msg)?;
                // NO terminal.draw() here!
            }
            
            // Priority 3: Periodic render (ONLY place that calls terminal.draw)
            _ = ticker.tick() => {
                if self.state.needs_render() {
                    self.render()?;
                    self.state.clear_dirty();
                }
            }
        }
    }
    
    Ok(())
}
```

### State Changes

```rust
// In AppState
pub struct AppState {
    // ... existing fields ...
    dirty: bool,  // Track if render needed
}

impl AppState {
    pub fn handle_key(&mut self, key: KeyEvent) -> TuiResult<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.dirty = true;  // Mark for render
            }
            // ... other cases ...
        }
        Ok(())
    }
    
    pub fn needs_render(&self) -> bool {
        self.dirty
    }
    
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}
```

---

## Expected Results

### Before (Current)
```
Keystroke → handle_key (1ms) → render (20ms) → Display
                                 ↑
                              BLOCKS HERE
Total: 21ms per keystroke = ~47 keys/second max
```

### After (Decoupled)
```
Keystroke → handle_key (1ms) → Done
                               (no blocking)
Total: 1ms per keystroke = instant

Render loop (independent):
Every 16ms → Check dirty → Render if needed → Display
            (60 FPS)
```

**Latency:**
- Input processing: <1ms (instant)
- Visual feedback: 0-16ms delay (average 8ms)
- Total perceived lag: Imperceptible at 60fps

---

## Implementation Checklist

- [ ] Add `dirty: bool` flag to `AppState`
- [ ] Add `needs_render()` and `clear_dirty()` methods
- [ ] Set `dirty = true` in all state-modifying methods
- [ ] Remove `self.render()` from event handlers
- [ ] Keep `self.render()` ONLY in ticker branch
- [ ] Add `if needs_render()` guard in ticker
- [ ] Update tests to verify dirty flag behavior
- [ ] Create real-terminal benchmark
- [ ] User testing to confirm fix

---

## Alternative Considered: Async Rendering

We could spawn renders in background:

```rust
// Spawn render task
let (render_tx, mut render_rx) = mpsc::unbounded_channel();
tokio::spawn(async move {
    while let Some(state) = render_rx.recv().await {
        terminal.draw(|f| render(f, &state))?;
    }
});

// Main loop - non-blocking render
Some(event) = event_rx.recv() => {
    self.state.handle_key(event)?;
    render_tx.send(self.state.clone())?;  // Non-blocking
}
```

**Why not:**
- Requires cloning entire state or `Arc<RwLock<AppState>>`
- More complex
- Can queue up renders (wasted work)
- Decoupled approach is simpler and sufficient

---

## Success Criteria

1. **Subjective:** User reports keyboard feels instant
2. **Objective:** Instrumented logs show <1ms input handling
3. **Tests:** Real-terminal benchmark confirms improvement
4. **No regression:** All functionality still works

---

## Next Actions

1. Implement dirty flag in `AppState`
2. Refactor event loop to decouple
3. Run with `RUST_LOG=trace` and verify timings
4. User validation
5. If still slow: profile render methods individually
