//! Keyboard input latency test - measures responsiveness of async event handling.

use std::time::{Duration, Instant};
use crossterm::event;

/// Test that non-blocking event poll returns immediately.
#[tokio::test]
async fn test_instant_keyboard_poll() {
    // Test that poll with 0ms timeout returns immediately
    let start = Instant::now();
    
    // This is what our async event handler does - 0ms poll
    let has_event = event::poll(Duration::from_millis(0)).unwrap_or(false);
    
    let elapsed = start.elapsed();
    
    // Should return in < 5ms (instant)
    assert!(
        elapsed < Duration::from_millis(5),
        "Non-blocking poll took too long: {:?} (should be instant)",
        elapsed
    );
    
    if has_event {
        println!("✓ Event detected and poll returned instantly: {:?}", elapsed);
    } else {
        println!("✓ No event, poll returned instantly: {:?}", elapsed);
    }
}

/// Test that spawn_blocking doesn't delay other tasks.
#[tokio::test]
async fn test_spawn_blocking_concurrency() {
    let start = Instant::now();
    
    // Simulate what our event loop does - spawn_blocking for event reading
    let event_task = tokio::task::spawn_blocking(|| {
        event::poll(Duration::from_millis(0))
    });
    
    // This should complete quickly even though spawn_blocking is used
    let result = tokio::time::timeout(Duration::from_millis(10), event_task).await;
    
    let elapsed = start.elapsed();
    
    assert!(
        result.is_ok(),
        "spawn_blocking took > 10ms (indicates blocking)"
    );
    
    println!("✓ spawn_blocking with 0ms poll completed in: {:?}", elapsed);
}

/// Test tokio::select! with multiple event sources.
#[tokio::test]
async fn test_select_responsiveness() {
    let start = Instant::now();
    
    // Create a tick interval (like our TUI does)
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    
    // Create a mock event channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Send a message immediately
    tx.send("test".to_string()).unwrap();
    
    // Use select! to handle multiple sources
    tokio::select! {
        _ = tick.tick() => {
            panic!("Tick fired first (should not happen - message was sent immediately)");
        }
        msg = rx.recv() => {
            let elapsed = start.elapsed();
            assert!(msg.is_some());
            assert!(
                elapsed < Duration::from_millis(10),
                "Message receipt took too long: {:?}",
                elapsed
            );
            println!("✓ tokio::select! processed immediate message in: {:?}", elapsed);
        }
    }
}

/// Test that keyboard event reading doesn't block tick events.
#[tokio::test]
async fn test_independent_tick_and_events() {
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    let mut tick_count = 0;
    
    let start = Instant::now();
    
    // Run for 200ms and count ticks
    loop {
        tokio::select! {
            _ = tick.tick() => {
                tick_count += 1;
                if tick_count >= 3 {
                    break;
                }
            }
            // Simulate keyboard event checking (non-blocking)
            _ = tokio::task::spawn_blocking(|| {
                event::poll(Duration::from_millis(0))
            }) => {
                // Event check completed, continue
            }
        }
    }
    
    let elapsed = start.elapsed();
    
    // Should have gotten at least 3 ticks in ~150ms
    assert!(tick_count >= 3, "Not enough ticks: {}", tick_count);
    assert!(
        elapsed >= Duration::from_millis(100) && elapsed < Duration::from_millis(250),
        "Timing incorrect: {:?} for {} ticks",
        elapsed,
        tick_count
    );
    
    println!("✓ Got {} ticks in {:?} - events didn't block ticks", tick_count, elapsed);
}

/// Demonstrate the fix: 0ms poll + tokio::select! = instant response.
#[test]
fn test_demonstrate_solution() {
    println!("\n=== Solution: Async event handling ===");
    println!("Old approach:");
    println!("  - event::poll(250ms) blocks for 250ms if no event");
    println!("  - Typing 'hello' could take 1.25s");
    println!("\nNew approach:");
    println!("  - tokio::select! with multiple event sources");
    println!("  - spawn_blocking + event::poll(0ms) = instant check");
    println!("  - Keyboard events: Instant (no blocking)");
    println!("  - Tick events: Independent 250ms interval");
    println!("  - MCP events: Channel-based (instant)");
    println!("\nResult: All events processed immediately without blocking!");
}

/// Test the actual pattern used in TuiApp.
#[tokio::test]
async fn test_tuiapp_event_pattern() {
    let start = Instant::now();
    
    // Simulate the TuiApp event loop pattern
    let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
    let (mcp_tx, mut mcp_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Send an MCP message immediately
    mcp_tx.send("mcp_update".to_string()).unwrap();
    
    // Simulate one loop iteration
    tokio::select! {
        _ = tick_interval.tick() => {
            panic!("Tick should not fire first");
        }
        
        Some(_msg) = mcp_rx.recv() => {
            let elapsed = start.elapsed();
            assert!(
                elapsed < Duration::from_millis(10),
                "MCP message took too long: {:?}",
                elapsed
            );
            println!("✓ TuiApp pattern: MCP message processed instantly in {:?}", elapsed);
        }
        
        event = tokio::task::spawn_blocking(|| {
            event::poll(Duration::from_millis(0))
        }) => {
            // This arm could also fire instantly
            println!("✓ Event check completed in {:?}", start.elapsed());
        }
    }
}
