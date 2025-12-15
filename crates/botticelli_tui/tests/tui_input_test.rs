use botticelli_tui::{AppState, ViewMode};

#[test]
fn test_initial_state_is_chat_view() {
    let state = AppState::default();

    assert_eq!(state.mode(), ViewMode::Chat);
}

#[test]
fn test_input_buffer_starts_empty() {
    let state = AppState::default();

    assert_eq!(state.input_buffer(), "");
}

#[test]
fn test_append_input() {
    let mut state = AppState::default();

    state.append_input("h");
    state.append_input("e");
    state.append_input("l");
    state.append_input("l");
    state.append_input("o");

    assert_eq!(state.input_buffer(), "hello");
}

#[test]
fn test_set_input_buffer() {
    let mut state = AppState::default();

    state.set_input_buffer("test message".to_string());

    assert_eq!(state.input_buffer(), "test message");
}

#[test]
fn test_clear_input() {
    let mut state = AppState::default();

    state.set_input_buffer("test message".to_string());
    assert_eq!(state.input_buffer(), "test message");

    state.clear_input();
    assert_eq!(state.input_buffer(), "");
}

#[test]
fn test_switch_view_mode() {
    let mut state = AppState::default();

    assert_eq!(state.mode(), ViewMode::Chat);

    state.set_mode(ViewMode::NarrativeBrowser);
    assert_eq!(state.mode(), ViewMode::NarrativeBrowser);

    state.set_mode(ViewMode::NarrativeEditor);
    assert_eq!(state.mode(), ViewMode::NarrativeEditor);
}

#[test]
fn test_narrative_selection() {
    let mut state = AppState::default();

    // Start with no selection
    assert_eq!(state.selected_narrative(), None);

    // Set narrative list
    state.set_narrative_list(vec![
        "narrative1".to_string(),
        "narrative2".to_string(),
        "narrative3".to_string(),
    ]);

    // Select first
    state.set_selected_narrative(Some(0));
    assert_eq!(state.selected_narrative(), Some(0));

    // Navigate down
    state.select_next_narrative();
    assert_eq!(state.selected_narrative(), Some(1));

    // Navigate up
    state.select_previous_narrative();
    assert_eq!(state.selected_narrative(), Some(0));
}
