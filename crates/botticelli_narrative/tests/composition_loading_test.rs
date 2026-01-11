//! Tests for narrative composition loading and context preservation.
//!
//! These tests verify that NarrativeSource correctly detects composition acts
//! and preserves MultiNarrative context when needed.

mod helpers;

use botticelli_interface::NarrativeProvider;
use botticelli_narrative::NarrativeSource;

#[test]
fn test_single_narrative_file_loads_as_single() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing single narrative file loads as Single variant");

    // Single-narrative TOML file should always load as Single variant
    let toml = r#"
[narrative]
name = "simple_test"
description = "A simple test narrative"

[toc]
order = ["greet"]

[acts.greet]
model = "gemini-2.0-flash-exp"
temperature = 0.7
max_tokens = 100

[[acts.greet.input]]
type = "text"
content = "Say hello"
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("simple.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote test TOML file");

    let source = NarrativeSource::from_file(&file_path, None)?;

    assert!(!source.has_composition_context());
    tracing::debug!("Verified no composition context");

    match &source {
        NarrativeSource::Single(narrative) => {
            assert_eq!(narrative.name(), "simple_test");
            tracing::debug!(name = narrative.name(), "Loaded as Single variant");
        }
        NarrativeSource::MultiWithContext { .. } => {
            panic!("Expected Single, got MultiWithContext");
        }
    }

    tracing::info!("Single narrative file loads as Single variant test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_without_composition_extracts_single() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing multi-narrative without composition extracts as Single");

    // Multi-narrative file where the specified narrative has no composition
    // should extract as Single (no context overhead)
    let toml = r#"
[narratives.first]
description = "First narrative without composition"
toc = ["act1"]

[narratives.second]
description = "Second narrative"
toc = ["act2"]

[acts.act1]
model = "gemini-2.0-flash-exp"
temperature = 0.7
max_tokens = 100

[[acts.act1.input]]
type = "text"
content = "Do something"

[acts.act2]
model = "gemini-2.0-flash-exp"

[[acts.act2.input]]
type = "text"
content = "Do something else"
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("multi_no_composition.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote multi-narrative test file");

    let source = NarrativeSource::from_file(&file_path, Some("first"))?;

    assert!(!source.has_composition_context());
    tracing::debug!("Verified no composition context");

    match &source {
        NarrativeSource::Single(narrative) => {
            assert_eq!(narrative.name(), "first");
            tracing::debug!(name = narrative.name(), "Extracted as Single variant");
        }
        NarrativeSource::MultiWithContext { .. } => {
            panic!("Expected Single, got MultiWithContext");
        }
    }

    tracing::info!("Multi-narrative without composition extracts as Single test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_with_composition_preserves_context() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing multi-narrative with composition preserves context");

    // Multi-narrative file where the specified narrative uses composition
    // should preserve full MultiWithContext
    let toml = r#"
[acts.call_worker]
narrative_ref = "worker"

[acts.work]
model = "gemini-2.0-flash-exp"
temperature = 0.5
max_tokens = 50

[[acts.work.input]]
type = "text"
content = "Do the work"

[narrative.orchestrator]
name = "orchestrator"
description = "Main narrative that composes others"
toc = ["call_worker"]

[narrative.worker]
name = "worker"
description = "Worker narrative"
toc = ["work"]
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("composition.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote composition test file");

    let source = NarrativeSource::from_file(&file_path, Some("orchestrator"))?;

    assert!(source.has_composition_context());
    tracing::debug!("Verified composition context exists");

    match &source {
        NarrativeSource::MultiWithContext {
            multi,
            execute_name,
        } => {
            assert_eq!(execute_name, "orchestrator");
            tracing::debug!(execute_name, "Loaded as MultiWithContext");

            // Verify both narratives are accessible
            assert!(multi.get_narrative("orchestrator").is_some());
            assert!(multi.get_narrative("worker").is_some());
            tracing::debug!("Verified both orchestrator and worker narratives accessible");
        }
        NarrativeSource::Single(_) => {
            panic!("Expected MultiWithContext, got Single");
        }
    }

    tracing::info!("Multi-narrative with composition preserves context test passed");
    Ok(())
}

#[test]
fn test_composition_with_multiple_references() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing composition with multiple narrative references");

    // Narrative with multiple composition acts should preserve context
    let toml = r#"
[acts.step1]
narrative_ref = "sub1"

[acts.step2]
narrative_ref = "sub2"

[acts.step3]
narrative_ref = "sub3"

[acts.work]
model = "gemini-2.0-flash-exp"

[[acts.work.input]]
type = "text"
content = "Sub1 work"

[narrative.main]
name = "main"
description = "Main with multiple composition acts"
toc = ["step1", "step2", "step3"]

[narrative.sub1]
name = "sub1"
toc = ["work"]

[narrative.sub2]
name = "sub2"
toc = ["work"]

[narrative.sub3]
name = "sub3"
toc = ["work"]
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("multi_ref.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote multi-reference test file");

    let source = NarrativeSource::from_file(&file_path, Some("main"))?;

    assert!(source.has_composition_context());
    tracing::debug!("Verified composition context with multiple references");

    if let NarrativeSource::MultiWithContext { multi, .. } = source {
        // All referenced narratives should be accessible
        assert!(multi.get_narrative("main").is_some());
        assert!(multi.get_narrative("sub1").is_some());
        assert!(multi.get_narrative("sub2").is_some());
        assert!(multi.get_narrative("sub3").is_some());
        tracing::debug!("Verified all 4 narratives accessible (main + 3 subs)");
    } else {
        panic!("Expected MultiWithContext");
    }

    tracing::info!("Composition with multiple references test passed");
    Ok(())
}

#[test]
fn test_mixed_acts_with_composition() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing mixed regular and composition acts");

    // Narrative with both regular acts and composition acts should preserve context
    let toml = r#"
[acts.regular1]
model = "gemini-2.0-flash-exp"

[[acts.regular1.input]]
type = "text"
content = "Regular act 1"

[acts.composed]
narrative_ref = "helper"

[acts.regular2]
model = "gemini-2.0-flash-exp"

[[acts.regular2.input]]
type = "text"
content = "Regular act 2"

[acts.help]
model = "gemini-2.0-flash-exp"

[[acts.help.input]]
type = "text"
content = "Helper work"

[narrative.mixed]
name = "mixed"
description = "Has both regular and composition acts"
toc = ["regular1", "composed", "regular2"]

[narrative.helper]
name = "helper"
toc = ["help"]
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("mixed.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote mixed acts test file");

    let source = NarrativeSource::from_file(&file_path, Some("mixed"))?;

    // Should preserve context because of the composed act
    assert!(source.has_composition_context());
    tracing::debug!("Verified context preserved for mixed acts");

    tracing::info!("Mixed regular and composition acts test passed");
    Ok(())
}

#[test]
fn test_narrative_source_get_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing NarrativeSource get_narrative() method");

    let toml = r#"
[narrative.test]
name = "test_narrative"
description = "Test"
toc = ["act"]

[acts.act]
model = "gemini-2.0-flash-exp"
[[acts.act.input]]
type = "text"
content = "Test"
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("test.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote test file");

    let source = NarrativeSource::from_file(&file_path, Some("test"))?;

    let narrative = source.get_narrative()?;
    assert_eq!(narrative.name(), "test_narrative");
    tracing::debug!(name = narrative.name(), "Retrieved narrative");

    tracing::info!("NarrativeSource get_narrative() test passed");
    Ok(())
}

#[test]
fn test_narrative_source_name() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing NarrativeSource name() method");

    let toml = r#"
[narrative]
name = "my_narrative"
description = "Test"

[toc]
order = ["act"]

[acts.act]
model = "gemini-2.0-flash-exp"
[[acts.act.input]]
type = "text"
content = "Test"
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("name_test.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote name test file");

    let source = NarrativeSource::from_file(&file_path, None)?;

    assert_eq!(source.name(), "my_narrative");
    tracing::debug!(name = source.name(), "Retrieved source name");

    tracing::info!("NarrativeSource name() test passed");
    Ok(())
}

#[test]
fn test_multi_narrative_requires_name() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing multi-narrative requires name parameter");

    let toml = r#"
[acts.act]
model = "gemini-2.0-flash-exp"

[[acts.act.input]]
type = "text"
content = "Test"

[narrative.first]
name = "first"
toc = ["act"]

[narrative.second]
name = "second"
toc = ["act"]
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("multi_no_name.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote multi-narrative file");

    // Should fail without narrative_name
    let result = NarrativeSource::from_file(&file_path, None);
    assert!(result.is_err());
    tracing::debug!("Verified error when name not provided for multi-narrative");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Multiple narratives found"),
        "Error message: {}",
        err_msg
    );
    tracing::debug!("Verified error message contains 'Multiple narratives found'");

    tracing::info!("Multi-narrative requires name test passed");
    Ok(())
}

#[test]
fn test_get_multi_context_returns_none_for_single() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing get_multi_context() returns None for Single variant");

    let toml = r#"
[narrative]
name = "single"
description = "Single narrative"

[toc]
order = ["act"]

[acts.act]
model = "gemini-2.0-flash-exp"
[[acts.act.input]]
type = "text"
content = "Test"
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("single.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote single narrative file");

    let source = NarrativeSource::from_file(&file_path, None)?;

    assert!(source.get_multi_context().is_none());
    tracing::debug!("Verified get_multi_context() returns None for Single");

    tracing::info!("get_multi_context() returns None for Single test passed");
    Ok(())
}

#[test]
fn test_get_multi_context_returns_some_for_composition() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing get_multi_context() returns Some for composition");

    let toml = r#"
[acts.comp]
narrative_ref = "sub"

[acts.work]
model = "gemini-2.0-flash-exp"

[[acts.work.input]]
type = "text"
content = "Work"

[narrative.main]
name = "main"
toc = ["comp"]

[narrative.sub]
name = "sub"
toc = ["work"]
"#;

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("comp.toml");
    std::fs::write(&file_path, toml)?;
    tracing::debug!(path = ?file_path, "Wrote composition file");

    let source = NarrativeSource::from_file(&file_path, Some("main"))?;

    assert!(source.get_multi_context().is_some());
    tracing::debug!("Verified get_multi_context() returns Some for composition");

    tracing::info!("get_multi_context() returns Some for composition test passed");
    Ok(())
}
