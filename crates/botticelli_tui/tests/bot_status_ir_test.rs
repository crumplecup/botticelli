//! IR and transition tests for [`BotStatusScreen`].
//!
//! These tests run without a real terminal — they inspect the [`TuiNode`] tree
//! returned by `to_tui_node()` and the [`BotTransition`] returned by
//! `handle_key()`.

use botticelli_tui::{
    BotKind, BotScreen, BotScreenContext, BotStatusScreen, BotTransition, RunState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use elicit_ratatui::{TuiNode, WidgetJson};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn collect_texts(node: &TuiNode) -> Vec<String> {
    let mut out = Vec::new();
    match node {
        TuiNode::Widget { widget } => {
            if let WidgetJson::Paragraph { text, block, .. } = widget.as_ref() {
                out.push(text.to_plain_string());
                if let Some(b) = block {
                    if let Some(t) = &b.title {
                        out.push(t.clone());
                    }
                }
            }
        }
        TuiNode::Layout { children, .. } => {
            for child in children {
                out.extend(collect_texts(child));
            }
        }
        TuiNode::StatusBar { .. } => {}
    }
    out
}

// ── IR tests ─────────────────────────────────────────────────────────────────

#[test]
fn bots_screen_title_is_bots() {
    let screen = BotStatusScreen::new();
    let node = screen.to_tui_node();
    let texts = collect_texts(&node);
    assert!(
        texts.iter().any(|t| t == "Bots"),
        "expected 'Bots' block title, got: {texts:?}"
    );
}

#[test]
fn bots_screen_shows_all_three_bot_names() {
    let screen = BotStatusScreen::new();
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("generation"),
        "missing generation in: {combined}"
    );
    assert!(
        combined.contains("curation"),
        "missing curation in: {combined}"
    );
    assert!(
        combined.contains("posting"),
        "missing posting in: {combined}"
    );
}

#[test]
fn bots_screen_default_all_stopped() {
    let screen = BotStatusScreen::new();
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("○ Stopped"),
        "expected stopped badges, got: {combined}"
    );
}

#[test]
fn bots_screen_shows_running_after_state_change() {
    let mut screen = BotStatusScreen::new();
    screen.on_bot_state_changed(BotKind::Generation, true);
    assert_eq!(screen.state(0), RunState::Running);
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("● Running"),
        "expected running badge, got: {combined}"
    );
}

#[test]
fn bots_screen_stop_clears_running() {
    let mut screen = BotStatusScreen::new();
    screen.on_bot_state_changed(BotKind::Curation, true);
    screen.on_bot_state_changed(BotKind::Curation, false);
    assert_eq!(screen.state(1), RunState::Stopped);
}

#[test]
fn bots_screen_selection_cursor_visible() {
    let screen = BotStatusScreen::new();
    assert_eq!(screen.selected(), 0);
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains('▶'),
        "expected selection cursor '▶': {combined}"
    );
}

// ── Transition tests ──────────────────────────────────────────────────────────

#[test]
fn j_moves_selection_down() {
    let mut screen = BotStatusScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('j')), &ctx);
    assert_eq!(screen.selected(), 1);
}

#[test]
fn k_moves_selection_up_wrapping() {
    let mut screen = BotStatusScreen::new();
    let ctx = BotScreenContext::mock();
    // At top → wraps to last bot (index 2)
    screen.handle_key(make_key(KeyCode::Char('k')), &ctx);
    assert_eq!(screen.selected(), 2);
}

#[test]
fn j_wraps_around_at_bottom() {
    let mut screen = BotStatusScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('j')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('j')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('j')), &ctx);
    assert_eq!(screen.selected(), 0);
}

#[test]
fn q_returns_quit() {
    let mut screen = BotStatusScreen::new();
    let ctx = BotScreenContext::mock();
    let t = screen.handle_key(make_key(KeyCode::Char('q')), &ctx);
    assert!(matches!(t, BotTransition::Quit));
}

#[test]
fn screen_name_is_bots() {
    let screen = BotStatusScreen::new();
    assert_eq!(screen.screen_name(), "Bots");
}
