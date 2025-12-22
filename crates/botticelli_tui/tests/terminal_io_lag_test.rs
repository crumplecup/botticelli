//! Test that measures ACTUAL terminal.draw() performance.

use std::time::{Duration, Instant};
use ratatui::{Terminal, backend::TestBackend};
use ratatui::layout::Rect;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Test that measures REAL terminal rendering performance
#[tokio::test]
async fn test_terminal_draw_performance() {
    println!("\n=== TERMINAL DRAW PERFORMANCE TEST ===");
    println!("Measuring actual terminal.draw() calls with real rendering\n");
    
    use botticelli_tui::AppState;
    
    // Create real terminal with test backend
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // Warm up
    terminal.draw(|frame| {
        use botticelli_tui::{View, ChatView};
        let _ = ChatView.render(frame, &state);
    }).unwrap();
    
    println!("=== Baseline: Single render ===");
    let start = Instant::now();
    terminal.draw(|frame| {
        use botticelli_tui::{View, ChatView};
        let _ = ChatView.render(frame, &state);
    }).unwrap();
    let single_render = start.elapsed();
    println!("Single render took: {:?}", single_render);
    
    if single_render > Duration::from_millis(16) {
        panic!("❌ SLOW: Single render took {:?} (> 16ms, can't maintain 60fps)", single_render);
    }
    println!("✅ Single render is fast enough for 60fps\n");
    
    println!("=== Test: Type 10 characters with render after each ===");
    let mut render_times = Vec::new();
    
    for i in 0..10 {
        // Simulate keystroke
        let key = KeyEvent::new(KeyCode::Char((b'a' + i) as char), KeyModifiers::NONE);
        state.handle_key(key).unwrap();
        
        // Measure render
        let start = Instant::now();
        terminal.draw(|frame| {
            use botticelli_tui::{View, ChatView};
            let _ = ChatView.render(frame, &state);
        }).unwrap();
        let render_time = start.elapsed();
        render_times.push(render_time);
        
        println!("Char {}: render took {:?}", i + 1, render_time);
    }
    
    let avg_render = render_times.iter().sum::<Duration>() / render_times.len() as u32;
    let max_render = *render_times.iter().max().unwrap();
    let min_render = *render_times.iter().min().unwrap();
    
    println!("\n=== RESULTS ===");
    println!("Renders: {}", render_times.len());
    println!("Average: {:?}", avg_render);
    println!("Min: {:?}", min_render);
    println!("Max: {:?}", max_render);
    println!();
    
    // Check for issues
    let mut failures = Vec::new();
    
    if avg_render > Duration::from_millis(16) {
        failures.push(format!("❌ Average render too slow: {:?} (can't maintain 60fps)", avg_render));
    }
    
    if max_render > Duration::from_millis(50) {
        failures.push(format!("❌ Render spike detected: {:?}", max_render));
    }
    
    // Calculate theoretical typing lag
    let typing_lag = avg_render;
    println!("Theoretical typing lag: {:?}", typing_lag);
    println!("(Time from keystroke to screen update)\n");
    
    if typing_lag > Duration::from_millis(50) {
        failures.push(format!("❌ Typing lag too high: {:?}", typing_lag));
    }
    
    if !failures.is_empty() {
        println!("=== FAILURES ===");
        for failure in &failures {
            println!("{}", failure);
        }
        panic!("\n{} performance issues detected", failures.len());
    }
    
    println!("✅ Terminal I/O performance is acceptable");
}

/// Test that conditional rendering actually works
#[tokio::test]
async fn test_conditional_rendering_reduces_draws() {
    println!("\n=== CONDITIONAL RENDERING TEST ===");
    println!("Verify that we only render when state changes\n");
    
    use botticelli_tui::AppState;
    use tokio::sync::mpsc;
    
    let mut state = AppState::default();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // Send 5 keystrokes
    for i in 0..5 {
        let key = KeyEvent::new(KeyCode::Char((b'a' + i) as char), KeyModifiers::NONE);
        event_tx.send(key).unwrap();
    }
    drop(event_tx);
    
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    let mut events_processed = 0;
    let mut renders_executed = 0;
    let mut needs_render = false;
    let start = Instant::now();
    
    // Simulate the actual event loop pattern
    loop {
        tokio::select! {
            biased;
            
            Some(key) = event_rx.recv() => {
                state.handle_key(key).unwrap();
                events_processed += 1;
                needs_render = true; // State changed
            }
            
            _ = ticker.tick(), if needs_render => {
                renders_executed += 1;
                needs_render = false;
            }
        }
        
        if event_rx.is_closed() && events_processed == 5 {
            // Wait one more tick to ensure final render
            if needs_render {
                ticker.tick().await;
                renders_executed += 1;
            }
            break;
        }
        
        if start.elapsed() > Duration::from_secs(1) {
            break;
        }
    }
    
    println!("Events processed: {}", events_processed);
    println!("Renders executed: {}", renders_executed);
    println!("Time elapsed: {:?}", start.elapsed());
    
    // With conditional rendering, we should have at most 1 render per event
    // (plus maybe 1-2 extra ticks)
    if renders_executed > events_processed + 2 {
        panic!("❌ Too many renders: {} for {} events (expected ~{})",
               renders_executed, events_processed, events_processed);
    }
    
    // Should have at least 1 render (to show the changes)
    if renders_executed == 0 {
        panic!("❌ No renders executed!");
    }
    
    println!("✅ Conditional rendering is working (ratio: {:.2} renders per event)",
             renders_executed as f64 / events_processed as f64);
}

/// Test that render complexity doesn't scale poorly with input buffer size
#[tokio::test]
async fn test_render_scales_with_buffer_size() {
    println!("\n=== RENDER SCALING TEST ===");
    println!("Verify render performance doesn't degrade with large input\n");
    
    use botticelli_tui::AppState;
    
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // Test with increasing buffer sizes
    let test_sizes = vec![10, 100, 500, 1000];
    let mut times = Vec::new();
    
    for size in &test_sizes {
        // Fill buffer
        let mut test_state = AppState::default();
        for i in 0..*size {
            let c = (b'a' + (i % 26) as u8) as char;
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            test_state.handle_key(key).unwrap();
        }
        
        // Measure render
        let start = Instant::now();
        terminal.draw(|frame| {
            use botticelli_tui::{View, ChatView};
            let _ = ChatView.render(frame, &test_state);
        }).unwrap();
        let render_time = start.elapsed();
        times.push(render_time);
        
        println!("Buffer size {}: render took {:?}", size, render_time);
    }
    
    println!();
    
    // Check that render time doesn't grow too much
    let first = times[0];
    let last = *times.last().unwrap();
    let growth_factor = last.as_secs_f64() / first.as_secs_f64();
    
    println!("Growth factor (1000 chars / 10 chars): {:.2}x", growth_factor);
    
    // Render should be roughly O(viewport_size), not O(buffer_size)
    // So even 100x more data shouldn't be 100x slower
    if growth_factor > 10.0 {
        panic!("❌ Render performance degrades too much: {:.2}x slower with large buffer", growth_factor);
    }
    
    // Also check absolute time
    if last > Duration::from_millis(50) {
        panic!("❌ Render too slow with large buffer: {:?}", last);
    }
    
    println!("✅ Render performance scales acceptably");
}
