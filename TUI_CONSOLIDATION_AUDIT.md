# TUI Consolidation Audit - Dec 22, 2024

## Problem
Two TUI implementations exist:
1. `botticelli_chat/src/tui/` - Has all features, BROKEN event loop (lag)
2. `botticelli_tui/src/` - Has FIXED event loop, missing features

## Features in botticelli_chat/src/tui (OLD - LAGGY)

### Tabs (6 total):
- ✅ Bots tab - manage bots
- ✅ Chat tab - chat interface  
- ✅ Database tab - browse database
- ✅ Narratives tab - browse/edit narratives (+ discovery, tree builder)
- ✅ Schedule tab - manage schedules
- ✅ Settings tab - settings

### Widgets:
- ✅ ChatInput widget
- ✅ NavigationPanel widget

### Event Loop:
- ❌ Uses `event::poll(100ms)` - BLOCKS, CAUSES LAG
- ❌ Renders on every loop iteration
- ❌ No prioritization

## Features in botticelli_tui/src (NEW - FAST)

### Views (5 total):
- ✅ ChatView - simple chat
- ✅ NarrativeBrowserView - browse narratives
- ✅ NarrativeEditorView - edit narratives
- ✅ ConversationHistoryView - view history
- ✅ SettingsView - settings

### Event Loop:
- ✅ Uses `tokio::select! biased` - NO BLOCKING
- ✅ Debounced rendering (60fps)
- ✅ Keyboard prioritized over ticks
- ✅ No lag

### Missing:
- ❌ No bots tab
- ❌ No database tab
- ❌ No schedule tab
- ❌ No narrative discovery
- ❌ Simpler (fewer features)

## Decision

**OPTION: Fix event loop in botticelli_chat/src/tui/app.rs IN PLACE**

Why:
1. Preserves all 6 tabs and features
2. Fixes the lag immediately
3. Can consolidate architecture later
4. No feature loss

Steps:
1. Copy fixed event loop from `botticelli_tui/src/app.rs` 
2. Apply to `botticelli_chat/src/tui/app.rs`
3. Test all tabs still work
4. Delete `botticelli_tui` or mark as deprecated later
