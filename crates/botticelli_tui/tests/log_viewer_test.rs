//! IR tests for [`LogViewerScreen`] — no terminal required.

use botticelli_tui::screen::BotScreen;
use botticelli_tui::{BotScreenContext, BotTransition, LogFilter, LogViewerScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctx() -> BotScreenContext {
    BotScreenContext::mock()
}

fn sample_lines() -> Vec<String> {
    vec![
        "2026-06-10 10:00:00 INFO  startup complete".to_string(),
        "2026-06-10 10:00:01 DEBUG  tick".to_string(),
        "2026-06-10 10:00:02 WARN  rate limit approaching".to_string(),
        "2026-06-10 10:00:03 ERROR  connection failed".to_string(),
        "2026-06-10 10:00:04 INFO  retrying".to_string(),
    ]
}

// ── Construction / basic render ───────────────────────────────────────────────

#[test]
fn empty_lines_renders_without_panic() {
    let screen = LogViewerScreen::new(vec![]);
    let _tree = screen.to_tui_node();
}

#[test]
fn with_lines_renders_without_panic() {
    let screen = LogViewerScreen::new(sample_lines());
    let _tree = screen.to_tui_node();
}

#[test]
fn screen_name_is_log_viewer() {
    let screen = LogViewerScreen::new(vec![]);
    assert_eq!(screen.screen_name(), "Log Viewer");
}

// ── Scroll ────────────────────────────────────────────────────────────────────

#[test]
fn j_increments_offset() {
    let lines: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    let mut screen = LogViewerScreen::new(lines);
    // tail mode puts offset near end; scroll up first
    screen.handle_key(key(KeyCode::Char('k')), &ctx());
    let before = screen.offset();
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    assert!(screen.offset() >= before);
}

#[test]
fn k_decrements_offset() {
    let lines: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    let mut screen = LogViewerScreen::new(lines);
    let before = screen.offset();
    screen.handle_key(key(KeyCode::Char('k')), &ctx());
    assert!(screen.offset() <= before);
}

#[test]
fn capital_g_returns_to_tail() {
    let lines: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
    let mut screen = LogViewerScreen::new(lines);
    // scroll up
    screen.handle_key(key(KeyCode::PageUp), &ctx());
    // jump to tail
    screen.handle_key(key(KeyCode::Char('G')), &ctx());
    assert_eq!(screen.filter(), LogFilter::All); // filter unchanged
    // offset should be near end
    let lines_count: usize = 50;
    assert!(screen.offset() >= lines_count - 25);
}

// ── Filter ────────────────────────────────────────────────────────────────────

#[test]
fn f_cycles_filter() {
    let mut screen = LogViewerScreen::new(sample_lines());
    assert_eq!(screen.filter(), LogFilter::All);
    screen.handle_key(key(KeyCode::Char('f')), &ctx());
    assert_eq!(screen.filter(), LogFilter::Info);
    screen.handle_key(key(KeyCode::Char('f')), &ctx());
    assert_eq!(screen.filter(), LogFilter::Warn);
    screen.handle_key(key(KeyCode::Char('f')), &ctx());
    assert_eq!(screen.filter(), LogFilter::Error);
    screen.handle_key(key(KeyCode::Char('f')), &ctx());
    assert_eq!(screen.filter(), LogFilter::All);
}

// ── Transitions ───────────────────────────────────────────────────────────────

#[test]
fn r_emits_load_log_lines() {
    let mut screen = LogViewerScreen::new(vec![]);
    let t = screen.handle_key(key(KeyCode::Char('r')), &ctx());
    assert!(matches!(t, BotTransition::LoadLogLines));
}

#[test]
fn quit_key_emits_quit() {
    let mut screen = LogViewerScreen::new(vec![]);
    let t = screen.handle_key(key(KeyCode::Char('q')), &ctx());
    assert!(matches!(t, BotTransition::Quit));
}

#[test]
fn nav_1_goes_to_bots() {
    let mut screen = LogViewerScreen::new(vec![]);
    let t = screen.handle_key(key(KeyCode::Char('1')), &ctx());
    assert!(matches!(t, BotTransition::GoToBots));
}

// ── Callback ──────────────────────────────────────────────────────────────────

#[test]
fn on_log_lines_loaded_updates_content() {
    let mut screen = LogViewerScreen::new(vec![]);
    screen.on_log_lines_loaded(sample_lines());
    let _tree = screen.to_tui_node();
}

#[test]
fn on_log_lines_loaded_preserves_tail_offset() {
    let initial: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    let mut screen = LogViewerScreen::new(initial);
    // Already in tail mode; load more lines — offset should track the tail
    let more: Vec<String> = (0..60).map(|i| format!("line {i}")).collect();
    let before_offset = screen.offset();
    screen.on_log_lines_loaded(more);
    // offset should move forward since more lines were added
    assert!(screen.offset() >= before_offset);
}
