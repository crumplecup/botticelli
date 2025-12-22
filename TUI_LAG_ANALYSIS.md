# TUI Keyboard Lag Analysis

## Current Architecture (GOOD)

The TUI architecture is actually well-designed:

1. **Event Capture** - Uses `crossterm::EventStream` (async, non-blocking)
2. **Channel Communication** - Events flow through unbounded channels
3. **Biased Select** - Keyboard events have priority in `tokio::select!`
4. **Deferred Rendering** - Marks `needs_render = true`, renders at 60fps ticker
5. **Batching** - Multiple keystrokes batch into single render
6. **Spawn Blocking** - Heavy work (LLM calls) properly spawned to `spawn_blocking`

## Key Code Paths

### Fast Path (Character Input)
```
EventStream → channel → handle_event → handle_key → append_input
```
This should be <1ms based on instrumentation.

### Potentially Slow Path (Enter Key)
```
handle_key → send_message_with_orchestration → spawn_blocking
```
The spawn is async, shouldn't block UI.

## Instrumentation Present

- `tui.rs` - Event processing timing
- `state.rs` - `handle_key` timing with 1ms warning threshold
- EventStream logs slow operations >500μs

## Possible Causes

1. **Terminal I/O Slowness** - `terminal.draw()` might be slow
   - Check: Add timing around `terminal.draw()` call
   - Already logged in tui.rs:182-186

2. **View Rendering Complexity** - Complex views taking >16ms
   - Check logs for "Render took" warnings
   - Currently should warn if >16ms

3. **System-Level Issues**
   - Terminal emulator performance
   - System load
   - I/O contention

4. **Lock Contention** (unlikely but possible)
   - Chat host mutex held too long
   - Would show up in spawn_blocking logs

## Next Steps

### 1. Capture Real User Session Logs

Run with full tracing:
```bash
RUST_LOG=botticelli_tui=trace cargo run --release --bin botticelli-chat
```

Type rapidly and check `botticelli-chat.log` for:
- "Event handling took" >5ms warnings
- "Render took" >16ms warnings  
- "handle_key SLOW" >1ms warnings
- "Channel send SLOW" >100μs warnings

### 2. If Logs Show Fast Operations But UX is Laggy

The problem is NOT in our code - it's:
- Terminal emulator (try different terminal)
- System resources (check CPU/memory)
- Display rendering pipeline

### 3. If Logs Show Slow Operations

Focus on the specific slow operation identified in logs.

## Test Strategy

### Integration Test Needed

Test that measures ACTUAL terminal I/O performance:
1. Create real terminal backend (not TestBackend)
2. Send rapid keystrokes through actual crossterm
3. Measure end-to-end latency from keystroke to screen update
4. Assert <50ms latency (human perception threshold)

This test doesn't exist yet because it requires:
- Real terminal (not just in-memory buffers)
- Actual crossterm event simulation
- Screen capture/comparison

## Conclusion

The architecture is sound. The lag is either:
1. **Measurable** - Will show in logs as SLOW warnings
2. **Perceptual** - Terminal/system issue outside our control

Next action: **Run instrumented session and analyze logs**
