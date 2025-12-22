//! Integration test - simulates actual typing and measures lag.

use botticelli_tui::AppState;
use std::time::{Duration, Instant};

/// Test typing characters one by one and measure cumulative lag.
#[tokio::test]
async fn test_rapid_typing_simulation() {
    let mut state = AppState::default();
    
    let text = "hello world";
    let start = Instant::now();
    
    // Simulate typing each character
    for (i, c) in text.chars().enumerate() {
        let char_start = Instant::now();
        
        // Simulate key press event -> handle_key
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::NONE,
        );
        
        state.handle_key(key_event).await.expect("handle_key failed");
        
        let char_elapsed = char_start.elapsed();
        
        // Each character should be processed in < 10ms
        assert!(
            char_elapsed < Duration::from_millis(10),
            "Character {} ('{}') took too long: {:?}",
            i, c, char_elapsed
        );
        
        println!("Char {} ('{}') processed in: {:?}", i, c, char_elapsed);
    }
    
    let total_elapsed = start.elapsed();
    
    // Verify buffer updated correctly
    assert_eq!(state.input_buffer(), text);
    
    // Total time for 11 characters should be < 100ms (< 10ms per char)
    assert!(
        total_elapsed < Duration::from_millis(100),
        "Typing '{}' took too long: {:?} (avg {:?} per char)",
        text,
        total_elapsed,
        total_elapsed / text.len() as u32
    );
    
    println!("\n✓ Typed '{}' in {:?} (avg {:?} per char)",
        text, total_elapsed, total_elapsed / text.len() as u32);
}

/// Test that backspace is instant too.
#[tokio::test]
async fn test_backspace_responsiveness() {
    let mut state = AppState::default();
    
    // Type some text first
    state.append_input("hello");
    
    let start = Instant::now();
    
    // Delete each character
    for i in 0..5 {
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        );
        
        state.handle_key(key_event).await.expect("handle_key failed");
        
        let elapsed = start.elapsed();
        println!("Backspace {} at {:?}", i, elapsed);
    }
    
    let total_elapsed = start.elapsed();
    
    // Should be empty now
    assert_eq!(state.input_buffer(), "");
    
    // 5 backspaces should complete in < 50ms
    assert!(
        total_elapsed < Duration::from_millis(50),
        "5 backspaces took too long: {:?}",
        total_elapsed
    );
    
    println!("✓ 5 backspaces in {:?}", total_elapsed);
}

/// Test real-world typing speed (60 WPM = ~5 chars/sec).
#[tokio::test]
async fn test_realistic_typing_speed() {
    let mut state = AppState::default();
    
    // 60 WPM = 5 chars per second = 200ms between chars
    // But we should handle much faster (120 WPM = 10 chars/sec = 100ms between chars)
    let sentence = "The quick brown fox jumps over the lazy dog";
    
    let start = Instant::now();
    let mut max_char_time = Duration::ZERO;
    
    for (i, c) in sentence.chars().enumerate() {
        let char_start = Instant::now();
        
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::NONE,
        );
        
        state.handle_key(key_event).await.expect("handle_key failed");
        
        let char_elapsed = char_start.elapsed();
        if char_elapsed > max_char_time {
            max_char_time = char_elapsed;
        }
        
        if char_elapsed > Duration::from_millis(5) {
            println!("⚠ Char {} ('{}') took: {:?}", i, c, char_elapsed);
        }
    }
    
    let total_elapsed = start.elapsed();
    let avg_per_char = total_elapsed / sentence.len() as u32;
    
    println!("\n✓ Typed {} chars in {:?}", sentence.len(), total_elapsed);
    println!("  Average per char: {:?}", avg_per_char);
    println!("  Max char time: {:?}", max_char_time);
    println!("  Effective WPM: {:.0}", (sentence.len() as f64 / 5.0) / (total_elapsed.as_secs_f64() / 60.0));
    
    // Should support 120+ WPM easily (< 5ms per char average)
    assert!(
        avg_per_char < Duration::from_millis(5),
        "Average char time too slow: {:?} (can't keep up with fast typing)",
        avg_per_char
    );
    
    // Verify final buffer
    assert_eq!(state.input_buffer(), sentence);
}
