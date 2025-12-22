# TUI Migration Plan - Move to Trait Architecture

## Goal
Migrate all functionality from `botticelli_chat/src/tui/` to `botticelli_tui/src/` using the View trait architecture.

## Architecture

### Current botticelli_tui View Trait
```rust
pub trait View {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()>;
    fn handle_input(&self, key: KeyEvent, state: &AppState) -> TuiResult<Option<Command>>;
}
```

## Migration Checklist

### Phase 1: Port Missing Views (migrate tabs to views)

#### [ ] BotsView (from tabs/bots.rs - 382 lines)
- Features: List bots, start/stop, view status
- Input: arrow keys, enter, s/t/r for start/stop/restart

#### [ ] DatabaseView (from tabs/database.rs - 510 lines)  
- Features: Browse tables, view content, load schema
- Input: arrow keys, enter, l for load

#### [ ] ScheduleView (from tabs/schedule.rs - 326 lines)
- Features: View scheduled tasks, manage schedules
- Input: arrow keys, enter

#### [ ] Enhance NarrativeBrowserView (from tabs/narratives.rs - 673 lines)
- Current: Basic browser
- Add: Discovery, tree builder functionality
- Files to merge:
  - tabs/narratives/discovery.rs (217 lines)
  - tabs/narratives/tree_builder.rs (194 lines)

### Phase 2: Port Widgets

#### [ ] Migrate widgets/chat_input.rs
- Reusable input widget
- Add to botticelli_tui/src/widgets/

#### [ ] Migrate widgets/navigation_panel.rs  
- Tab/view navigation widget
- Add to botticelli_tui/src/widgets/

### Phase 3: Update AppState

#### [ ] Extend botticelli_tui::AppState
- Add bot management state
- Add database browser state  
- Add schedule state
- Keep clean separation

### Phase 4: Wire Up in App

#### [ ] Add view switching in app.rs
- Tab key cycles through views
- Number keys for direct access (1-6)

#### [ ] Keep fixed event loop
- tokio::select! biased
- 60fps debounced rendering
- Keyboard priority

### Phase 5: Remove Old Code

#### [ ] Delete botticelli_chat/src/tui/ entirely
#### [ ] Update botticelli_chat/src/lib.rs
#### [ ] Update binary to use botticelli_tui

## Implementation Order

1. **BotsView** - simplest, standalone functionality
2. **DatabaseView** - moderate, needs DB integration  
3. **ScheduleView** - moderate, needs schedule integration
4. **Enhance NarrativeBrowserView** - complex, multiple files
5. **Port widgets** - support code
6. **Clean up** - delete old code

## Testing Strategy

After each view migration:
- [ ] Compile successfully
- [ ] View renders without crash
- [ ] Keyboard input works
- [ ] No lag (verify with typing test)
- [ ] Original functionality preserved

## Notes

- Keep the FIXED event loop from botticelli_tui
- Use View trait pattern consistently
- State in AppState, rendering in View
- Commands for actions
