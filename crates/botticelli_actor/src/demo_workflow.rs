use crate::{ActorError, ActorErrorKind, ActorResult};
use botticelli_interface::BotStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

/// Configuration for demo workflow.
#[derive(
    Clone,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_builder::Builder,
)]
#[setters(prefix = "with_")]
pub struct WorkflowConfig {
    /// Storage backend for validation queries.
    #[serde(skip)]
    storage: Option<Arc<dyn BotStorage>>,
    /// MCP server HTTP endpoint.
    mcp_endpoint: String,
    /// Delay between stages in milliseconds.
    stage_delay_ms: u64,
    /// Whether to run in test mode (skip actual execution).
    test_mode: bool,
}

impl std::fmt::Debug for WorkflowConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowConfig")
            .field("mcp_endpoint", &self.mcp_endpoint)
            .field("stage_delay_ms", &self.stage_delay_ms)
            .field("test_mode", &self.test_mode)
            .finish_non_exhaustive()
    }
}

/// Workflow stage with validation.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct WorkflowStage {
    /// Stage identifier.
    id: String,
    /// Stage name.
    name: String,
    /// Natural language prompt.
    prompt: String,
    /// Expected output table.
    expected_table: Option<String>,
    /// Minimum expected rows.
    min_rows: usize,
    /// Dependencies (stage IDs that must complete first).
    dependencies: Vec<String>,
}

/// Validation result for a workflow stage.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ValidationResult {
    /// Stage ID.
    stage_id: String,
    /// Whether validation passed.
    passed: bool,
    /// Number of rows found.
    rows_found: usize,
    /// Error message if validation failed.
    error: Option<String>,
}

/// Demo workflow executor for complete Botticelli pipelines.
pub struct WorkflowExecutor {
    config: WorkflowConfig,
    stages: Vec<WorkflowStage>,
    completed_stages: Vec<String>,
    validation_results: Vec<ValidationResult>,
}

impl WorkflowExecutor {
    /// Creates a new workflow executor with Botticelli promotion workflow.
    pub fn new(config: WorkflowConfig) -> Self {
        let stages = Self::create_botticelli_promotion_workflow();

        Self {
            config,
            stages,
            completed_stages: Vec::new(),
            validation_results: Vec::new(),
        }
    }

    /// Creates the complete Botticelli promotion workflow.
    fn create_botticelli_promotion_workflow() -> Vec<WorkflowStage> {
        vec![
            WorkflowStage {
                id: "strategy".to_string(),
                name: "Promotion Strategy Planning".to_string(),
                prompt: "Create a narrative for promoting Botticelli on Discord. Generate a table called 'promotion_strategy' with columns: strategy_name (text), target_audience (text), key_message (text), success_metric (text). Include at least 3 strategies.".to_string(),
                expected_table: Some("promotion_strategy".to_string()),
                min_rows: 3,
                dependencies: vec![],
            },
            WorkflowStage {
                id: "channels".to_string(),
                name: "Discord Channel Design".to_string(),
                prompt: "Create a narrative to design Discord channels for Botticelli. Generate a table called 'discord_channels' with columns: channel_name (text), channel_type (text), description (text), topic (text). Include channels for announcements, support, showcase, development, and community.".to_string(),
                expected_table: Some("discord_channels".to_string()),
                min_rows: 5,
                dependencies: vec!["strategy".to_string()],
            },
            WorkflowStage {
                id: "content".to_string(),
                name: "Content Generation".to_string(),
                prompt: "Create a narrative to generate social media posts for Botticelli. Generate a table called 'social_posts' with columns: post_title (text), post_content (text), channel_name (text), post_type (text), scheduled_time (text). Create at least 10 diverse posts.".to_string(),
                expected_table: Some("social_posts".to_string()),
                min_rows: 10,
                dependencies: vec!["channels".to_string()],
            },
            WorkflowStage {
                id: "media".to_string(),
                name: "Media Asset Planning".to_string(),
                prompt: "Create a narrative to plan media assets for Botticelli posts. Generate a table called 'media_assets' with columns: asset_name (text), asset_type (text), description (text), usage_context (text). Include logos, screenshots, diagrams, and promotional images.".to_string(),
                expected_table: Some("media_assets".to_string()),
                min_rows: 8,
                dependencies: vec!["content".to_string()],
            },
            WorkflowStage {
                id: "calendar".to_string(),
                name: "Content Calendar".to_string(),
                prompt: "Create a narrative to build a 30-day content calendar. Generate a table called 'content_calendar' with columns: date (text), post_id (text), channel (text), content_type (text), status (text). Map posts to dates.".to_string(),
                expected_table: Some("content_calendar".to_string()),
                min_rows: 20,
                dependencies: vec!["content".to_string()],
            },
        ]
    }

    /// Executes the complete workflow with validation.
    #[instrument(skip(self))]
    pub async fn execute(&mut self) -> ActorResult<WorkflowSummary> {
        info!(
            total_stages = self.stages.len(),
            "Starting workflow execution"
        );

        for stage in self.stages.clone() {
            if !self.check_dependencies(&stage) {
                error!(
                    stage = %stage.id,
                    deps = ?stage.dependencies,
                    "Stage dependencies not met"
                );
                return Err(ActorError::new(ActorErrorKind::InvalidConfiguration(
                    format!("Dependencies not met for stage: {}", stage.id),
                )));
            }

            info!(stage = %stage.id, name = %stage.name, "Executing stage");

            match self.execute_stage(&stage).await {
                Ok(()) => {
                    info!(stage = %stage.id, "Stage execution completed");
                }
                Err(e) => {
                    error!(stage = %stage.id, error = %e, "Stage execution failed");
                    self.record_validation(ValidationResult {
                        stage_id: stage.id.clone(),
                        passed: false,
                        rows_found: 0,
                        error: Some(e.to_string()),
                    });
                    return Err(e);
                }
            }

            let validation = self.validate_stage(&stage).await?;
            self.record_validation(validation.clone());

            if !validation.passed {
                error!(
                    stage = %stage.id,
                    rows_found = validation.rows_found,
                    min_rows = stage.min_rows,
                    "Stage validation failed"
                );
                return Err(ActorError::new(ActorErrorKind::PlatformPermanent(format!(
                    "Validation failed for stage: {}",
                    stage.id
                ))));
            }

            self.completed_stages.push(stage.id.clone());

            sleep(Duration::from_millis(self.config.stage_delay_ms)).await;
        }

        info!(
            completed = self.completed_stages.len(),
            "Workflow completed successfully"
        );

        Ok(self.create_summary())
    }

    /// Checks if stage dependencies are met.
    fn check_dependencies(&self, stage: &WorkflowStage) -> bool {
        stage
            .dependencies
            .iter()
            .all(|dep| self.completed_stages.contains(dep))
    }

    /// Executes a single workflow stage.
    #[instrument(skip(self, stage))]
    async fn execute_stage(&self, stage: &WorkflowStage) -> ActorResult<()> {
        debug!(prompt = %stage.prompt, "Sending prompt");

        if self.config.test_mode {
            warn!("Test mode - skipping actual execution");
            sleep(Duration::from_millis(100)).await;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let endpoint = &self.config.mcp_endpoint;

        info!(endpoint = %endpoint, "Sending prompt to MCP server");

        let response = client
            .post(format!("{}/tools/create_narrative", endpoint))
            .json(&serde_json::json!({
                "user_prompt": stage.prompt
            }))
            .send()
            .await
            .map_err(|e| {
                ActorError::new(ActorErrorKind::PlatformTemporary(format!(
                    "HTTP request failed: {}",
                    e
                )))
            })?;

        let result: serde_json::Value = response.json().await.map_err(|e| {
            ActorError::new(ActorErrorKind::PlatformTemporary(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        info!(result = ?result, "Narrative created");

        Ok(())
    }

    /// Validates stage output by checking content table row counts.
    #[instrument(skip(self, stage))]
    async fn validate_stage(&self, stage: &WorkflowStage) -> ActorResult<ValidationResult> {
        if stage.expected_table.is_none() {
            debug!("No validation required");
            return Ok(ValidationResult {
                stage_id: stage.id.clone(),
                passed: true,
                rows_found: 0,
                error: None,
            });
        }

        let table_name = stage.expected_table.as_ref().unwrap();
        info!(table = %table_name, min_rows = stage.min_rows, "Validating stage output");

        if self.config.test_mode {
            warn!("Test mode - skipping validation");
            return Ok(ValidationResult {
                stage_id: stage.id.clone(),
                passed: true,
                rows_found: stage.min_rows,
                error: None,
            });
        }

        let storage = self.config.storage.as_ref().ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Storage not configured".to_string(),
            ))
        })?;

        match storage.list_content(table_name, 10_000).await {
            Ok(records) => {
                let rows_found = records.len();
                let passed = rows_found >= stage.min_rows;

                info!(
                    table = %table_name,
                    rows_found,
                    min_rows = stage.min_rows,
                    passed,
                    "Validation complete"
                );

                Ok(ValidationResult {
                    stage_id: stage.id.clone(),
                    passed,
                    rows_found,
                    error: if passed {
                        None
                    } else {
                        Some(format!(
                            "Expected at least {} rows, found {}",
                            stage.min_rows, rows_found
                        ))
                    },
                })
            }
            Err(e) => {
                error!(table = %table_name, error = %e, "Validation query failed");
                Ok(ValidationResult {
                    stage_id: stage.id.clone(),
                    passed: false,
                    rows_found: 0,
                    error: Some(format!("Query failed: {}", e)),
                })
            }
        }
    }

    /// Records validation result.
    fn record_validation(&mut self, result: ValidationResult) {
        self.validation_results.push(result);
    }

    /// Creates workflow execution summary.
    fn create_summary(&self) -> WorkflowSummary {
        WorkflowSummary {
            total_stages: self.stages.len(),
            completed_stages: self.completed_stages.len(),
            validation_results: self.validation_results.clone(),
        }
    }
}

/// Summary of workflow execution.
#[derive(Debug, Clone)]
pub struct WorkflowSummary {
    /// Total number of stages.
    pub total_stages: usize,
    /// Number of completed stages.
    pub completed_stages: usize,
    /// Validation results for each stage.
    pub validation_results: Vec<ValidationResult>,
}

impl WorkflowSummary {
    /// Prints formatted summary.
    pub fn print(&self) {
        info!("=== Workflow Execution Summary ===");
        info!(
            "Stages: {}/{} completed",
            self.completed_stages, self.total_stages
        );

        for result in &self.validation_results {
            let status = if result.passed { "✓" } else { "✗" };
            info!(
                "{} Stage: {} - {} rows",
                status, result.stage_id, result.rows_found
            );

            if let Some(ref error) = result.error {
                warn!("  Error: {}", error);
            }
        }
    }
}
