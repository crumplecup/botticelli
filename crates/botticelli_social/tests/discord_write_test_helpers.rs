//! Test helpers for Discord write operations
//!
//! Provides utilities for setting up, executing, and tearing down Discord write operation tests.

use std::path::PathBuf;
use std::process::Command;

/// Result type for test operations
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper for running narrative-based write operation tests
pub struct WriteOperationTest {
    setup_narrative: PathBuf,
    test_narrative: PathBuf,
    teardown_narrative: Option<PathBuf>,
}

impl WriteOperationTest {
    /// Create a new write operation test
    pub fn new(
        setup_narrative: impl Into<PathBuf>,
        test_narrative: impl Into<PathBuf>,
    ) -> Self {
        Self {
            setup_narrative: setup_narrative.into(),
            test_narrative: test_narrative.into(),
            teardown_narrative: None,
        }
    }

    /// Add a teardown narrative
    pub fn with_teardown(mut self, teardown_narrative: impl Into<PathBuf>) -> Self {
        self.teardown_narrative = Some(teardown_narrative.into());
        self
    }

    /// Run the complete test cycle
    pub fn run(&self) -> TestResult {
        // Run setup
        self.run_narrative(&self.setup_narrative, "Setup")?;

        // Run test
        let test_result = self.run_narrative(&self.test_narrative, "Test");

        // Run teardown if specified
        if let Some(teardown) = &self.teardown_narrative {
            if let Err(e) = self.run_narrative(teardown, "Teardown") {
                eprintln!("Warning: Teardown failed: {}", e);
            }
        }

        test_result
    }

    /// Run a single narrative using just command
    fn run_narrative(&self, path: &PathBuf, stage: &str) -> TestResult {
        println!("\n=== Running {} narrative: {} ===", stage, path.display());
        
        // Extract just the narrative name from the path
        let narrative_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid narrative path")?;
        
        let output = Command::new("just")
            .arg("narrate")
            .arg(narrative_name)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "{} narrative failed:\nStdout: {}\nStderr: {}",
                stage, stdout, stderr
            )
            .into());
        }

        Ok(())
    }
}

/// Get the path to a test narrative
pub fn narrative_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("narratives")
        .join("discord")
        .join(format!("{}.toml", name))
}
