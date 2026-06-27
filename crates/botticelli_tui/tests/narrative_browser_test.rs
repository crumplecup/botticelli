//! Tests for [`NarrativeBrowserScreen`].

use std::fs;
use tempfile::TempDir;

use botticelli_tui::{BotScreen, BotScreenContext, BotTransition, NarrativeBrowserScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn make_screen(dir: Option<std::path::PathBuf>) -> NarrativeBrowserScreen {
    NarrativeBrowserScreen::new(dir)
}

// ── construction ─────────────────────────────────────────────────────────────

#[test]
fn no_dir_configured_shows_empty_list() {
    let screen = make_screen(None);
    assert!(screen.entries().is_empty());
}

#[test]
fn empty_dir_shows_no_entries() {
    let dir = TempDir::new().unwrap();
    let screen = make_screen(Some(dir.path().to_path_buf()));
    assert!(screen.entries().is_empty());
}

#[test]
fn non_toml_files_are_ignored() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("readme.md"), "# hello").unwrap();
    fs::write(dir.path().join("data.json"), "{}").unwrap();
    let screen = make_screen(Some(dir.path().to_path_buf()));
    assert!(screen.entries().is_empty());
}

#[test]
fn toml_files_are_listed() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("alpha.toml"), "[narrative]\n").unwrap();
    fs::write(dir.path().join("beta.toml"), "[narrative]\n").unwrap();
    let screen = make_screen(Some(dir.path().to_path_buf()));
    assert_eq!(screen.entries().len(), 2);
}

#[test]
fn entries_are_sorted_by_name() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("zzz.toml"), "[narrative]\n").unwrap();
    fs::write(dir.path().join("aaa.toml"), "[narrative]\n").unwrap();
    let screen = make_screen(Some(dir.path().to_path_buf()));
    assert_eq!(screen.entries()[0].name(), "aaa");
    assert_eq!(screen.entries()[1].name(), "zzz");
}

// ── navigation ────────────────────────────────────────────────────────────────

#[test]
fn j_moves_selection_down() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.toml"), "[narrative]\n").unwrap();
    fs::write(dir.path().join("b.toml"), "[narrative]\n").unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    assert_eq!(*screen.selected(), 0);
    screen.handle_key(key(KeyCode::Char('j')), &ctx);
    assert_eq!(*screen.selected(), 1);
}

#[test]
fn j_wraps_at_bottom() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.toml"), "[narrative]\n").unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    screen.handle_key(key(KeyCode::Char('j')), &ctx);
    assert_eq!(*screen.selected(), 0);
}

#[test]
fn k_moves_selection_up() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.toml"), "[narrative]\n").unwrap();
    fs::write(dir.path().join("b.toml"), "[narrative]\n").unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    screen.handle_key(key(KeyCode::Char('j')), &ctx);
    screen.handle_key(key(KeyCode::Char('k')), &ctx);
    assert_eq!(*screen.selected(), 0);
}

#[test]
fn k_does_not_go_below_zero() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.toml"), "[narrative]\n").unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    screen.handle_key(key(KeyCode::Char('k')), &ctx);
    assert_eq!(*screen.selected(), 0);
}

// ── transitions ───────────────────────────────────────────────────────────────

#[test]
fn esc_returns_go_to_bots() {
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(None);
    let t = screen.handle_key(key(KeyCode::Esc), &ctx);
    assert!(matches!(t, BotTransition::GoToBots));
}

#[test]
fn enter_on_empty_list_returns_editor_with_no_path() {
    let dir = TempDir::new().unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    let t = screen.handle_key(key(KeyCode::Enter), &ctx);
    assert!(matches!(
        t,
        BotTransition::GoToNarrativeEditor { path: None }
    ));
}

#[test]
fn enter_on_entry_returns_editor_with_path() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("my.toml"), "[narrative]\n").unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(Some(dir.path().to_path_buf()));
    let t = screen.handle_key(key(KeyCode::Enter), &ctx);
    assert!(matches!(
        t,
        BotTransition::GoToNarrativeEditor { path: Some(_) }
    ));
}

#[test]
fn n_opens_new_wizard() {
    let ctx = BotScreenContext::mock();
    let mut screen = make_screen(None);
    let t = screen.handle_key(key(KeyCode::Char('n')), &ctx);
    assert!(matches!(
        t,
        BotTransition::GoToNarrativeWizard { path: None }
    ));
}

// ── IR ────────────────────────────────────────────────────────────────────────

#[test]
fn screen_name_is_narratives() {
    use botticelli_tui::BotScreen;
    let screen = make_screen(None);
    assert_eq!(screen.screen_name(), "Narratives");
}

#[test]
fn to_tui_node_does_not_panic_with_no_dir() {
    use botticelli_tui::BotScreen;
    let screen = make_screen(None);
    let _ = screen.to_tui_node();
}

#[test]
fn to_tui_node_does_not_panic_with_entries() {
    use botticelli_tui::BotScreen;
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("x.toml"), "[narrative]\n").unwrap();
    let screen = make_screen(Some(dir.path().to_path_buf()));
    let _ = screen.to_tui_node();
}
