//! Partial narrative state during elicitation.

use botticelli_core::Input;
use botticelli_interface::RegistryOperations;
use botticelli_narrative::CarouselConfig;
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::tools::NarrativeHelper;
use botticelli_error::{McpError, McpResult};

/// Partial act definition.
#[derive(Debug, Clone, Serialize, Deserialize, derive_new::new)]
pub struct PartialAct {
    /// Act prompt/instruction.
    pub prompt: String,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional temperature override.
    pub temperature: Option<f64>,
    /// Act inputs.
    #[serde(default)]
    pub inputs: Vec<Input>,
    /// Optional carousel configuration.
    #[serde(default)]
    pub carousel: Option<CarouselConfig>,
}

/// Narrative under construction.
///
/// Uses builder pattern for type-safe construction.
/// All fields are optional during elicitation.
#[derive(Debug, Clone, Builder, Serialize, Deserialize, Getters, Default)]
#[builder(setter(into), default)]
pub struct PartialNarrative {
    /// Narrative name.
    pub(crate) name: Option<String>,

    /// Narrative description.
    pub(crate) description: Option<String>,

    /// Default model for all acts.
    pub(crate) model: Option<String>,

    /// Default temperature.
    pub(crate) temperature: Option<f64>,

    /// Default max tokens.
    pub(crate) max_tokens: Option<u32>,

    /// Act execution order.
    pub(crate) act_order: Vec<String>,

    /// Act definitions (name -> PartialAct).
    pub(crate) acts: HashMap<String, PartialAct>,

    /// Optional narrative-level carousel configuration.
    pub(crate) carousel: Option<CarouselConfig>,

    /// Generated TOML content (cached).
    toml_content: Option<String>,
}

impl PartialNarrative {
    /// Create a new empty partial narrative.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if minimum required fields are present.
    ///
    /// Minimum: name, description, at least one act.
    #[tracing::instrument(skip(self))]
    pub fn has_minimum_required(&self) -> bool {
        self.name.is_some() && self.description.is_some() && !self.acts.is_empty()
    }

    /// Validate the partial narrative.
    ///
    /// Converts to TOML and validates using existing validator.
    #[tracing::instrument(skip(self))]
    pub fn validate(&self) -> McpResult<botticelli_narrative::validator::ValidationResult> {
        use botticelli_narrative::validator::Validator;
        let toml = self.to_toml()?;
        Ok(Validator::validate_toml(&toml))
    }

    /// Convert to TOML string.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing.
    #[tracing::instrument(skip(self))]
    pub fn to_toml(&self) -> McpResult<String> {
        let name = self
            .name
            .as_ref()
            .ok_or_else(|| McpError::invalid_input("Missing narrative name".to_string()))?;

        let description = self
            .description
            .as_ref()
            .ok_or_else(|| McpError::invalid_input("Missing narrative description".to_string()))?;

        let mut toml = String::new();

        // [narrative] section
        toml.push_str("[narrative]\n");
        toml.push_str(&format!("name = \"{}\"\n", name));
        toml.push_str(&format!(
            "description = \"{}\"\n",
            NarrativeHelper::escape_toml_string(description)
        ));

        if let Some(ref model) = self.model {
            toml.push_str(&format!("model = \"{}\"\n", model));
        }

        if let Some(temp) = self.temperature {
            toml.push_str(&format!("temperature = {}\n", temp));
        }

        if let Some(max) = self.max_tokens {
            toml.push_str(&format!("max_tokens = {}\n", max));
        }

        toml.push('\n');

        // [toc] section
        toml.push_str("[toc]\n");
        toml.push_str("order = [");
        for (i, act_name) in self.act_order.iter().enumerate() {
            if i > 0 {
                toml.push_str(", ");
            }
            toml.push_str(&format!("\"{}\"", act_name));
        }
        toml.push_str("]\n\n");

        // [acts] section
        toml.push_str("[acts]\n");
        for act_name in &self.act_order {
            if let Some(act) = self.acts.get(act_name) {
                toml.push_str(&format!(
                    "{} = \"{}\"\n",
                    act_name,
                    NarrativeHelper::escape_toml_string(&act.prompt)
                ));
            }
        }

        Ok(toml)
    }

    /// Attempt to convert to a complete Narrative.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    #[tracing::instrument(skip(self))]
    pub fn try_into_narrative(&self) -> McpResult<botticelli_narrative::Narrative> {
        let toml = self.to_toml()?;
        let validation = self.validate()?;

        if !validation.is_valid() {
            return Err(McpError::invalid_input(format!(
                "Narrative validation failed: {} errors",
                validation.errors().len()
            )));
        }

        botticelli_narrative::Narrative::from_toml_str(&toml, self.name.as_deref())
            .map_err(|e| McpError::execution_failed(e.to_string()))
    }
}

impl RegistryOperations for PartialNarrative {
    type Key = String;
    type Error = McpError;

    fn registry_key(&self) -> Self::Key {
        self.name.clone().unwrap_or_else(|| "unnamed".to_string())
    }

    fn from_json_args(args: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(args).map_err(|e| {
            McpError::invalid_input(format!("Failed to deserialize PartialNarrative: {}", e))
        })
    }

    fn to_json(&self) -> Result<Value, Self::Error> {
        serde_json::to_value(self).map_err(|e| {
            McpError::execution_failed(format!("Failed to serialize PartialNarrative: {}", e))
        })
    }

    fn update_from_json(&mut self, args: Value) -> Result<(), Self::Error> {
        let obj = args
            .as_object()
            .ok_or_else(|| McpError::invalid_input("Expected JSON object".to_string()))?;

        // Update fields present in JSON
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            self.name = Some(name.to_string());
        }
        if let Some(desc) = obj.get("description").and_then(|v| v.as_str()) {
            self.description = Some(desc.to_string());
        }
        if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
            self.model = Some(model.to_string());
        }
        if let Some(temp) = obj.get("temperature").and_then(|v| v.as_f64()) {
            self.temperature = Some(temp);
        }
        if let Some(max) = obj.get("max_tokens").and_then(|v| v.as_u64()) {
            self.max_tokens = Some(max as u32);
        }

        Ok(())
    }
}
