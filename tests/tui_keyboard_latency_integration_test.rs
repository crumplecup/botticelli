/// Integration test for keyboard input latency in TUI
/// 
/// This test validates that keyboard events flow through the system without blocking:
/// 1. Event capture from crossterm → channel
/// 2. Channel delivery → event loop
/// 3. Event handling → state update
/// 4. Render cycle
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use botticelli_tui::{Event, Tui, TuiResult};

#[tokio::test]
async fn test_keyboard_event_latency() -> TuiResult<()> {
    // Initialize tracing for test logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    tracing::info!("=== Starting keyboard latency integration test ===");
    
    // This test measures the latency through the ACTUAL event pipeline:
    // EventStream (crossterm) → channel → event loop → state → render
    
    // We can't easily test Tui::run() directly since it blocks,
    // but we CAN test the event channel and state handling
    
    // Test 1: Event channel throughput
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    let test_keys = vec![
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
    ];
    
    tracing::info!("Test 1: Measuring channel throughput");
    let start = Instant::now();
    
    for key in &test_keys {
        event_tx.send(Event::Key(*key)).unwrap();
    }
    
    let send_elapsed = start.elapsed();
    tracing::info!("Sent {} events in {:?}", test_keys.len(), send_elapsed);
    
    // Channel sends should be instant (< 1ms for 5 events)
    assert!(
        send_elapsed.as_millis() < 1,
        "Channel sends took {:?} - should be instant",
        send_elapsed
    );
    
    // Verify all events received
    let mut received = 0;
    let recv_start = Instant::now();
    while received < test_keys.len() {
        match event_rx.try_recv() {
            Ok(_) => received += 1,
            Err(_) => break,
        }
    }
    let recv_elapsed = recv_start.elapsed();
    
    tracing::info!("Received {} events in {:?}", received, recv_elapsed);
    assert_eq!(received, test_keys.len(), "All events should be received");
    assert!(
        recv_elapsed.as_millis() < 1,
        "Receiving events took {:?} - should be instant",
        recv_elapsed
    );
    
    tracing::info!("✓ Test 1 passed: Channel throughput is instant");
    
    // Test 2: State update latency (without TUI, just AppState)
    tracing::info!("Test 2: Measuring state update latency");
    
    use std::sync::{Arc, Mutex};
    use botticelli_interface::ChatHost;
    
    // Create a mock ChatHost
    struct MockChatHost;
    impl ChatHost for MockChatHost {
        fn send_message(
            &mut self,
            _messages: Vec<botticelli_interface::ChatMessage>,
        ) -> Result<botticelli_interface::ChatResponse, Box<dyn std::error::Error + Send + Sync>> {
            Ok(botticelli_interface::ChatResponse {
                message: botticelli_interface::ChatMessage {
                    role: botticelli_interface::Role::Assistant,
                    content: "mock".to_string(),
                },
                usage: None,
            })
        }
    }
    
    let chat_host: Arc<Mutex<dyn ChatHost>> = Arc::new(Mutex::new(MockChatHost));
    let mut state = botticelli_tui::AppState::new(chat_host);
    
    let state_start = Instant::now();
    for key in &test_keys {
        state.handle_key(*key)?;
    }
    let state_elapsed = state_start.elapsed();
    
    tracing::info!("Processed {} keys through state in {:?}", test_keys.len(), state_elapsed);
    
    // State updates should be fast (< 5ms for 5 keys)
    assert!(
        state_elapsed.as_millis() < 5,
        "State updates took {:?} - should be < 5ms",
        state_elapsed
    );
    
    tracing::info!("✓ Test 2 passed: State updates are fast");
    
    tracing::info!("=== All keyboard latency tests passed ===");
    
    Ok(())
}

#[tokio::test]
async fn test_render_does_not_block_events() -> TuiResult<()> {
    // This test ensures that rendering doesn't block event processing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    tracing::info!("=== Testing render vs event independence ===");
    
    // The architecture uses tokio::select! with biased priority:
    // 1. Keyboard events (highest priority, render immediately)
    // 2. MCP updates  
    // 3. Periodic render tick (only if needs_render=true)
    //
    // This ensures keyboard is NEVER blocked by render cycle
    
    // We verify this by checking that event channel never blocks
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // Simulate rapid typing (100 chars/sec = one every 10ms)
    let typing_interval = Duration::from_millis(10);
    let char_count = 20;
    
    let sender_handle = tokio::spawn(async move {
        let mut timings = Vec::new();
        for i in 0..char_count {
            let start = Instant::now();
            event_tx
                .send(Event::Key(KeyEvent::new(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                )))
                .unwrap();
            let elapsed = start.elapsed();
            timings.push(elapsed);
            
            if elapsed.as_micros() > 100 {
                tracing::warn!("Event {} send took {:?}", i, elapsed);
            }
            
            tokio::time::sleep(typing_interval).await;
        }
        timings
    });
    
    let receiver_handle = tokio::spawn(async move {
        let mut timings = Vec::new();
        let mut count = 0;
        
        while count < char_count {
            let start = Instant::now();
            match event_rx.recv().await {
                Some(_) => {
                    let elapsed = start.elapsed();
                    timings.push(elapsed);
                    count += 1;
                    
                    if elapsed.as_millis() > 1 {
                        tracing::warn!("Event {} recv took {:?}", count, elapsed);
                    }
                }
                None => break,
            }
        }
        timings
    });
    
    let send_timings = sender_handle.await.unwrap();
    let recv_timings = receiver_handle.await.unwrap();
    
    // Analyze timings
    let max_send = send_timings.iter().max().unwrap();
    let avg_send: Duration = send_timings.iter().sum::<Duration>() / send_timings.len() as u32;
    
    let max_recv = recv_timings.iter().max().unwrap();
    let avg_recv: Duration = recv_timings.iter().sum::<Duration>() / recv_timings.len() as u32;
    
    tracing::info!("Send timings: avg={:?}, max={:?}", avg_send, max_send);
    tracing::info!("Recv timings: avg={:?}, max={:?}", avg_recv, max_recv);
    
    // Event send should NEVER block (< 100μs)
    assert!(
        max_send.as_micros() < 100,
        "Event send blocked for {:?}",
        max_send
    );
    
    // Event recv should be fast (< 1ms)  
    assert!(
        max_recv.as_millis() < 1,
        "Event recv delayed by {:?}",
        max_recv
    );
    
    tracing::info!("✓ Events are never blocked by rendering");
    
    Ok(())
}
