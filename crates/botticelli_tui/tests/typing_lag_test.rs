//! Integration test - simulates actual typing and measures lag.

use botticelli_tui::AppState;
use std::time::{Duration, Instant};

/// Test typing characters one by one and measure cumulative lag.
#[tokio::test]
async fn test_rapid_typing_simulation() {
    let mut state = AppState::default();
    
    let text = "hello world";
    let start = Instant::now();
    let mut slowest_char = Duration::ZERO;
    let mut slowest_idx = 0;
    
    // Simulate typing each character
    for (i, c) in text.chars().enumerate() {
        let char_start = Instant::now();
        
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::NONE,
        );
        
        state.handle_key(key_event).expect("handle_key failed");
        
        let char_elapsed = char_start.elapsed();
        if char_elapsed > slowest_char {
            slowest_char = char_elapsed;
            slowest_idx = i;
        }
        
        println!("Char {} ('{}') processed in: {:?}", i, c, char_elapsed);
    }
    
    let total_elapsed = start.elapsed();
    let avg_per_char = total_elapsed / text.len() as u32;
    
    println!("\n✓ Typed '{}' in {:?}", text, total_elapsed);
    println!("  Average: {:?} per char", avg_per_char);
    println!("  Slowest: char {} at {:?}", slowest_idx, slowest_char);
    
    // Verify buffer
    assert_eq!(state.input_buffer(), text);
    
    // If average > 5ms, we have a problem
    if avg_per_char > Duration::from_millis(5) {
        panic!("Typing is TOO SLOW! Avg {:?} per char (need < 5ms)", avg_per_char);
    }
}
