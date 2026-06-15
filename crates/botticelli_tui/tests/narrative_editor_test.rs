//! Tests for [`NarrativeEditorScreen`].

use std::fs;
use tempfile::TempDir;

use botticelli_tui::{
    BotScreen, BotScreenContext, BotTransition, EditorContent, NarrativeEditorScreen,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

// ── construction ─────────────────────────────────────────────────────────────

#[test]
fn new_file_has_new_content() {
    let screen = NarrativeEditorScreen::new_file();
    assert!(matches!(screen.content(), EditorContent::New));
}

#[test]
fn open_missing_file_shows_error() {
    let screen = NarrativeEditorScreen::open("/nonexistent/path/file.toml".into());
    assert!(matches!(screen.content(), EditorContent::Error(_)));
}

#[test]
fn open_invalid_toml_shows_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.toml");
    fs::write(&path, "not valid toml [[[").unwrap();
    let screen = NarrativeEditorScreen::open(path);
    assert!(matches!(screen.content(), EditorContent::Error(_)));
}

#[test]
fn open_valid_file_shows_loaded() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("good.toml");
    fs::write(&path, "[narrative]\n").unwrap();
    let screen = NarrativeEditorScreen::open(path);
    assert!(matches!(screen.content(), EditorContent::Loaded { .. }));
}

// ── validation summary ────────────────────────────────────────────────────────

#[test]
fn valid_file_shows_checkmark_in_summary() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ok.toml");
    let content = r#"
[narrative]
name = "test"
description = "A test narrative"

[toc]
order = ["greet"]

[acts]
greet = "Hello"
"#;
    fs::write(&path, content).unwrap();
    let screen = NarrativeEditorScreen::open(path);
    assert!(
        screen.validation_summary().starts_with('✓'),
        "expected valid summary, got: {}",
        screen.validation_summary()
    );
}

// ── navigation ────────────────────────────────────────────────────────────────

#[test]
fn esc_returns_go_to_narratives() {
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::new_file();
    let t = screen.handle_key(key(KeyCode::Esc), &ctx);
    assert!(matches!(t, BotTransition::GoToNarratives));
}

#[test]
fn j_increments_scroll() {
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::new_file();
    assert_eq!(*screen.scroll(), 0);
    screen.handle_key(key(KeyCode::Char('j')), &ctx);
    assert_eq!(*screen.scroll(), 1);
}

#[test]
fn k_does_not_go_below_zero() {
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::new_file();
    screen.handle_key(key(KeyCode::Char('k')), &ctx);
    assert_eq!(*screen.scroll(), 0);
}

// ── save transition ───────────────────────────────────────────────────────────

#[test]
fn ctrl_s_on_new_file_stays_because_no_path() {
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::new_file();
    let t = screen.handle_key(ctrl(KeyCode::Char('s')), &ctx);
    // No path yet — save is a no-op until the user gives a filename.
    assert!(matches!(t, BotTransition::Stay));
}

#[test]
fn ctrl_s_on_loaded_file_emits_save_narrative() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("save_me.toml");
    let content = r#"
[narrative]
name = "save"
description = "A save test"

[toc]
order = ["a"]

[acts]
a = "prompt"
"#;
    fs::write(&path, content).unwrap();
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::open(path.clone());
    assert!(
        matches!(screen.content(), EditorContent::Loaded { .. }),
        "expected Loaded, got error: {:?}",
        screen.validation_summary()
    );
    let t = screen.handle_key(ctrl(KeyCode::Char('s')), &ctx);
    match t {
        BotTransition::SaveNarrative { path: p, toml } => {
            assert_eq!(p, path);
            assert!(!toml.is_empty());
        }
        other => panic!("expected SaveNarrative, got {other:?}"),
    }
}

#[test]
fn ctrl_s_on_error_content_stays() {
    let ctx = BotScreenContext::mock();
    let mut screen = NarrativeEditorScreen::open("/no/such/file.toml".into());
    let t = screen.handle_key(ctrl(KeyCode::Char('s')), &ctx);
    assert!(matches!(t, BotTransition::Stay));
}

// ── IR ────────────────────────────────────────────────────────────────────────

#[test]
fn screen_name_is_narrative_editor() {
    let screen = NarrativeEditorScreen::new_file();
    assert_eq!(screen.screen_name(), "Narrative Editor");
}

#[test]
fn to_tui_node_does_not_panic_for_new_file() {
    let screen = NarrativeEditorScreen::new_file();
    let _ = screen.to_tui_node();
}

#[test]
fn to_tui_node_does_not_panic_for_error() {
    let screen = NarrativeEditorScreen::open("/no/such/file.toml".into());
    let _ = screen.to_tui_node();
}

#[test]
fn to_tui_node_does_not_panic_for_loaded_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("loaded.toml");
    fs::write(&path, "[narrative]\n").unwrap();
    let screen = NarrativeEditorScreen::open(path);
    let _ = screen.to_tui_node();
}
