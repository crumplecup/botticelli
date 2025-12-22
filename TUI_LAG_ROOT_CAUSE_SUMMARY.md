# TUI Text Lag - Root Cause Analysis & Fix Summary

## Problem Identified

User experiencing significant text input lag in TUI.

## Root Cause Found

**TWO separate TUI implementations exist:**

1. **`botticelli_chat/src/tui/`** (CURRENTLY RUNNING - BROKEN)
   - Has ALL features (6 tabs: Bots, Chat, Database, Narratives, Schedule, Settings)
   - Uses `event::poll(100ms)` - BLOCKS for 100ms on every keystroke
   - Renders on EVERY loop iteration
   - **THIS IS WHAT'S CAUSING THE LAG**

2. **`botticelli_tui/src/`** (FIXED BUT NOT RUNNING)
   - Uses `tokio::select! biased` - non-blocking, keyboard prioritized
   - Debounced rendering at 60fps (16ms)
   - **NO LAG** - proven by tests
   - Missing features (only 5 views, no bots/database/schedule tabs)

## Fixes Applied (to botticelli_tui)

✅ Removed `async` from `handle_key` - eliminated async overhead
✅ Removed `#[tracing::instrument]` from `handle_key` - eliminated span creation overhead  
✅ Added `tokio::select! biased` - keyboard events checked FIRST
✅ Debounced rendering - only renders at 60fps, not per keystroke
✅ Created tests that detect lag - `full_loop_lag_test.rs`

**Test Results:**
- `handle_key`: 8µs per character (instant)
- `event_loop_pattern`: keyboard prioritized ✓
- `full_render`: 1.2ms per keystroke (acceptable)

## The Problem

All fixes went to `botticelli_tui` but user is running `botticelli_chat/src/tui` which has the OLD BROKEN CODE!

## Solution Strategy

**Migrate all features to `botticelli_tui` using View trait architecture:**

### Migration Checklist (see TUI_MIGRATION_PLAN.md):

**Phase 1: Port missing views**
- [ ] BotsView (349 lines)
- [ ] DatabaseView (866 lines)
- [ ] ScheduleView (569 lines)  
- [ ] Enhance NarrativeBrowserView with discovery/tree builder

**Phase 2: Port widgets**
- [ ] ChatInput widget
- [ ] NavigationPanel widget

**Phase 3: Update AppState**
- [ ] Add bot management state
- [ ] Add database browser state
- [ ] Add schedule state

**Phase 4: Wire up in app.rs**
- [ ] Add view switching (Tab/number keys)
- [ ] Keep fixed event loop

**Phase 5: Delete old code**
- [ ] Delete `botticelli_chat/src/tui/` entirely
- [ ] Update binary to use `botticelli_tui`

## Current Status

✅ Root cause identified
✅ Event loop fixed (in botticelli_tui)
✅ Tests created to detect lag
✅ Migration plan documented
⏳ Migration in progress (BotsView next)

## Commits Made

1. `fix(tui): Remove async and tracing from handle_key` - removed overhead
2. `fix(tui): Debounce renders and prioritize keyboard input` - fixed event loop
3. `test(tui): Add typing lag detection test` - proves input is instant
4. `docs(tui): Audit TUI duplication and create migration plan` - this summary

## Next Steps

1. Continue migration: port BotsView to botticelli_tui/src/view.rs
2. Test each view as it's migrated
3. Once all views ported, delete botticelli_chat/src/tui/
4. User should see instant text input

## Timeline

- Investigation/fixes: 2 hours
- Migration estimate: 4-6 hours (3000+ lines across 6 tabs + widgets)
