//! Keyboard input latency test - measures responsiveness.

use std::time::{Duration, Instant};
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent};

/// Test that keyboard events can be read quickly without blocking.
/// 
/// This simulates the event loop and measures how long it takes to process
/// multiple keyboard events in sequence.
#[test]
fn test_keyboard_responsiveness() {
    // This test can only run in an interactive terminal
    // We'll test the EventHandler polling mechanism instead
    
    let tick_rate = Duration::from_millis(250);
    
    // Simulate multiple quick polls like the event loop does
    let start = Instant::now();
    let mut poll_count = 0;
    
    // Try to poll 10 times (should complete quickly if not blocking)
    for _ in 0..10 {
        // This is what EventHandler does
        if event::poll(Duration::from_millis(1)).unwrap_or(false) {
            // Event available, read it
            let _ = event::read();
        }
        poll_count += 1;
    }
    
    let elapsed = start.elapsed();
    
    // 10 polls with 1ms timeout should take < 50ms total
    assert!(
        elapsed < Duration::from_millis(50),
        "Polling took too long: {:?} for {} polls (indicates blocking)",
        elapsed,
        poll_count
    );
    
    println!("✓ Event polling test passed: {:?} for {} polls", elapsed, poll_count);
}

/// Test that the tick rate doesn't cause excessive delays.
#[test]
fn test_tick_rate_impact() {
    let tick_rates = [
        Duration::from_millis(10),
        Duration::from_millis(50),
        Duration::from_millis(100),
        Duration::from_millis(250),
    ];
    
    for tick_rate in tick_rates {
        let start = Instant::now();
        
        // Single poll - should return immediately if no events
        let has_event = event::poll(tick_rate).unwrap_or(false);
        
        let elapsed = start.elapsed();
        
        if has_event {
            println!("Event detected during test (user input?), skipping timing assertion");
            continue;
        }
        
        // If no events, should have waited the full tick_rate
        assert!(
            elapsed >= tick_rate,
            "Poll returned too early: {:?} < {:?}",
            elapsed,
            tick_rate
        );
        
        // But not much longer (allow 20ms variance)
        assert!(
            elapsed < tick_rate + Duration::from_millis(20),
            "Poll took too long: {:?} vs expected {:?}",
            elapsed,
            tick_rate
        );
        
        println!("✓ Tick rate {:?} test passed: actual {:?}", tick_rate, elapsed);
    }
}

/// Demonstrate the problem: 250ms tick rate means 250ms minimum delay per keystroke.
#[test]
fn test_demonstrate_250ms_problem() {
    // With 250ms tick rate, if user types 5 characters quickly,
    // and each poll() call blocks for up to 250ms when no event is present,
    // the UI feels laggy
    
    let tick_rate = Duration::from_millis(250);
    
    println!("\n=== Demonstrating 250ms tick rate problem ===");
    println!("Tick rate: {:?}", tick_rate);
    println!("If user types quickly, each character could wait up to {:?}", tick_rate);
    println!("Typing 'hello' (5 chars) could take up to {:?}", tick_rate * 5);
    println!("\nSolution: Use shorter tick rate (e.g., 16ms for 60fps)");
    println!("Or: Use non-blocking event reading in separate thread");
}

/// Test recommended tick rate for responsive UI.
#[test]
fn test_recommended_tick_rate() {
    // For 60fps feel, we want ~16ms frame time
    let recommended_tick_rate = Duration::from_millis(16);
    
    let start = Instant::now();
    let _ = event::poll(recommended_tick_rate);
    let elapsed = start.elapsed();
    
    println!("\n=== Recommended tick rate ===");
    println!("Tick rate: {:?} (60fps)", recommended_tick_rate);
    println!("Actual poll time: {:?}", elapsed);
    println!("This gives smooth, responsive UI");
}
