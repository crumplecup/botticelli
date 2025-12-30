//! Integration test for TUI keyboard input lag.
//!
//! This test measures end-to-end latency from key press to state update.

use botticelli_core::ToolDefinition;
use botticelli_error::ChatResult;
use botticelli_interface::{ChatHost, ChatMessage};
use botticelli_tui::{AppState, ChatView, View};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Mock ChatHost for testing
struct MockChatHost;

#[async_trait::async_trait]
impl ChatHost for MockChatHost {
    async fn send_message(&mut self, _message: String) -> ChatResult<String> {
        Ok("Mock response".to_string())
    }

    async fn get_conversation(&self) -> ChatResult<Vec<ChatMessage>> {
        Ok(vec![])
    }

    async fn available_tools(&self) -> ChatResult<Vec<ToolDefinition>> {
        Ok(vec![])
    }

    async fn execute_tool(
        &mut self,
        _name: &str,
        _arguments: serde_json::Value,
    ) -> ChatResult<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
}

#[test]
fn test_keyboard_input_lag() {
    // Initialize tracing for diagnostics
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();

    // Create TUI with TestBackend
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    // Create AppState with mock chat host
    let chat_host: Arc<tokio::sync::Mutex<dyn ChatHost>> =
        Arc::new(tokio::sync::Mutex::new(MockChatHost));
    let mut state = AppState::new(chat_host);

    // Create ChatView using default (implements View trait)
    let mut chat_view = ChatView::default();

    // Simulate rapid typing: "hello world"
    let test_string = "hello world";
    let keystrokes: Vec<KeyEvent> = test_string
        .chars()
        .map(|c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
        .collect();

    tracing::info!("Simulating {} keystrokes", keystrokes.len());

    // Track timing for each keystroke
    let mut timings = Vec::new();

    for (i, key_event) in keystrokes.iter().enumerate() {
        let start = Instant::now();

        // Process key event through the view (returns Command)
        let _command = chat_view
            .handle_input(*key_event, &state)
            .expect("handle input");

        // Force render to simulate real usage
        terminal
            .draw(|f| {
                chat_view.render(f, &state).expect("render");
            })
            .expect("draw");

        let elapsed = start.elapsed();
        timings.push(elapsed);

        tracing::debug!(
            "Keystroke {} ('{}') processed in {:?}",
            i,
            test_string.chars().nth(i).unwrap(),
            elapsed
        );
    }

    // Analyze results
    let total_time: Duration = timings.iter().sum();
    let avg_time = total_time / timings.len() as u32;
    let max_time = timings.iter().max().unwrap();
    let min_time = timings.iter().min().unwrap();

    tracing::info!("=== TIMING RESULTS ===");
    tracing::info!("Total keystrokes: {}", timings.len());
    tracing::info!("Total time: {:?}", total_time);
    tracing::info!("Average per keystroke: {:?}", avg_time);
    tracing::info!("Min: {:?}, Max: {:?}", min_time, max_time);

    // Count slow keystrokes (> 50ms is noticeable lag)
    let slow_count = timings
        .iter()
        .filter(|&&t| t > Duration::from_millis(50))
        .count();
    if slow_count > 0 {
        tracing::warn!("{} keystrokes took >50ms (noticeable lag)", slow_count);
    }

    // Assert acceptable performance
    // Average should be < 16ms (60 FPS)
    assert!(
        avg_time < Duration::from_millis(16),
        "Average keystroke latency too high: {:?} (expected <16ms)",
        avg_time
    );

    // Max should be < 50ms (noticeable lag threshold)
    assert!(
        *max_time < Duration::from_millis(50),
        "Max keystroke latency too high: {:?} (expected <50ms)",
        max_time
    );

    // No more than 10% of keystrokes should be > 32ms
    let slow_threshold = Duration::from_millis(32);
    let slow_count = timings.iter().filter(|&&t| t > slow_threshold).count();
    let slow_percentage = (slow_count as f64 / timings.len() as f64) * 100.0;

    assert!(
        slow_percentage < 10.0,
        "Too many slow keystrokes: {:.1}% > 32ms (expected <10%)",
        slow_percentage
    );
}
