//! IR tests for [`ScheduleScreen`] — no terminal required.

use botticelli_tui::screen::BotScreen;
use botticelli_tui::{BotScreenContext, BotTransition, ScheduleScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctx() -> BotScreenContext {
    BotScreenContext::mock()
}

fn task_rows(n: usize) -> (Vec<String>, Vec<String>) {
    let rows = (0..n)
        .map(|i| format!("active | task-{i} | actor-{i} | next: 2026-06-10"))
        .collect();
    let ids = (0..n).map(|i| format!("task-{i}")).collect();
    (rows, ids)
}

// ── No-storage mode ───────────────────────────────────────────────────────────

#[test]
fn no_storage_renders_without_panic() {
    let screen = ScheduleScreen::new(vec![], vec![], false);
    let _tree = screen.to_tui_node();
}

#[test]
fn no_storage_quit() {
    let mut screen = ScheduleScreen::new(vec![], vec![], false);
    let t = screen.handle_key(key(KeyCode::Char('q')), &ctx());
    assert!(matches!(t, BotTransition::Quit));
}

// ── Empty storage ─────────────────────────────────────────────────────────────

#[test]
fn empty_tasks_renders_without_panic() {
    let screen = ScheduleScreen::new(vec![], vec![], true);
    let _tree = screen.to_tui_node();
}

#[test]
fn r_key_emits_load_schedule_data() {
    let mut screen = ScheduleScreen::new(vec![], vec![], true);
    let t = screen.handle_key(key(KeyCode::Char('r')), &ctx());
    assert!(matches!(t, BotTransition::LoadScheduleData));
}

#[test]
fn enter_on_empty_list_stays() {
    let mut screen = ScheduleScreen::new(vec![], vec![], true);
    let t = screen.handle_key(key(KeyCode::Enter), &ctx());
    assert!(matches!(t, BotTransition::Stay));
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[test]
fn j_moves_selection_down() {
    let (rows, ids) = task_rows(3);
    let mut screen = ScheduleScreen::new(rows, ids, true);
    assert_eq!(screen.selected(), 0);
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(screen.selected(), 1);
}

#[test]
fn k_does_not_go_below_zero() {
    let (rows, ids) = task_rows(3);
    let mut screen = ScheduleScreen::new(rows, ids, true);
    screen.handle_key(key(KeyCode::Char('k')), &ctx());
    assert_eq!(screen.selected(), 0);
}

#[test]
fn j_does_not_exceed_last_item() {
    let (rows, ids) = task_rows(2);
    let mut screen = ScheduleScreen::new(rows, ids, true);
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(screen.selected(), 1, "should not exceed last index");
}

#[test]
fn enter_emits_load_task_executions() {
    let (rows, ids) = task_rows(2);
    let mut screen = ScheduleScreen::new(rows, ids, true);
    screen.handle_key(key(KeyCode::Char('j')), &ctx()); // select task-1
    let t = screen.handle_key(key(KeyCode::Enter), &ctx());
    assert!(matches!(t, BotTransition::LoadTaskExecutions { ref task_id } if task_id == "task-1"),);
}

// ── Callbacks ─────────────────────────────────────────────────────────────────

#[test]
fn on_schedule_loaded_updates_rows() {
    let mut screen = ScheduleScreen::new(vec![], vec![], true);
    let (rows, ids) = task_rows(3);
    screen.on_schedule_loaded(rows, ids);
    assert_eq!(screen.selected(), 0);
    // navigate to confirm rows were loaded
    screen.handle_key(key(KeyCode::Char('j')), &ctx());
    assert_eq!(screen.selected(), 1);
}

#[test]
fn on_task_executions_loaded_stores_rows() {
    let mut screen = ScheduleScreen::new(vec![], vec![], true);
    screen.on_task_executions_loaded(vec!["exec-1".to_string(), "exec-2".to_string()]);
    assert_eq!(screen.exec_rows().len(), 2);
}

#[test]
fn on_schedule_loaded_resets_exec_rows() {
    let mut screen = ScheduleScreen::new(vec![], vec![], true);
    screen.on_task_executions_loaded(vec!["old".to_string()]);
    let (rows, ids) = task_rows(1);
    screen.on_schedule_loaded(rows, ids);
    assert!(
        screen.exec_rows().is_empty(),
        "refresh clears old exec rows"
    );
}

#[test]
fn screen_name_is_schedule() {
    let screen = ScheduleScreen::new(vec![], vec![], true);
    assert_eq!(screen.screen_name(), "Schedule");
}
