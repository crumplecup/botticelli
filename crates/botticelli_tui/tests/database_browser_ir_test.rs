//! IR tests for [`DatabaseBrowserScreen`] — no terminal required.

use botticelli_tui::screen::BotScreen;
use botticelli_tui::{BotScreenContext, BotTransition, DatabaseBrowserScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use elicit_ratatui::TuiNode;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctx() -> BotScreenContext {
    BotScreenContext::mock()
}

fn count_widgets(node: &TuiNode) -> usize {
    match node {
        TuiNode::Widget { .. } | TuiNode::StatusBar { .. } => 1,
        TuiNode::Layout { children, .. } => children.iter().map(count_widgets).sum(),
    }
}

// ── NoStorage mode ────────────────────────────────────────────────────────────

#[test]
fn no_storage_renders_message() {
    let screen = DatabaseBrowserScreen::new(false);
    let tree = screen.to_tui_node();
    assert!(
        count_widgets(&tree) >= 3,
        "should have header, message, help"
    );
}

#[test]
fn no_storage_navigate_to_bots() {
    let mut screen = DatabaseBrowserScreen::new(false);
    let t = screen.handle_key(key(KeyCode::Char('1')), &ctx());
    assert!(matches!(t, BotTransition::GoToBots));
}

#[test]
fn no_storage_quit() {
    let mut screen = DatabaseBrowserScreen::new(false);
    let t = screen.handle_key(key(KeyCode::Char('q')), &ctx());
    assert!(matches!(t, BotTransition::Quit));
}

// ── TableList mode ────────────────────────────────────────────────────────────

#[test]
fn table_list_renders_five_tables() {
    let screen = DatabaseBrowserScreen::new(true);
    assert!(!screen.is_content_view(), "starts in table list mode");
    let tree = screen.to_tui_node();
    // header + 5 table rows + fill + help bar = at least 8 widgets
    assert!(count_widgets(&tree) >= 8);
}

#[test]
fn navigate_down_wraps() {
    let mut screen = DatabaseBrowserScreen::new(true);
    // 5 tables — pressing 'k' from 0 should wrap to 4
    let t = screen.handle_key(key(KeyCode::Char('k')), &ctx());
    assert!(matches!(t, BotTransition::Stay));
    assert_eq!(screen.selected(), 4);
}

#[test]
fn navigate_down_increments() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(screen.selected(), 1);
}

#[test]
fn enter_emits_load_transition() {
    let mut screen = DatabaseBrowserScreen::new(true);
    let t = screen.handle_key(key(KeyCode::Enter), &ctx());
    assert!(
        matches!(t, BotTransition::LoadDatabaseTable { ref table } if table == "narrative_executions"),
        "first table should be narrative_executions"
    );
}

#[test]
fn enter_second_table_emits_actor_states() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    let t = screen.handle_key(key(KeyCode::Enter), &ctx());
    assert!(matches!(t, BotTransition::LoadDatabaseTable { ref table } if table == "actor_states"),);
}

#[test]
fn navigate_to_schedule_screen() {
    let mut screen = DatabaseBrowserScreen::new(true);
    let t = screen.handle_key(key(KeyCode::Char('5')), &ctx());
    assert!(matches!(t, BotTransition::GoToSchedule));
}

// ── ContentView mode ──────────────────────────────────────────────────────────

#[test]
fn on_table_loaded_switches_to_content_view() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.on_table_loaded("content", vec!["row1".to_string(), "row2".to_string()]);
    assert!(screen.is_content_view());
}

#[test]
fn content_view_renders_rows() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.on_table_loaded("content", (0..5).map(|i| format!("row-{i}")).collect());
    let tree = screen.to_tui_node();
    // header + 5 rows + fill + status = at least 8 widgets
    assert!(count_widgets(&tree) >= 8);
}

#[test]
fn esc_returns_to_table_list() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.on_table_loaded("content", vec!["r".to_string()]);
    assert!(screen.is_content_view());
    let t = screen.handle_key(key(KeyCode::Esc), &ctx());
    assert!(matches!(t, BotTransition::Stay));
    assert!(!screen.is_content_view());
}

#[test]
fn empty_table_renders_without_panic() {
    let mut screen = DatabaseBrowserScreen::new(true);
    screen.on_table_loaded("content", vec![]);
    let tree = screen.to_tui_node();
    assert!(count_widgets(&tree) >= 2);
}

#[test]
fn screen_name_is_database() {
    let screen = DatabaseBrowserScreen::new(true);
    assert_eq!(screen.screen_name(), "Database");
}
