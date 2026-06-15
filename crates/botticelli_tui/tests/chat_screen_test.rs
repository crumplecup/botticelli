//! IR and transition tests for [`ChatScreen`].
//!
//! All tests run without a real terminal or LLM — they inspect the
//! [`TuiNode`] tree from `to_tui_node()` and the [`BotTransition`] from
//! `handle_key()`.

use botticelli_tui::{
    BotScreen, BotScreenContext, BotTransition, ChatMessage, ChatRole, ChatScreen, ModelStatus,
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
                if let Some(b) = block
                    && let Some(t) = &b.title
                {
                    out.push(t.clone());
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
fn chat_screen_title_is_chat() {
    let screen = ChatScreen::new();
    let node = screen.to_tui_node();
    let texts = collect_texts(&node);
    assert!(
        texts.iter().any(|t| t == "Chat"),
        "expected 'Chat' block title, got: {texts:?}"
    );
}

#[test]
fn chat_screen_empty_shows_no_messages() {
    let screen = ChatScreen::new();
    assert_eq!(screen.messages().len(), 0);
}

#[test]
fn chat_screen_renders_user_message() {
    let mut screen = ChatScreen::new();
    screen.on_chat_response("hello".to_string()); // prime with assistant msg
    // Directly add a user message by simulating the flow
    let ctx = BotScreenContext::mock();
    for c in "hello world".chars() {
        screen.handle_key(make_key(KeyCode::Char(c)), &ctx);
    }
    screen.handle_key(make_key(KeyCode::Enter), &ctx);

    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("You: hello world"),
        "expected user message in IR: {combined}"
    );
}

#[test]
fn chat_screen_renders_assistant_message() {
    let mut screen = ChatScreen::new();
    screen.on_chat_response("I am the bot.".to_string());

    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("Bot: I am the bot."),
        "expected assistant message in IR: {combined}"
    );
}

#[test]
fn chat_screen_input_appears_in_ir() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('h')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('i')), &ctx);

    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("hi"),
        "expected input buffer 'hi' in IR: {combined}"
    );
}

// ── Transition tests ──────────────────────────────────────────────────────────

#[test]
fn chars_append_to_input_buffer() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('a')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('b')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('c')), &ctx);
    assert_eq!(screen.input(), "abc");
}

#[test]
fn backspace_removes_last_char() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('a')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('b')), &ctx);
    screen.handle_key(make_key(KeyCode::Backspace), &ctx);
    assert_eq!(screen.input(), "a");
}

#[test]
fn enter_with_text_emits_chat_send() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('h')), &ctx);
    screen.handle_key(make_key(KeyCode::Char('i')), &ctx);
    let t = screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(
        matches!(t, BotTransition::ChatSend { ref content } if content == "hi"),
        "expected ChatSend {{ content: \"hi\" }}, got {t:?}"
    );
}

#[test]
fn enter_clears_input_buffer() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('x')), &ctx);
    screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert_eq!(screen.input(), "");
}

#[test]
fn enter_with_empty_input_stays() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    let t = screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(matches!(t, BotTransition::Stay));
}

#[test]
fn waiting_blocks_new_sends() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('a')), &ctx);
    screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(*screen.waiting());
    let t = screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(matches!(t, BotTransition::Stay));
}

#[test]
fn esc_navigates_away_while_waiting() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('a')), &ctx);
    screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(*screen.waiting());
    let t = screen.handle_key(make_key(KeyCode::Esc), &ctx);
    assert!(matches!(t, BotTransition::GoToBots));
}

#[test]
fn on_chat_response_clears_waiting_and_adds_message() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    screen.handle_key(make_key(KeyCode::Char('q')), &ctx);
    screen.handle_key(make_key(KeyCode::Enter), &ctx);
    assert!(*screen.waiting());

    screen.on_chat_response("pong".to_string());
    assert!(!*screen.waiting());
    assert_eq!(
        screen.messages().last(),
        Some(&ChatMessage {
            role: ChatRole::Assistant,
            content: "pong".to_string(),
        })
    );
}

// ── Model status indicator tests ──────────────────────────────────────────────

#[test]
fn new_screen_starts_loading() {
    let screen = ChatScreen::new();
    assert_eq!(screen.model_status(), &ModelStatus::Loading);
}

#[test]
fn on_model_ready_clears_loading_indicator() {
    let mut screen = ChatScreen::new();
    screen.on_model_ready();
    assert_eq!(screen.model_status(), &ModelStatus::Ready);
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        !combined.contains("Loading"),
        "loading text should be gone after ready: {combined}"
    );
}

#[test]
fn loading_indicator_visible_while_loading() {
    let screen = ChatScreen::new();
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("Loading"),
        "expected loading indicator in IR: {combined}"
    );
}

#[test]
fn on_chat_replying_shows_replying_indicator() {
    let mut screen = ChatScreen::new();
    screen.on_model_ready();
    screen.on_chat_replying();
    assert_eq!(screen.model_status(), &ModelStatus::Replying);
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("Replying"),
        "expected replying indicator in IR: {combined}"
    );
}

#[test]
fn on_chat_response_clears_replying_indicator() {
    let mut screen = ChatScreen::new();
    screen.on_model_ready();
    screen.on_chat_replying();
    screen.on_chat_response("done".to_string());
    assert_eq!(screen.model_status(), &ModelStatus::Ready);
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        !combined.contains("Replying"),
        "replying indicator should clear after response: {combined}"
    );
}

#[test]
fn on_model_failed_shows_error_message() {
    let mut screen = ChatScreen::new();
    screen.on_model_failed("disk full".to_string());
    assert_eq!(
        screen.model_status(),
        &ModelStatus::Failed("disk full".to_string())
    );
    let node = screen.to_tui_node();
    let combined = collect_texts(&node).join(" ");
    assert!(
        combined.contains("disk full"),
        "expected error detail in IR: {combined}"
    );
}

#[test]
fn esc_returns_go_to_bots() {
    let mut screen = ChatScreen::new();
    let ctx = BotScreenContext::mock();
    let t = screen.handle_key(make_key(KeyCode::Esc), &ctx);
    assert!(matches!(t, BotTransition::GoToBots));
}

#[test]
fn screen_name_is_chat() {
    let screen = ChatScreen::new();
    assert_eq!(screen.screen_name(), "Chat");
}
