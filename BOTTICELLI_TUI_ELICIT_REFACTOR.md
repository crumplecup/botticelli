# botticelli_tui — elicit_ui Refactor Plan

**Status**: ✅ COMPLETE (all 8 phases)  
**Branch**: dev  
**Supercedes**: All archived TUI planning docs (see PLANNING_INDEX.md)

---

## Vision

Replace botticelli_tui's hand-rolled ratatui event loop and global `AppState` with
the elicit_ui IR pipeline proven in strictly_games. The result is a **bot operator
console** that:

- Renders identically on terminal (ratatui), desktop (egui), and web (leptos)
- Carries compile-time layout-correctness proofs (`Established<BotUiConsistent>`)
- Connects to a live `BotServer` for real bot control and metrics
- Persists narratives via `TomlNarrativeFile` serialization
- Is testable without a terminal (test the IR, not the pixels)

---

## Architecture

### Core pattern (from strictly_games)

```
BotScreen::to_verified_tree(viewport)
    └─ AccessKit IR  →  VerifiedTree (WcagVerified)
         └─ RatatuiBackend::render(&tree)  →  TuiNode tree
              └─ verified_draw(frame, area, &root)  →  Established<BotUiConsistent>

KeyEvent  →  BotScreen::handle_key()  →  BotTransition
    └─ BotController applies transition → new BotScreen
```

### BotScreen trait

```rust
pub trait BotScreen {
    fn to_verified_tree(&self, viewport: Viewport) -> VerifiedTree;
    fn handle_key(&mut self, key: KeyEvent, ctx: &BotScreenContext) -> BotTransition;
}
```

`BotScreenContext` carries read-only shared handles: `Arc<BotServer>`,
`Arc<MetricsCollector>`, `Arc<dyn BotticelliDriver>`, `Arc<dyn BotStorage>`.
Screens never mutate shared state directly — they return commands via `BotTransition`.

`storage` is optional in context (`Option<Arc<dyn BotStorage>>`); screens that need
it (Database Browser, Schedule history) degrade gracefully when absent.

### BotTransition enum

```rust
pub enum BotTransition {
    Stay,
    GoToBots,
    GoToChat,
    GoToNarratives,
    GoToNarrativeEditor { path: Option<PathBuf> },
    GoToDatabase,
    GoToSchedule,
    GoToLogViewer,
    GoToSettings,
    // Server commands (executed by controller, not screen)
    StartBot(BotKind),
    StopBot(BotKind),
    RestartBot(BotKind),
    SaveNarrative { path: PathBuf, content: TomlNarrativeFile },
    Quit,
}
```

### Proof chain

```rust
// Propositions
pub struct WcagVerified;      // from elicit_ui
pub struct RenderComplete;    // from elicit_ui
pub struct BotUiConsistent;   // botticelli-specific

// Proof aliases
type BotProof = Established<And<RenderComplete, NoOverflow>>;

// verified_draw returns proof credential
pub fn verified_draw(
    frame: &mut Frame,
    area: Rect,
    root: &TuiNode,
) -> Result<BotProof, LayoutError>;
```

---

## Screens

### 1. Bots (operator console — highest priority)

**What it shows:**
- Three rows: `generation_bot`, `curation_bot`, `posting_bot`
- Per-bot: status badge (Running / Idle / Error), last cycle timestamp,
  last cycle duration, next scheduled run
- Live metrics panel: total cycles, avg duration, error rate

**Keys:**
- `j`/`k` — select bot
- `s` — StartBot(selected)
- `x` — StopBot(selected)
- `r` — RestartBot(selected)
- `Enter` — drill into bot detail (last narrative output)

**Data source:** `BotServer` actor refs + `MetricsCollector`

---

### 2. Chat

**What it shows:**
- Message history with styled user / assistant bubbles
- Input line at bottom
- Model indicator (top-right)

**Keys:**
- Type to build input, `Enter` to send
- `Esc` to cancel input
- `↑`/`↓` to scroll history

**Data source:** `Arc<dyn BotticelliDriver>` via `BotScreenContext`

**Note:** Use `TuiCommunicator` from `elicit_ratatui` for the input widget — it
already handles `elicit_select` and `elicit_text` with single-keystroke selection
and prompt pane management.

---

### 3. Narrative Browser

**What it shows:**
- Left pane (30%): list of `.toml` files in narratives directory
- Right pane (70%): parsed `TomlNarrativeFile` preview — narrative keys, act
  count, act prompts (truncated)

**Keys:**
- `j`/`k` — navigate list
- `Enter` — open in Narrative Editor
- `n` — create new (opens Editor with blank)

---

### 4. Narrative Editor

**What it shows:**
- Title bar: file path (unsaved indicator `*`)
- Main area: narrative TOML rendered as structured form (acts, narrative keys)
- Status bar: validation result (from `validate_narrative_toml`)

**Keys:**
- `e` on an act — inline edit the prompt string
- `Ctrl+S` — serialize via `toml::to_string_pretty()` → write to disk
  → emits `SaveNarrative` transition
- `Esc` — back to browser (confirm if unsaved)

**Data source / persistence:** `TomlNarrativeFile` (serialize/deserialize),
`validate_narrative_toml` for live validation feedback.

---

### 5. Database Browser

**What it shows:**
- Mode 1 (Tables): fixed list of BotStorage logical tables —
  `narrative_executions`, `act_executions`, `actor_states`, `actor_executions`,
  `content`, `post_history`, `model_responses`
- Mode 2 (Content): paginated row data rendered as a ratatui `Table` widget;
  each row is a `serde_json::Value` from the typed record's `content_json`
  or serialized as JSON

**Keys:**
- `Enter` — drill into table content
- `Esc` — back to table list
- `j`/`k` / `PgUp`/`PgDn` — navigate rows

**Data source:** `Arc<dyn BotStorage>` from `BotScreenContext` (optional —
shows "storage not configured" message when absent). Calls the appropriate
sub-trait method per table:
- `narrative_executions` → `NarrativeStore::list_narrative_executions`
- `actor_states` → `ActorStateStore::list_actor_states`
- `content` → `ContentStore::list_content("content", limit)`
- etc.

No SQL schema mode — BotStorage tables have typed structs, not inferred columns.

---

### 6. Schedule

**What it shows:**
- Left (45%): list of scheduled tasks with next-run time and enabled badge
- Right (55%): selected task detail — cron expression, last run, history

**Keys:**
- `t` — toggle task enabled/disabled
- `r` — run now (immediate trigger)

**Data source:**
- Live task list: `BotServer::schedule` (running `Schedule` structs from `botticelli_server`)
- Execution history: `ActorStateStore::list_actor_executions` via `Arc<dyn BotStorage>`

---

### 7. Log Viewer

**What it shows:**
- Tail of `botticelli-server.log` (the file written by `--log-file`)
- Auto-scroll to bottom, pause on any key
- Level filter: All / INFO / WARN / ERROR (toggle with `f`)
- Search bar (`/` to open, `Esc` to close)

**Keys:**
- `j`/`k` — scroll
- `G` — jump to bottom (resume tail)
- `f` — cycle log level filter
- `/` — search mode

**Data source:** File tail via `tokio::fs` + mpsc channel; no BotServer coupling.

---

### 8. Settings

**What it shows:**
- Model selector (list of available drivers)
- Log level override
- Narrative directory path
- Theme (colour palette toggle)

**Keys:** `j`/`k` navigate, `Enter` / arrow to edit, `Ctrl+S` persist to config file.

---

## Migration Phases

### Phase 1 — Dependencies + scaffolding

- [ ] Add `elicit_ui`, `elicit_ratatui` to `botticelli_tui/Cargo.toml`
- [ ] Add `elicit_ui` workspace dep if not already present
- [ ] Define `BotScreen` trait, `BotTransition` enum, `BotScreenContext` struct
- [ ] Define `BotUiConsistent` proof proposition
- [ ] Implement `verified_draw` (mirrors strictly_games `contracts.rs`)
- [ ] Stub `BotController` (owns `Box<dyn BotScreen>`, calls `to_verified_tree` +
      `verified_draw` each frame)
- [ ] Wire new controller into `bin/botticelli-tui.rs` replacing `minimal_event_loop`
- [ ] `just check botticelli_tui` — zero warnings

### Phase 2 — Delete dead code

- [ ] Delete `app.rs` (deprecated marker file)
- [ ] Delete `run.rs` (deprecated marker file)
- [ ] Remove global `AppState` — replace with per-screen state structs
- [ ] Remove `ViewMode` enum — `BotTransition` replaces it
- [ ] Remove `View` trait — `BotScreen` replaces it
- [ ] Remove `minimal_loop.rs` once controller is wired
- [ ] `just check botticelli_tui` — zero warnings

### Phase 3 — Bot Control screen (highest value, live wire)

- [ ] Implement `BotStatusScreen` with `to_verified_tree`
- [ ] Test IR in `tests/bot_status_ir_test.rs` (no terminal needed)
- [ ] Wire `BotScreenContext` to real `BotServer` (pass `Arc<BotServer>`)
- [ ] Implement `StartBot` / `StopBot` / `RestartBot` transitions in controller
- [ ] `just test-package botticelli_tui`

### Phase 4 — Chat screen

- [ ] Implement `ChatScreen` with `TuiCommunicator` for input widget
- [ ] Async response via mpsc channel (same pattern as current `minimal_loop`)
- [ ] Test with pre-baked `MockDriver`

### Phase 5 — Narrative Browser + Editor ✅

- [x] Implement `NarrativeBrowserScreen`
- [x] Implement `NarrativeEditorScreen` with `TomlNarrativeFile` round-trip
- [x] Wire `SaveNarrative` transition to disk write
- [x] Live validation via `validate_narrative_toml`
- [x] Test editor save/load in `tests/narrative_editor_test.rs`

### Phase 6 — Database Browser ✅

- [x] Add `storage: Option<Arc<dyn BotStorage>>` to `BotScreenContext`
- [x] Wire `open_storage_from_env()` in binary startup; pass into context
- [x] Add `botticelli_database` to `cli` feature in `botticelli_tui/Cargo.toml`
- [x] Implement `DatabaseBrowserScreen` with two-mode drill-down:
      table list → paginated row content (no SQL schema mode)
- [x] Wire to `Arc<dyn BotStorage>` in `BotScreenContext`
- [x] Graceful "storage not configured" fallback when `storage` is `None`
- [x] Test IR in `tests/database_browser_ir_test.rs` (14 tests, no terminal required)

### Phase 7 — Schedule + Log Viewer ✅

- [x] Implement `ScheduleScreen` with actor-state list from storage + per-task
      execution history; `r` refreshes, `Enter` loads executions for selected task
- [x] Implement `LogViewerScreen` with async file tail (500-line window),
      level filter (All/INFO+/WARN+/ERROR), `f` cycles filter, `G` jumps to tail

### Phase 8 — Settings + final cleanup ✅

- [x] Implement `SettingsScreen` — shows Narratives Dir, Log File, Storage Path,
      Log Level (cycle with Enter, save with `s` → writes `botticelli-settings.env`)
- [x] Remove 6 dead diesel-era `tui-table/server/all/last/demo/list-tables` justfile
      recipes; live `tui`, `tui-debug`, `tui-example`, `tui-chat` preserved
- [x] 115 tests passing, zero clippy warnings, all feature combos (default / no-default / cli) clean
- [x] PLANNING_INDEX.md updated

---

## Key Invariants

**No global mutable state.** Each `BotScreen` owns its own fields. `BotScreenContext`
is read-only (all `Arc<...>`). Mutation happens only via `BotTransition` executed
by `BotController`.

**Screens are pure.** `to_verified_tree` takes `&self` — no side effects, same
output for same state. This is what makes IR tests work.

**Controller executes commands.** When a transition like `StartBot` or `SaveNarrative`
arrives, `BotController` calls into `BotServer` / the filesystem. Screens never
do I/O.

**One proof per frame.** `verified_draw` must succeed before the frame is presented.
On proof failure (terminal too small), show a resize prompt that satisfies
`NoOverflow` by construction — same pattern as strictly_games.

---

## Files Created / Modified

```
crates/botticelli_tui/
├── Cargo.toml                           # add elicit_ui, elicit_ratatui
├── src/
│   ├── lib.rs                           # re-exports, new module structure
│   ├── bin/botticelli-tui.rs            # wire BotController
│   ├── controller.rs                    # BotController state machine  (NEW)
│   ├── context.rs                       # BotScreenContext              (NEW)
│   ├── screen.rs                        # BotScreen trait + BotTransition (NEW)
│   ├── contracts.rs                     # BotUiConsistent, verified_draw  (NEW)
│   ├── screens/
│   │   ├── mod.rs
│   │   ├── bots.rs                      # BotStatusScreen               (NEW)
│   │   ├── chat.rs                      # ChatScreen                    (NEW)
│   │   ├── narrative_browser.rs         # NarrativeBrowserScreen        (NEW)
│   │   ├── narrative_editor.rs          # NarrativeEditorScreen         (NEW)
│   │   ├── database.rs                  # DatabaseBrowserScreen         (NEW)
│   │   ├── schedule.rs                  # ScheduleScreen                (NEW)
│   │   ├── log_viewer.rs                # LogViewerScreen               (NEW)
│   │   └── settings.rs                  # SettingsScreen                (NEW)
│   ├── app.rs                           # DELETE (deprecated)
│   ├── run.rs                           # DELETE (deprecated)
│   ├── commands.rs                      # DELETE (replaced by BotTransition)
│   ├── minimal_loop.rs                  # DELETE (replaced by BotController)
│   ├── state.rs                         # DELETE (replaced by per-screen state)
│   ├── view.rs                          # DELETE (replaced by BotScreen)
│   └── events.rs                        # KEEP or fold into controller
└── tests/
    ├── bot_status_ir_test.rs            # IR test (no terminal)           (NEW)
    ├── narrative_editor_test.rs         # save/load round-trip            (NEW)
    ├── chat_screen_test.rs              # mock driver chat test           (NEW)
    └── database_browser_ir_test.rs      # IR test with RedbStorage::in_memory (NEW)
```

---

## Dependencies to Add

```toml
# botticelli_tui/Cargo.toml — add to [features] cli = [...]
"dep:botticelli_database",   # open_storage_from_env(), RedbStorage for tests
```

```toml
# botticelli_tui/Cargo.toml — already present (added in Phase 1):
elicit_ui.workspace = true
elicit_ratatui = { workspace = true, features = ["runtime"] }
botticelli_server.workspace = true   # for BotServer, MetricsCollector
```

Storage is initialized in the binary:
```rust
// src/bin/botticelli-tui.rs
use botticelli_database::open_storage_from_env;

let storage = open_storage_from_env().await.ok();  // None if env not set
let ctx = BotScreenContext { storage, ..Default::default() };
```

---

## Testing Strategy

**IR tests** (no terminal, fast):
```rust
#[test]
fn bot_status_screen_renders_all_three_bots() {
    let screen = BotStatusScreen::new(mock_metrics());
    let tree = screen.to_verified_tree(Viewport::new(120, 40));
    // assert on node count, labels, status badges
    let nodes = tree.nodes();
    assert!(nodes.iter().any(|n| n.label() == "generation_bot"));
}
```

**Proof tests** (layout correctness):
```rust
#[test]
fn verified_draw_succeeds_at_minimum_terminal_size() {
    let mut terminal = TestBackend::new(80, 24);
    terminal.draw(|frame| {
        let root = /* build TuiNode from screen */;
        let proof = verified_draw(frame, frame.area(), &root);
        assert!(proof.is_ok());
    });
}
```

**Transition tests** (pure state machine):
```rust
#[test]
fn pressing_s_emits_start_bot() {
    let mut screen = BotStatusScreen::new(mock_metrics());
    screen.select_bot(BotKind::Generation);
    let ctx = BotScreenContext::mock();
    let transition = screen.handle_key(key('s'), &ctx);
    assert!(matches!(transition, BotTransition::StartBot(BotKind::Generation)));
}
```
