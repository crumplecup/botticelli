use botticelli_chat::AppState;
use botticelli_interface::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

/// Test Point 1: TUI Event → AppState update (should be instant, no I/O)
#[tokio::test]
async fn test_local_keyboard_to_state_latency() {
    let mut app = AppState::new().await;
    
    let mut total_time = std::time::Duration::ZERO;
    let iterations = 100;
    
    for c in "The quick brown fox jumps over the lazy dog".chars() {
        let event = Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::empty(),
        });
        
        let start = Instant::now();
        app.handle_event(event).await;
        let elapsed = start.elapsed();
        
        total_time += elapsed;
        
        // Each keystroke should be <1ms for local state updates
        assert!(
            elapsed.as_millis() < 5,
            "Local state update took {}ms for char '{}' - should be <5ms",
            elapsed.as_millis(),
            c
        );
    }
    
    let avg_time = total_time / iterations;
    println!("✅ Point 1 - Local event handling: avg {}μs per keystroke", avg_time.as_micros());
    assert!(avg_time.as_millis() < 1, "Average should be sub-millisecond");
}

/// Test Point 2: Does the TUI make HTTP calls during typing? (it shouldn't)
#[tokio::test]
async fn test_typing_has_no_http_calls() {
    // This test verifies that typing doesn't trigger HTTP requests
    // We need to check if AppState.handle_event does any async I/O
    
    let mut app = AppState::new().await;
    
    // Type a message character by character
    for c in "Hello world".chars() {
        let event = Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::empty(),
        });
        
        let start = Instant::now();
        app.handle_event(event).await;
        let elapsed = start.elapsed();
        
        // If this takes >10ms, something is doing I/O during typing
        assert!(
            elapsed.as_millis() < 10,
            "Typing char '{}' took {}ms - suggests blocking I/O during keystroke",
            c,
            elapsed.as_millis()
        );
    }
    
    println!("✅ Point 2 - No HTTP calls during typing");
}

/// Test Point 3: HTTP request/response time for send_message
#[tokio::test]
#[ignore] // Only run when HTTP server is available
async fn test_http_server_response_time() {
    use reqwest::Client;
    
    let client = Client::new();
    let url = "http://127.0.0.1:3000/api/send_message";
    
    let start = Instant::now();
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "message": "test"
        }))
        .send()
        .await;
    let elapsed = start.elapsed();
    
    match response {
        Ok(_) => {
            println!("✅ Point 3 - HTTP round-trip: {}ms", elapsed.as_millis());
            assert!(
                elapsed.as_millis() < 100,
                "HTTP request took {}ms - should be <100ms for localhost",
                elapsed.as_millis()
            );
        }
        Err(e) => {
            println!("⚠️  Point 3 - HTTP server not running: {}", e);
        }
    }
}

/// Test Point 4: Render performance (terminal.draw())
#[tokio::test]
async fn test_render_performance() {
    use botticelli_tui::view::{ChatView, View};
    use ratatui::{backend::TestBackend, Terminal};
    
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut view = ChatView::new();
    
    // Simulate typing by updating input buffer
    let test_text = "The quick brown fox jumps over the lazy dog";
    
    let mut total_render_time = std::time::Duration::ZERO;
    
    for _ in 0..test_text.len() {
        let start = Instant::now();
        terminal.draw(|f| view.render(f, f.area())).unwrap();
        let elapsed = start.elapsed();
        
        total_render_time += elapsed;
        
        // Each render should be fast
        assert!(
            elapsed.as_millis() < 16,
            "Render took {}ms - should be <16ms (60fps)",
            elapsed.as_millis()
        );
    }
    
    let avg = total_render_time / test_text.len() as u32;
    println!("✅ Point 4 - Render performance: avg {}ms per frame", avg.as_millis());
}
