//! IR tests for [`SettingsScreen`] — no terminal required.

use botticelli_tui::screen::BotScreen;
use botticelli_tui::{BotScreenContext, BotTransition, SettingsScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctx() -> BotScreenContext {
    BotScreenContext::mock()
}

fn screen() -> SettingsScreen {
    SettingsScreen::new(
        Some("/data/narratives".to_string()),
        Some("botticelli-server.log".to_string()),
        Some("./botticelli.redb".to_string()),
    )
}

// ── Render ────────────────────────────────────────────────────────────────────

#[test]
fn renders_without_panic() {
    let s = screen();
    let _tree = s.to_tui_node();
}

#[test]
fn renders_with_no_optional_values() {
    let s = SettingsScreen::new(None, None, None);
    let _tree = s.to_tui_node();
}

#[test]
fn screen_name_is_settings() {
    let s = screen();
    assert_eq!(s.screen_name(), "Settings");
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[test]
fn j_moves_selection_down() {
    let mut s = screen();
    assert_eq!(s.selected(), 0);
    s.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(s.selected(), 1);
}

#[test]
fn k_wraps_to_last() {
    let mut s = screen();
    s.handle_key(key(KeyCode::Char('k')), &ctx());
    assert_eq!(s.selected(), 3, "should wrap to last setting");
}

#[test]
fn j_wraps_to_first() {
    let mut s = screen();
    // go to last
    for _ in 0..3 {
        s.handle_key(key(KeyCode::Char('j')), &ctx());
    }
    assert_eq!(s.selected(), 3);
    s.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(s.selected(), 0, "should wrap from last to first");
}

// ── Log level cycling ─────────────────────────────────────────────────────────

#[test]
fn enter_on_log_level_cycles_value() {
    let mut s = screen();
    assert_eq!(s.selected(), 0, "log level is index 0");
    let before = s.log_level_idx();
    s.handle_key(key(KeyCode::Enter), &ctx());
    let after = s.log_level_idx();
    assert_ne!(after, before, "Enter should cycle to next level");
}

#[test]
fn enter_on_non_editable_row_stays() {
    let mut s = screen();
    s.handle_key(key(KeyCode::Char('j')), &ctx()); // move to narratives_dir
    assert_eq!(s.selected(), 1);
    let t = s.handle_key(key(KeyCode::Enter), &ctx());
    assert!(matches!(t, BotTransition::Stay));
}

#[test]
fn log_level_cycles_through_all_four() {
    let mut s = screen();
    let initial = s.log_level_idx();
    s.handle_key(key(KeyCode::Enter), &ctx());
    s.handle_key(key(KeyCode::Enter), &ctx());
    s.handle_key(key(KeyCode::Enter), &ctx());
    s.handle_key(key(KeyCode::Enter), &ctx());
    assert_eq!(s.log_level_idx(), initial, "full cycle returns to start");
}

// ── Save ──────────────────────────────────────────────────────────────────────

#[test]
fn s_key_emits_save_settings() {
    let mut s = screen();
    let t = s.handle_key(key(KeyCode::Char('s')), &ctx());
    assert!(
        matches!(t, BotTransition::SaveSettings { ref rust_log } if !rust_log.is_empty()),
        "s should emit SaveSettings"
    );
}

#[test]
fn save_settings_carries_current_level() {
    let mut s = screen();
    // cycle to debug (index 1)
    s.handle_key(key(KeyCode::Enter), &ctx()); // 0 -> 1
    let t = s.handle_key(key(KeyCode::Char('s')), &ctx());
    assert!(
        matches!(t, BotTransition::SaveSettings { ref rust_log } if rust_log == "debug"),
        "saved level should match currently selected"
    );
}

// ── Navigation shortcuts ──────────────────────────────────────────────────────

#[test]
fn q_quits() {
    let mut s = screen();
    let t = s.handle_key(key(KeyCode::Char('q')), &ctx());
    assert!(matches!(t, BotTransition::Quit));
}

#[test]
fn key_1_goes_to_bots() {
    let mut s = screen();
    let t = s.handle_key(key(KeyCode::Char('1')), &ctx());
    assert!(matches!(t, BotTransition::GoToBots));
}
