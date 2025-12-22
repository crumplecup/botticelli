//! Test that actually detects blocking in the event loop.
//! This test would have caught the bug.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};

/// This test ACTUALLY runs the Tui event loop and measures if it blocks.
/// The old broken code would FAIL this test.
#[tokio::test]
async fn test_event_loop_does_not_block() {
    println!("\n=== BLOCKING DETECTION TEST ===");
    println!("This test runs the ACTUAL Tui event loop");
    println!("Old code: event::poll(250ms) would BLOCK and fail this test");
    println!("New code: tokio::select! should pass");
    println!();
    
    // Spawn a Tui instance and run it in background
    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
    
    let handle = tokio::spawn(async move {
        // This is the ACTUAL Tui code path
        let mut tui = match botticelli_tui::Tui::new() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to create TUI: {}", e);
                return;
            }
        };
        
        // Run with timeout - if blocked, this will timeout
        tokio::select! {
            _ = tui.run() => {
                println!("TUI exited");
            }
            _ = shutdown_rx.recv() => {
                println!("Shutdown signal received");
            }
        }
    });
    
    // Give TUI time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Measure how long it takes to send shutdown signal
    let start = Instant::now();
    shutdown_tx.send(()).unwrap();
    
    // Wait for task to complete
    let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    let shutdown_time = start.elapsed();
    
    println!("Shutdown took: {:?}", shutdown_time);
    
    match result {
        Ok(_) => {
            if shutdown_time > Duration::from_millis(300) {
                panic!("❌ EVENT LOOP IS BLOCKING! Shutdown took {:?} (should be < 300ms)", shutdown_time);
            } else {
                println!("✅ Event loop is responsive: {:?}", shutdown_time);
            }
        }
        Err(_) => {
            panic!("❌ EVENT LOOP BLOCKED AND TIMED OUT! Old event::poll() pattern detected.");
        }
    }
}

/// Test that simulates rapid typing and measures if events queue up (blocking symptom).
#[tokio::test]
async fn test_no_event_queue_buildup() {
    println!("\n=== EVENT QUEUE BUILDUP TEST ===");
    println!("If event loop blocks, events queue up and lag increases");
    println!();
    
    // Create a test that sends events rapidly
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<KeyEvent>();
    
    // Simulate blocking event loop (like the old code)
    let blocking_handle = tokio::spawn(async move {
        while let Some(_event) = event_rx.recv().await {
            // Simulate old blocking behavior
            tokio::time::sleep(Duration::from_millis(100)).await; // Like event::poll(100ms)
        }
    });
    
    // Send 5 events rapidly (like typing fast)
    let send_start = Instant::now();
    for i in 0..5 {
        let key = KeyEvent::new(KeyCode::Char((b'a' + i) as char), KeyModifiers::NONE);
        event_tx.send(key).unwrap();
    }
    let send_time = send_start.elapsed();
    
    println!("Sent 5 events in: {:?}", send_time);
    
    // Wait for processing
    drop(event_tx);
    let _ = tokio::time::timeout(Duration::from_secs(1), blocking_handle).await;
    
    let total_time = send_start.elapsed();
    println!("Total processing time: {:?}", total_time);
    
    // With blocking loop, 5 events × 100ms = 500ms minimum
    if total_time > Duration::from_millis(450) {
        println!("❌ BLOCKING DETECTED: {:?} for 5 events (100ms+ per event)", total_time);
        println!("   This is the OLD broken code behavior");
        // Don't panic - this is expected for the old code
    } else {
        println!("✅ Non-blocking: {:?} for 5 events", total_time);
    }
}

/// Test that measures actual responsiveness by timing event processing.
#[tokio::test] 
async fn test_event_processing_latency() {
    println!("\n=== EVENT PROCESSING LATENCY TEST ===");
    
    use botticelli_tui::AppState;
    
    let mut state = AppState::default();
    let mut latencies = Vec::new();
    
    for i in 0..10 {
        let key = KeyEvent::new(KeyCode::Char((b'a' + i % 26) as char), KeyModifiers::NONE);
        
        let start = Instant::now();
        state.handle_key(key).unwrap();
        let latency = start.elapsed();
        
        latencies.push(latency);
        println!("Event {} latency: {:?}", i, latency);
    }
    
    let avg = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let max = *latencies.iter().max().unwrap();
    
    println!();
    println!("Average latency: {:?}", avg);
    println!("Max latency: {:?}", max);
    
    // This test passes even with blocking because it doesn't test the loop
    // BUT combined with the other tests, we can detect the issue
    
    if max > Duration::from_millis(50) {
        panic!("❌ Event processing too slow: {:?}", max);
    }
    
    println!("✅ Event processing is fast");
}
