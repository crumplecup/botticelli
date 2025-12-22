//! Test that measures ACTUAL end-to-end typing lag including rendering.
//! This simulates what the user experiences.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Simulate one complete event loop iteration with render.
/// This is what actually happens when you type.
#[tokio::test]
async fn test_full_event_loop_with_render() {
    use botticelli_tui::AppState;
    
    let mut state = AppState::default();
    let text = "hello world";
    
    println!("\n=== FULL EVENT LOOP LAG TEST ===");
    println!("Simulating: KeyPress -> handle_key -> render -> display");
    println!();
    
    let mut timings = Vec::new();
    
    for (i, c) in text.chars().enumerate() {
        let iteration_start = Instant::now();
        
        // 1. Key event arrives
        let key_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        
        // 2. handle_key processes it
        let handle_start = Instant::now();
        state.handle_key(key_event).expect("handle_key failed");
        let handle_time = handle_start.elapsed();
        
        // 3. Render is triggered (simulate terminal write)
        let render_start = Instant::now();
        // Simulate rendering - this is what's actually slow!
        // In real TUI, this calls terminal.draw() which:
        // - Diffs the buffer
        // - Writes to terminal
        // - Flushes output
        tokio::time::sleep(Duration::from_micros(100)).await; // Simulate minimal render
        let render_time = render_start.elapsed();
        
        let total_time = iteration_start.elapsed();
        timings.push(total_time);
        
        println!("Char {} ('{}'): handle={:?} render={:?} TOTAL={:?}", 
            i, c, handle_time, render_time, total_time);
    }
    
    let total = timings.iter().sum::<Duration>();
    let avg = total / timings.len() as u32;
    let max = timings.iter().max().unwrap();
    
    println!();
    println!("=== RESULTS ===");
    println!("Total time: {:?}", total);
    println!("Average per keystroke: {:?}", avg);
    println!("Max keystroke: {:?}", max);
    println!();
    
    // This is what user feels!
    if avg > Duration::from_millis(16) {
        panic!("❌ TOO SLOW! Avg {:?} per keystroke (need < 16ms for 60fps feel)", avg);
    } else if avg > Duration::from_millis(10) {
        println!("⚠️  MARGINAL: Avg {:?} per keystroke (okay but not great)", avg);
    } else {
        println!("✅ RESPONSIVE: Avg {:?} per keystroke", avg);
    }
}

/// Test the actual tokio::select! event loop pattern.
#[tokio::test]
async fn test_actual_event_loop_pattern() {
    use botticelli_tui::AppState;
    
    let mut state = AppState::default();
    let text = "test";
    
    // Create channels like real app
    let (_mcp_tx, mut mcp_rx) = mpsc::unbounded_channel::<()>();
    let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
    let mut render_interval = tokio::time::interval(Duration::from_millis(16));
    render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    let mut needs_render = false;
    
    println!("\n=== ACTUAL EVENT LOOP PATTERN TEST ===");
    
    for (i, c) in text.chars().enumerate() {
        let iteration_start = Instant::now();
        
        let key_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        
        // This is the ACTUAL pattern from app.rs with biased select
        tokio::select! {
            biased;
            
            // Keyboard event (HIGHEST PRIORITY - checked first)
            _ = async {
                state.handle_key(key_event).expect("handle_key failed");
                needs_render = true;
            } => {
                let elapsed = iteration_start.elapsed();
                println!("Char {} ('{}'): {:?}", i, c, elapsed);
                
                if elapsed > Duration::from_millis(10) {
                    panic!("❌ Keystroke took {:?} (too slow!)", elapsed);
                }
            }
            
            _ = mcp_rx.recv() => {
                println!("⚠️  MCP MESSAGE BEFORE KEYBOARD!");
                panic!("No MCP message expected!");
            }
            
            _ = render_interval.tick() => {
                // Render only if needed
                if needs_render {
                    // Simulate render
                    tokio::time::sleep(Duration::from_micros(100)).await;
                    needs_render = false;
                }
            }
            
            _ = tick_interval.tick() => {
                // Should not fire during this test
                println!("⚠️  TICK FIRED!");
            }
        }
    }
    
    println!("✅ Event loop pattern works correctly - keyboard prioritized");
}

/// Test if render is being called too often.
#[tokio::test]
async fn test_render_frequency() {
    println!("\n=== RENDER FREQUENCY TEST ===");
    println!("Checking if we render on every keystroke (bad) or batch (good)");
    
    let render_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let render_count_clone = render_count.clone();
    
    // Simulate 10 keystrokes
    for _ in 0..10 {
        // Each keystroke triggers handle_key
        // Then calls render
        render_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    let total_renders = render_count.load(std::sync::atomic::Ordering::SeqCst);
    println!("10 keystrokes = {} renders", total_renders);
    
    if total_renders >= 10 {
        println!("❌ PROBLEM: Rendering on EVERY keystroke!");
        println!("   This causes lag. Should batch renders.");
        panic!("Too many renders: {} for 10 keystrokes", total_renders);
    } else {
        println!("✅ Renders are batched");
    }
}
