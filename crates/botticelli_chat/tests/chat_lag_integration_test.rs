//! Integration test for keyboard lag in the actual chat binary.
//!
//! This test spawns the real botticelli-chat binary with instrumentation
//! and analyzes trace output to detect keyboard lag.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
#[ignore] // Run with: cargo test --test chat_lag_integration_test -- --ignored --nocapture
fn test_actual_chat_keyboard_lag_with_logs() {
    // Check if MCP server is running
    let mcp_running = Command::new("curl")
        .args(&["-s", "http://localhost:8080/health"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !mcp_running {
        eprintln!("⚠️  MCP server not running on http://localhost:8080");
        eprintln!("💡 Start in another terminal:");
        eprintln!("   cargo run --bin botticelli-mcp-pmcp-http --features database,gemini,groq,streamable-http");
        panic!("MCP server required for integration test");
    }

    println!("✅ MCP server is running");
    println!("🚀 Spawning botticelli-chat with RUST_LOG=debug...\n");

    // Spawn with debug logging
    let mut child = Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "botticelli_chat",
            "--bin",
            "botticelli-chat",
            "--features",
            "cli,tui",
            "--",
            "--config",
            "chat.toml",
        ])
        .env("RUST_LOG", "botticelli_chat=debug,botticelli_tui=debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn chat binary");

    // Capture stderr logs
    let stderr = child.stderr.take().expect("Failed to get stderr");
    let log_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut all_logs = Vec::new();
        for line in reader.lines().flatten() {
            println!("📋 {}", line);
            all_logs.push(line);
        }
        all_logs
    });

    // Wait for initialization
    println!("\n⏳ Waiting 3s for initialization...");
    thread::sleep(Duration::from_secs(3));

    // Send keystrokes
    println!("⌨️  Sending keystrokes: 'test'...\n");
    if let Some(stdin) = child.stdin.as_mut() {
        for ch in "test".chars() {
            let _ = write!(stdin, "{}", ch);
            let _ = stdin.flush();
            thread::sleep(Duration::from_millis(50)); // Realistic typing speed
        }
    }

    // Wait for processing
    thread::sleep(Duration::from_secs(2));

    // Cleanup
    println!("\n🧹 Shutting down...");
    let _ = child.kill();
    let _ = child.wait();

    // Analyze logs
    let logs = log_handle.join().expect("Failed to collect logs");

    println!("\n📊 ANALYSIS:");
    println!("════════════════════════════════════════");
    println!("Total log lines: {}", logs.len());

    // Look for slow operations
    let mut slow_ops = Vec::new();
    for log in &logs {
        // Look for timing annotations
        if log.contains("elapsed=") || log.contains("duration=") || log.contains("took") {
            println!("⏱️  {}", log);

            // Check for slow operations (>50ms is noticeable lag)
            if log.contains("ms") {
                if let Some(ms_str) = extract_milliseconds(log) {
                    if ms_str > 50.0 {
                        slow_ops.push((ms_str, log.clone()));
                    }
                }
            }
        }
    }

    println!("\n🐌 SLOW OPERATIONS (>50ms):");
    for (ms, log) in &slow_ops {
        println!("  {:.1}ms: {}", ms, log);
    }

    if !slow_ops.is_empty() {
        panic!(
            "\n❌ KEYBOARD LAG DETECTED: {} operations >50ms\n",
            slow_ops.len()
        );
    }

    println!("\n✅ No keyboard lag detected");
}

fn extract_milliseconds(log: &str) -> Option<f64> {
    // Try to extract millisecond values from log lines
    // Examples: "elapsed=123ms", "took 45ms", "duration=67.8ms"
    for word in log.split_whitespace() {
        if word.ends_with("ms") {
            let num_str = word.trim_end_matches("ms").trim_end_matches(|c: char| !c.is_numeric() && c != '.');
            if let Ok(val) = num_str.parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
}
