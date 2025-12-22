//! Comprehensive TUI keyboard lag detection test.
//!
//! This test measures end-to-end keyboard input latency INCLUDING render cycle.

use botticelli_tui::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Target latency: <50ms per keystroke for acceptable UX
const MAX_ACCEPTABLE_LATENCY_MS: u128 = 50;

/// Simulates typing a string and measures latency including simulated rendering
#[tokio::test]
async fn test_keyboard_lag_end_to_end() {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();

    info!("Starting comprehensive keyboard lag test WITH rendering simulation");

    // Create app state AND terminal
    let mut state = AppState::default();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Test string to type
    let test_input = "Hello, this is a typing test!";
    let mut total_latency = Duration::ZERO;
    let mut max_latency = Duration::ZERO;
    let mut total_render_time = Duration::ZERO;
    let mut max_render_time = Duration::ZERO;

    info!("Simulating typing: '{}'", test_input);

    for (i, ch) in test_input.chars().enumerate() {
        let start = Instant::now();
        
        // Simulate key press event  
        let key_event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        
        // Process the event
        state.handle_key(key_event).expect("Handle key should succeed");
        
        let handle_time = start.elapsed();
        
        // Simulate render cycle (this is what happens in real TUI)
        let render_start = Instant::now();
        terminal
            .draw(|f| {
                use botticelli_tui::View;
                use botticelli_tui::ViewMode;
                
                // Render the current view (matches real code)
                let result = match state.mode() {
                    ViewMode::Chat => botticelli_tui::ChatView.render(f, &state),
                    ViewMode::NarrativeBrowser => botticelli_tui::NarrativeBrowserView.render(f, &state),
                    ViewMode::ConversationHistory => botticelli_tui::ConversationHistoryView.render(f, &state),
                    ViewMode::NarrativeEditor => botticelli_tui::NarrativeEditorView.render(f, &state),
                    ViewMode::Bots => botticelli_tui::BotsView.render(f, &state),
                    ViewMode::Database => botticelli_tui::DatabaseView.render(f, &state),
                    ViewMode::Schedule => botticelli_tui::ScheduleView.render(f, &state),
                    ViewMode::Settings => Ok(()), // Settings view not yet implemented
                };
                
                if let Err(e) = result {
                    warn!(error = ?e, "Render error");
                }
            })
            .expect("Render should succeed");
        let render_time = render_start.elapsed();
        
        let total_time = start.elapsed();
        
        total_latency += total_time;
        max_latency = max_latency.max(total_time);
        total_render_time += render_time;
        max_render_time = max_render_time.max(render_time);
        
        debug!(
            char = %ch,
            handle_ms = handle_time.as_millis(),
            render_ms = render_time.as_millis(),
            total_ms = total_time.as_millis(),
            position = i,
            "Keystroke processed + rendered"
        );

        // Warn if render is slow
        if render_time.as_millis() > 16 {
            warn!(
                char = %ch,
                render_ms = render_time.as_millis(),
                "⚠️  SLOW RENDER (>16ms = dropped frame)"
            );
        }

        // Fail fast if any single keystroke cycle is too slow
        if total_time.as_millis() > MAX_ACCEPTABLE_LATENCY_MS {
            panic!(
                "Keystroke '{}' at position {} took {}ms (max allowed: {}ms)\n\
                 - Handle: {}ms\n\
                 - Render: {}ms",
                ch,
                i,
                total_time.as_millis(),
                MAX_ACCEPTABLE_LATENCY_MS,
                handle_time.as_millis(),
                render_time.as_millis(),
            );
        }
    }

    let avg_latency_ms = total_latency.as_millis() / test_input.len() as u128;
    let max_latency_ms = max_latency.as_millis();
    let avg_render_ms = total_render_time.as_millis() / test_input.len() as u128;
    let max_render_ms = max_render_time.as_millis();

    info!(
        avg_latency_ms = avg_latency_ms,
        max_latency_ms = max_latency_ms,
        avg_render_ms = avg_render_ms,
        max_render_ms = max_render_ms,
        keystrokes = test_input.len(),
        "Typing test complete"
    );

    // Verify average latency is acceptable
    assert!(
        avg_latency_ms < MAX_ACCEPTABLE_LATENCY_MS,
        "Average latency {}ms exceeds maximum {}ms",
        avg_latency_ms,
        MAX_ACCEPTABLE_LATENCY_MS
    );

    // Verify max latency is acceptable
    assert!(
        max_latency_ms < MAX_ACCEPTABLE_LATENCY_MS * 2,
        "Maximum latency {}ms exceeds maximum {}ms",
        max_latency_ms,
        MAX_ACCEPTABLE_LATENCY_MS * 2
    );

    info!("✅ Keyboard lag test PASSED (including rendering)");
}

/// Tests rapid typing burst to detect buffering/queueing issues INCLUDING rendering
#[tokio::test]
async fn test_rapid_typing_burst() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();

    info!("Starting rapid typing burst test WITH rendering");

    let mut state = AppState::default();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Simulate 50 rapid keystrokes
    let burst_size = 50;
    let start = Instant::now();
    
    let mut max_per_key = Duration::ZERO;
    let mut slow_count = 0;

    for i in 0..burst_size {
        let key_start = Instant::now();
        
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        state.handle_key(key_event).expect("Handle key should succeed");
        
        // Render after each keystroke (matches real behavior)
        terminal
            .draw(|f| {
                use botticelli_tui::View;
                use botticelli_tui::ViewMode;
                
                let result = match state.mode() {
                    ViewMode::Chat => botticelli_tui::ChatView.render(f, &state),
                    ViewMode::NarrativeBrowser => botticelli_tui::NarrativeBrowserView.render(f, &state),
                    ViewMode::ConversationHistory => botticelli_tui::ConversationHistoryView.render(f, &state),
                    ViewMode::NarrativeEditor => botticelli_tui::NarrativeEditorView.render(f, &state),
                    ViewMode::Bots => botticelli_tui::BotsView.render(f, &state),
                    ViewMode::Database => botticelli_tui::DatabaseView.render(f, &state),
                    ViewMode::Schedule => botticelli_tui::ScheduleView.render(f, &state),
                    ViewMode::Settings => Ok(()), // Settings view not yet implemented
                };
                
                if let Err(e) = result {
                    warn!(error = ?e, "Render error");
                }
            })
            .expect("Render should succeed");
            
        let key_time = key_start.elapsed();
        max_per_key = max_per_key.max(key_time);
        
        if key_time.as_millis() > 16 {
            slow_count += 1;
            warn!(
                key_num = i,
                time_ms = key_time.as_millis(),
                "Slow keystroke cycle"
            );
        }
    }

    let total_time = start.elapsed();
    let avg_per_key = total_time.as_millis() / burst_size;
    let max_per_key_ms = max_per_key.as_millis();

    info!(
        total_ms = total_time.as_millis(),
        avg_per_key_ms = avg_per_key,
        max_per_key_ms = max_per_key_ms,
        slow_count = slow_count,
        keystrokes = burst_size,
        "Rapid typing burst complete"
    );

    // Each key should still be fast even in a burst
    assert!(
        avg_per_key < MAX_ACCEPTABLE_LATENCY_MS,
        "Rapid typing averaged {}ms per key (max allowed: {}ms)",
        avg_per_key,
        MAX_ACCEPTABLE_LATENCY_MS
    );
    
    // Max should be reasonable too
    assert!(
        max_per_key_ms < MAX_ACCEPTABLE_LATENCY_MS * 2,
        "Maximum keystroke time was {}ms (max allowed: {}ms)",
        max_per_key_ms,
        MAX_ACCEPTABLE_LATENCY_MS * 2
    );

    info!("✅ Rapid typing test PASSED (including rendering)");
}
