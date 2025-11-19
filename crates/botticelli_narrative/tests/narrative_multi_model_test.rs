#![cfg(feature = "gemini")]

// Integration test: Multi-model narrative execution.
//
// This test validates that narratives can execute with different models per act,
// which is critical for cost optimization and feature selection.

use botticelli_models::GeminiClient;
use botticelli_narrative::{Narrative, NarrativeExecutor};
use std::path::Path;

/// Test that narratives can use different models for different acts.
///
/// Uses the model_options.toml narrative which demonstrates multiple Gemini models.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_narrative_multi_model_execution() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");
    let executor = NarrativeExecutor::new(client);

    let narrative_path = Path::new("narratives/model_options.toml");
    let narrative = Narrative::from_file(narrative_path)
        .expect("Failed to load model_options.toml narrative");

    let execution = executor
        .execute(&narrative)
        .await
        .expect("Narrative execution failed");

    // Should have executed the enabled acts (currently 4: flash_20, flash_lite_20, flash_25, flash_lite_25)
    assert!(
        execution.act_executions.len() >= 3,
        "Should have executed at least 3 acts, got {}",
        execution.act_executions.len()
    );

    // Verify all acts produced responses
    for (idx, act) in execution.act_executions.iter().enumerate() {
        assert!(
            !act.response.is_empty(),
            "Act {} ({}) should have produced a response",
            idx,
            act.act_name
        );

        // Verify model was specified
        assert!(
            act.model.is_some(),
            "Act {} ({}) should have a model specified",
            idx,
            act.act_name
        );
    }

    // Log execution summary
    tracing::info!(
        "Multi-model narrative executed {} acts successfully",
        execution.act_executions.len()
    );
}
