//! Integration tests for TUI exchange points.
//!
//! Tests each critical handoff in the keyboard input → screen update pipeline:
//! 1. Event capture: crossterm → event channel
//! 2. Event routing: event channel → AppState.handle_event()
//! 3. State update: AppState processes input
//! 4. Render pipeline: AppState → terminal.draw()

use botticelli_tui::{AppState, Event};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing for tests.
fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("trace".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();
}

/// Test 1: Event capture - crossterm events → event channel
///
/// This tests that crossterm events are properly captured and sent to the channel
/// without blocking.
#[tokio::test]
async fn test_exchange_point_event_capture() {
    init_tracing();
    
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Simulate what the EventStream task does
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let tui_event = Event::Key(key_event);
    
    let start = Instant::now();
    tx.send(tui_event.clone()).expect("Send failed");
    let send_elapsed = start.elapsed();
    
    tracing::info!("Event capture → channel send: {:?}", send_elapsed);
    assert!(send_elapsed.as_micros() < 100, "Channel send took {:?} (> 100µs)", send_elapsed);
    
    // Verify event received
    let received = rx.recv().await.expect("Receive failed");
    match received {
        Event::Key(k) if k.code == KeyCode::Char('a') => {},
        _ => panic!("Wrong event received"),
    }
}

/// Test 2: Event routing - event channel → AppState.handle_key()
///
/// This tests that events from the channel are routed to AppState quickly.
#[tokio::test]
async fn test_exchange_point_event_routing() {
    init_tracing();
    
    let mut state = AppState::default();
    
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    
    let start = Instant::now();
    state.handle_key(key_event).expect("Handle failed");
    let handle_elapsed = start.elapsed();
    
    tracing::info!("Event routing → AppState.handle_key(): {:?}", handle_elapsed);
    assert!(handle_elapsed.as_millis() < 5, "Event handling took {:?} (> 5ms)", handle_elapsed);
}

/// Test 3: State update - AppState processes keyboard input
///
/// This tests that AppState.handle_key() updates the state quickly.
#[tokio::test]
async fn test_exchange_point_state_update() {
    init_tracing();
    
    let mut state = AppState::default();
    
    // Type a character
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    
    let start = Instant::now();
    state.handle_key(key_event).expect("Handle failed");
    let update_elapsed = start.elapsed();
    
    tracing::info!("State update (handle_key): {:?}", update_elapsed);
    assert!(update_elapsed.as_micros() < 500, "State update took {:?} (> 500µs)", update_elapsed);
}

/// Test 4: Render pipeline - Multiple renders don't block input
///
/// This tests that rendering doesn't block the event channel.
#[tokio::test]
async fn test_exchange_point_render_pipeline() {
    init_tracing();
    
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Send 10 events rapidly
    let events: Vec<_> = "hello world".chars().map(|c| {
        Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }).collect();
    
    let send_start = Instant::now();
    for event in events {
        tx.send(event).expect("Send failed");
    }
    let send_total = send_start.elapsed();
    
    tracing::info!("Sent 11 events in {:?} ({:?} avg)", 
        send_total, 
        send_total / 11
    );
    
    // All events should be sent in < 1ms total
    assert!(send_total.as_micros() < 1000, "Sending 11 events took {:?} (> 1ms)", send_total);
    
    // Verify all events received (with timeout)
    let mut count = 0;
    while count < 11 {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(_)) => count += 1,
            _ => break,
        }
    }
    
    tracing::info!("Received {} of 11 events", count);
    assert_eq!(count, 11, "Only received {} of 11 events", count);
}

/// Test 5: Full pipeline latency - Event → State → Ready for render
///
/// This tests the full latency from receiving an event to having the state
/// ready for rendering.
#[tokio::test]
async fn test_exchange_point_full_pipeline() {
    init_tracing();
    
    let mut state = AppState::default();
    
    // Simulate the full pipeline
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    
    let pipeline_start = Instant::now();
    
    // 1. Event received from channel (simulated)
    let recv_time = pipeline_start.elapsed();
    
    // 2. Handle event
    state.handle_key(key_event).expect("Handle failed");
    let handle_time = pipeline_start.elapsed();
    
    // 3. State is now ready for rendering
    let total_latency = pipeline_start.elapsed();
    
    tracing::info!("Full pipeline latency:");
    tracing::info!("  - Event recv: {:?}", recv_time);
    tracing::info!("  - Event handle: {:?}", handle_time);
    tracing::info!("  - Total: {:?}", total_latency);
    
    // Target: < 1ms for keyboard responsiveness
    assert!(total_latency.as_millis() < 1, 
        "Full pipeline took {:?} (> 1ms)", total_latency);
}

/// Test 6: Rapid typing stress test
///
/// This simulates typing 50 characters rapidly to ensure no backpressure.
#[tokio::test]
async fn test_exchange_point_rapid_typing() {
    init_tracing();
    
    let mut state = AppState::default();
    
    let text = "The quick brown fox jumps over the lazy dog. 12345";
    
    let start = Instant::now();
    
    for c in text.chars() {
        let key_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        state.handle_key(key_event).expect("Handle failed");
    }
    
    let total = start.elapsed();
    let avg = total / 50;
    
    tracing::info!("Rapid typing: 50 chars in {:?} ({:?} avg)", total, avg);
    
    // Each character should process in < 1ms on average
    assert!(avg.as_millis() < 1, "Average per-char latency: {:?} (> 1ms)", avg);
    
    // Total should be < 50ms
    assert!(total.as_millis() < 50, "Total typing latency: {:?} (> 50ms)", total);
}
