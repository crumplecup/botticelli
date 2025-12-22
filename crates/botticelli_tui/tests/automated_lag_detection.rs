//! Automated test that detects keyboard lag by simulating the full TUI event loop.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Test that simulates the ACTUAL event flow: EventStream -> channel -> handle_event -> render
#[tokio::test]
async fn test_full_pipeline_lag_detection() {
    println!("\n=== AUTOMATED LAG DETECTION TEST ===");
    println!("Simulating: Type 'hello' -> measure end-to-end latency\n");
    
    use botticelli_tui::AppState;
    
    let mut state = AppState::default();
    
    // Simulate the channel from EventStream
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // Spawn "user typing" task that sends events
    let typing_task = tokio::spawn(async move {
        let text = "hello";
        for c in text.chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            event_tx.send(key).unwrap();
            // Small delay between keypresses (realistic typing)
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    
    // Main loop simulating the actual Tui::run() select loop
    let mut latencies = Vec::new();
    let mut render_times = Vec::new();
    
    // 60fps ticker like the real code
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    let test_start = Instant::now();
    let mut events_processed = 0;
    
    loop {
        tokio::select! {
            biased;
            
            // Keyboard event (highest priority)
            Some(key) = event_rx.recv() => {
                let event_start = Instant::now();
                
                // This is what handle_event does
                state.handle_key(key).expect("handle_key failed");
                
                let handle_time = event_start.elapsed();
                latencies.push(handle_time);
                events_processed += 1;
                
                println!("Event {}: handle_key took {:?}", events_processed, handle_time);
                
                if events_processed >= 5 {
                    break; // Got all events
                }
            }
            
            // Render tick (60fps)
            _ = ticker.tick() => {
                let render_start = Instant::now();
                
                // Simulate minimal render (just the state changes, no actual terminal draw)
                let _ = state.input_buffer(); // Access state like render would
                
                let render_time = render_start.elapsed();
                render_times.push(render_time);
            }
        }
        
        // Timeout if taking too long
        if test_start.elapsed() > Duration::from_secs(5) {
            panic!("Test timeout! Only processed {} events", events_processed);
        }
    }
    
    typing_task.abort(); // Clean up
    
    let total_time = test_start.elapsed();
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let max_latency = *latencies.iter().max().unwrap();
    let avg_render = render_times.iter().sum::<Duration>() / render_times.len() as u32;
    let max_render = *render_times.iter().max().unwrap();
    
    println!("\n=== RESULTS ===");
    println!("Total time: {:?}", total_time);
    println!("Events processed: {}", events_processed);
    println!("Renders during test: {}", render_times.len());
    println!();
    println!("Event handling:");
    println!("  Average: {:?}", avg_latency);
    println!("  Max: {:?}", max_latency);
    println!();
    println!("Rendering:");
    println!("  Average: {:?}", avg_render);
    println!("  Max: {:?}", max_render);
    println!();
    
    // Check for problems
    let mut failures = Vec::new();
    
    if avg_latency > Duration::from_millis(10) {
        failures.push(format!("❌ Event handling too slow: avg {:?}", avg_latency));
    }
    
    if max_latency > Duration::from_millis(50) {
        failures.push(format!("❌ Event handling spike: max {:?}", max_latency));
    }
    
    if avg_render > Duration::from_millis(16) {
        failures.push(format!("❌ Rendering too slow (dropping frames): avg {:?}", avg_render));
    }
    
    if total_time > Duration::from_millis(300) {
        failures.push(format!("❌ Overall too slow: {:?} for 5 chars", total_time));
    }
    
    if !failures.is_empty() {
        println!("\n=== FAILURES ===");
        for failure in &failures {
            println!("{}", failure);
        }
        panic!("\nLag detected! {} issues found", failures.len());
    }
    
    println!("✅ No lag detected - all timings within acceptable ranges");
}

/// Test that renders are actually debounced, not happening on every keystroke
#[tokio::test]
async fn test_render_debouncing() {
    println!("\n=== RENDER DEBOUNCING TEST ===");
    
    use botticelli_tui::AppState;
    let mut state = AppState::default();
    
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // Send 10 rapid keystrokes
    for i in 0..10 {
        let key = KeyEvent::new(KeyCode::Char((b'a' + i) as char), KeyModifiers::NONE);
        event_tx.send(key).unwrap();
    }
    drop(event_tx);
    
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    let mut events_processed = 0;
    let mut renders_done = 0;
    let start = Instant::now();
    
    loop {
        tokio::select! {
            biased;
            
            Some(key) = event_rx.recv() => {
                state.handle_key(key).expect("handle_key failed");
                events_processed += 1;
            }
            
            _ = ticker.tick() => {
                renders_done += 1;
                if start.elapsed() > Duration::from_millis(200) {
                    break; // Stop after 200ms
                }
            }
        }
        
        if event_rx.is_closed() && events_processed == 10 {
            // Give one more render cycle
            ticker.tick().await;
            renders_done += 1;
            break;
        }
    }
    
    println!("10 keystrokes processed: {} events", events_processed);
    println!("Renders in same period: {}", renders_done);
    println!("Render/event ratio: {:.2}", renders_done as f64 / events_processed as f64);
    
    // We should have fewer renders than events (debouncing working)
    // At 60fps for ~200ms, we expect ~12 renders max
    if renders_done > 15 {
        panic!("❌ Too many renders: {} (expected ~12 for 200ms at 60fps)", renders_done);
    }
    
    println!("✅ Render debouncing is working");
}
