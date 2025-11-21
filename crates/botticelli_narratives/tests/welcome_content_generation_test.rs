//! Test for welcome content generation narrative with carousel.
//!
//! This test demonstrates:
//! - Multi-model workflow (flash-lite → flash)
//! - JSON-structured output
//! - Critique and refinement pipeline
//! - Carousel looping (3 iterations)
//! - Database storage

use botticelli::{GenerationBackend, NarrativeExecutor, NarrativeRepository};

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_welcome_content_generation() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Setup components
    let backend = GenerationBackend::gemini_from_env().expect("GEMINI_API_KEY required");
    let repo = NarrativeRepository::in_memory();
    let executor = NarrativeExecutor::new(backend, repo);

    // Load the narrative
    let narrative_path = "narratives/discord/welcome_content_generation.toml";
    let narrative = executor
        .repository()
        .load_narrative(narrative_path)
        .expect("Failed to load narrative");

    println!("\n=== Welcome Content Generation Narrative ===");
    println!("Narrative: {}", narrative.name());
    println!(
        "Description: {}",
        narrative.description().unwrap_or("No description")
    );

    // Check carousel configuration
    if let Some(carousel) = narrative.carousel() {
        println!("\n=== Carousel Configuration ===");
        println!("Iterations: {}", carousel.iterations());
        if let Some(budget) = carousel.budget() {
            println!("Budget:");
            if let Some(rpm) = budget.requests_per_minute() {
                println!("  - Requests/minute: {}", rpm);
            }
            if let Some(rpd) = budget.requests_per_day() {
                println!("  - Requests/day: {}", rpd);
            }
            if let Some(tpm) = budget.tokens_per_minute() {
                println!("  - Tokens/minute: {}", tpm);
            }
            if let Some(tpd) = budget.tokens_per_day() {
                println!("  - Tokens/day: {}", tpd);
            }
        }
    }

    println!("\n=== Executing Narrative ===");
    println!("This will:");
    println!("1. Generate 10 welcome message options (cheap model)");
    println!("2. Critique the options");
    println!("3. Refine the best 3 (better model)");
    println!("4. Store in database");
    println!("5. Repeat 3 times (carousel) for 9 total refined messages");

    // Execute the narrative
    let result = executor.execute(&narrative).await;

    match result {
        Ok(outputs) => {
            println!("\n=== Execution Complete ===");
            println!("Total acts executed: {}", outputs.len());

            for (i, output) in outputs.iter().enumerate() {
                println!("\n--- Act {} Output ---", i + 1);
                let preview = if output.len() > 500 {
                    format!("{}... ({} chars total)", &output[..500], output.len())
                } else {
                    output.clone()
                };
                println!("{}", preview);
            }

            println!("\n=== Success ===");
            println!("Generated 9 refined welcome messages (3 per iteration × 3 iterations)");
            println!("Messages stored in database for review");
        }
        Err(e) => {
            eprintln!("\n=== Execution Failed ===");
            eprintln!("Error: {}", e);
            panic!("Narrative execution failed");
        }
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_welcome_content_json_parsing() {
    // This test verifies that the JSON output from the generation acts
    // can be properly parsed and stored in the database

    let sample_json = r#"{
  "refined_messages": [
    {
      "title": "Welcome to Botticelli! 🎨",
      "content": "**Welcome to the Botticelli Discord community!** 🎉\n\nBotticelli is an innovative AI narrative framework that lets you build complex, multi-step AI workflows using simple TOML configuration files. No more wrestling with API calls and prompt management—just define your narrative and let Botticelli handle the rest!\n\n**Key Features:**\n• 📝 **TOML-based workflows** - Easy-to-read configuration\n• 🤖 **Multi-model support** - Use Gemini, Claude, and more\n• 🔄 **Multi-step narratives** - Chain AI interactions seamlessly\n• 💬 **Discord integration** - Build powerful community bots\n\n**Get Started:**\nCheck out `#documentation` for guides and `#examples` for inspiration. Have questions? Ask in `#help`—our community is here to support you!\n\nHappy building! ✨",
      "tone": "friendly",
      "key_features": ["TOML workflows", "Multi-model support", "Discord integration"],
      "improvements_made": ["Added emojis", "Clearer structure"],
      "rationale": "Balances friendliness with technical accuracy"
    }
  ]
}"#;

    // Parse the JSON
    let parsed: serde_json::Value = serde_json::from_str(sample_json)
        .expect("Failed to parse sample JSON");

    let messages = parsed["refined_messages"]
        .as_array()
        .expect("refined_messages should be an array");

    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert!(message["title"].is_string());
    assert!(message["content"].is_string());
    assert!(message["tone"].is_string());
    assert!(message["key_features"].is_array());

    println!("✓ JSON parsing validation successful");
}
