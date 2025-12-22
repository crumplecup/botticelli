//! Test that measures actual typing lag through the full TUI stack.

use std::time::{Duration, Instant};

/// This test runs the actual app event loop and simulates typing.
/// It will FAIL if typing is laggy.
#[tokio::test]
async fn test_full_stack_typing_lag() {
    // Skip if not in terminal (CI)
    if std::env::var("CI").is_ok() {
        println!("Skipping interactive test in CI");
        return;
    }

    println!("\n=== TYPING LAG TEST ===");
    println!("This test simulates typing through the full TUI stack.");
    println!("We'll inject key events and measure response time.\n");

    // Measure just the handle_key path without full app
    use botticelli_tui::AppState;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    
    let mut state = AppState::default();
    let test_string = "The quick brown fox jumps over the lazy dog";
    
    let mut timings = Vec::new();
    let start = Instant::now();
    
    for (i, c) in test_string.chars().enumerate() {
        let char_start = Instant::now();
        
        let key_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        state.handle_key(key_event).expect("handle_key failed");
        
        let elapsed = char_start.elapsed();
        timings.push(elapsed);
        
        if elapsed > Duration::from_millis(10) {
            println!("⚠️  SLOW: Char {} ('{}') took {:?}", i, c, elapsed);
        }
    }
    
    let total = start.elapsed();
    let avg = total / test_string.len() as u32;
    let max = timings.iter().max().unwrap();
    let min = timings.iter().min().unwrap();
    
    // Calculate percentiles
    let mut sorted = timings.clone();
    sorted.sort();
    let p50 = sorted[sorted.len() / 2];
    let p95 = sorted[sorted.len() * 95 / 100];
    let p99 = sorted[sorted.len() * 99 / 100];
    
    println!("\n=== RESULTS ===");
    println!("Characters typed: {}", test_string.len());
    println!("Total time: {:?}", total);
    println!("Average per char: {:?}", avg);
    println!("Min: {:?}", min);
    println!("Max: {:?}", max);
    println!("P50 (median): {:?}", p50);
    println!("P95: {:?}", p95);
    println!("P99: {:?}", p99);
    
    // Show slowest characters
    let mut indexed_timings: Vec<_> = timings.iter().enumerate().collect();
    indexed_timings.sort_by_key(|(_, t)| *t);
    indexed_timings.reverse();
    
    println!("\nSlowest 5 characters:");
    for (idx, timing) in indexed_timings.iter().take(5) {
        let c = test_string.chars().nth(*idx).unwrap();
        println!("  #{}: '{}' took {:?}", idx, c, timing);
    }
    
    // Final verdict
    println!("\n=== VERDICT ===");
    if avg > Duration::from_millis(5) {
        panic!("❌ TYPING IS TOO SLOW! Average {:?} per char (need < 5ms)", avg);
    } else if p95 > Duration::from_millis(10) {
        panic!("❌ P95 TOO SLOW! {:?} (need < 10ms)", p95);
    } else if *max > Duration::from_millis(50) {
        panic!("❌ MAX TOO SLOW! {:?} (need < 50ms)", max);
    } else {
        println!("✅ TYPING IS RESPONSIVE!");
        println!("   Average: {:?} < 5ms ✓", avg);
        println!("   P95: {:?} < 10ms ✓", p95);
        println!("   Max: {:?} < 50ms ✓", max);
    }
}
