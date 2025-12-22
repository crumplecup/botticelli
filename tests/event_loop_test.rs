//! Test the clean event loop architecture.

use botticelli_tui::event_loop::{BackgroundWorker, EventHandler, UiCommand, UiUpdate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[tokio::test]
async fn test_keyboard_input_immediate() {
    // Setup channels
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (update_tx, mut update_rx) = mpsc::unbounded_channel();

    // Start background worker
    let worker = BackgroundWorker::new(command_rx, update_tx);
    tokio::spawn(worker.run());

    // Send character inputs
    for c in "hello".chars() {
        command_tx.send(UiCommand::CharInput(c)).unwrap();
    }

    // Collect updates with timeout
    let mut result = String::new();
    for _ in 0..5 {
        match timeout(Duration::from_millis(100), update_rx.recv()).await {
            Ok(Some(UiUpdate::TextUpdated(text))) => {
                result = text;
            }
            _ => break,
        }
    }

    assert_eq!(result, "hello");
}

#[tokio::test]
async fn test_event_handler_ctrl_c() {
    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let handler = EventHandler::new(command_tx);

    // Send Ctrl+C
    let event = crossterm::event::Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ));

    handler.handle_event(event).unwrap();

    // Should receive quit command
    match timeout(Duration::from_millis(100), command_rx.recv()).await {
        Ok(Some(UiCommand::Quit)) => {}
        _ => panic!("Expected Quit command"),
    }
}

#[tokio::test]
async fn test_backspace() {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (update_tx, mut update_rx) = mpsc::unbounded_channel();

    let worker = BackgroundWorker::new(command_rx, update_tx);
    tokio::spawn(worker.run());

    // Type "hi" then backspace
    command_tx.send(UiCommand::CharInput('h')).unwrap();
    command_tx.send(UiCommand::CharInput('i')).unwrap();
    command_tx
        .send(UiCommand::ControlKey(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::empty(),
        )))
        .unwrap();

    // Collect final state
    let mut result = String::new();
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    while let Ok(update) = update_rx.try_recv() {
        if let UiUpdate::TextUpdated(text) = update {
            result = text;
        }
    }

    assert_eq!(result, "h");
}
