//! Partial narrative state during elicitation.

use botticelli_core::Input;
use botticelli_error::{McpError, McpResult};
use botticelli_interface::RegistryOperations;
use botticelli_narrative::CarouselConfig;
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, instrument, trace, warn};

use crate::tools::NarrativeHelper;

/// Partial act definition.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    derive_new::new,
    derive_getters::Getters,
    derive_setters::Setters,
    elicitation::Elicit,
)]
#[setters(prefix = "set_", borrow_self)]
pub struct PartialAct {
    /// Act prompt/instruction.
    prompt: String,
    /// Optional model override.
    model: Option<String>,
    /// Optional temperature override.
    temperature: Option<f64>,
    /// Act inputs.
    #[serde(default)]
    inputs: Vec<Input>,
    /// Optional carousel configuration.
    #[serde(default)]
    carousel: Option<CarouselConfig>,
}

/// Narrative under construction.
///
/// Uses builder pattern for type-safe construction.
/// All fields are optional during elicitation.
#[derive(
    Debug,
    Clone,
    Builder,
    Serialize,
    Deserialize,
    Getters,
    Default,
    derive_setters::Setters,
    elicitation::Elicit,
)]
#[builder(setter(into), default)]
#[setters(prefix = "with_", borrow_self)]
pub struct PartialNarrative {
    /// Narrative name.
    name: Option<String>,

    /// Narrative description.
    description: Option<String>,

    /// Default model for all acts.
    model: Option<String>,

    /// Default temperature.
    temperature: Option<f64>,

    /// Default max tokens.
    max_tokens: Option<u32>,

    /// Act execution order.
    act_order: Vec<String>,

    /// Act definitions (name -> PartialAct).
    acts: HashMap<String, PartialAct>,

    /// Optional narrative-level carousel configuration.
    carousel: Option<CarouselConfig>,

    /// Generated TOML content (cached).
    #[setters(skip)]
    toml_content: Option<String>,
}

impl PartialNarrative {
    /// Create a new empty partial narrative.
    #[instrument]
    pub fn new() -> Self {
        debug!("Creating new empty partial narrative");
        Self::default()
    }

    /// Get mutable access to acts HashMap.
    ///
    /// Use this to add, remove, or modify acts.
    #[instrument(skip(self))]
    pub fn acts_mut(&mut self) -> &mut HashMap<String, PartialAct> {
        trace!(
            acts_count = self.acts.len(),
            "Providing mutable access to acts"
        );
        &mut self.acts
    }

    /// Get mutable access to act order Vec.
    ///
    /// Use this to reorder acts.
    #[instrument(skip(self))]
    pub fn act_order_mut(&mut self) -> &mut Vec<String> {
        trace!(
            order_length = self.act_order.len(),
            "Providing mutable access to act order"
        );
        &mut self.act_order
    }

    /// Check if minimum required fields are present.
    ///
    /// Minimum: name, description, at least one act.
    #[instrument(skip(self), fields(
        has_name = self.name().is_some(),
        has_description = self.description().is_some(),
        acts_count = self.acts().len()
    ))]
    pub fn has_minimum_required(&self) -> bool {
        let result =
            self.name().is_some() && self.description().is_some() && !self.acts().is_empty();
        debug!(meets_minimum = result, "Checked minimum requirements");
        result
    }

    /// Validate the partial narrative.
    ///
    /// Converts to TOML and validates using existing validator.
    #[instrument(skip(self), fields(
        name = ?self.name(),
        acts_count = self.acts().len()
    ))]
    pub fn validate(&self) -> McpResult<botticelli_narrative::validator::ValidationResult> {
        debug!("Starting validation");
        use botticelli_narrative::validator::Validator;

        let toml = self.to_toml()?;
        trace!(toml_length = toml.len(), "Generated TOML for validation");

        let result = Validator::validate_toml(&toml);

        if result.is_valid() {
            debug!(
                errors = 0,
                warnings = result.warnings().len(),
                "Validation passed"
            );
        } else {
            warn!(
                errors = result.errors().len(),
                warnings = result.warnings().len(),
                "Validation failed"
            );
        }

        Ok(result)
    }

    /// Convert to TOML string.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing.
    #[instrument(skip(self), fields(
        name = ?self.name(),
        acts_count = self.acts().len(),
        act_order_length = self.act_order().len()
    ))]
    pub fn to_toml(&self) -> McpResult<String> {
        debug!("Converting partial narrative to TOML");

        let name = self.name().as_ref().ok_or_else(|| {
            warn!("Missing narrative name");
            McpError::invalid_input("Missing narrative name".to_string())
        })?;

        let description = self.description().as_ref().ok_or_else(|| {
            warn!("Missing narrative description");
            McpError::invalid_input("Missing narrative description".to_string())
        })?;

        trace!(
            name,
            description_length = description.len(),
            "Retrieved required fields"
        );

        let mut toml = String::new();

        // [narrative] section
        toml.push_str("[narrative]\n");
        toml.push_str(&format!("name = \"{}\"\n", name));
        toml.push_str(&format!(
            "description = \"{}\"\n",
            NarrativeHelper::escape_toml_string(description)
        ));

        if let Some(model) = self.model() {
            trace!(model, "Adding model to TOML");
            toml.push_str(&format!("model = \"{}\"\n", model));
        }

        if let Some(temp) = self.temperature() {
            trace!(temperature = temp, "Adding temperature to TOML");
            toml.push_str(&format!("temperature = {}\n", temp));
        }

        if let Some(max) = self.max_tokens() {
            trace!(max_tokens = max, "Adding max_tokens to TOML");
            toml.push_str(&format!("max_tokens = {}\n", max));
        }

        toml.push('\n');

        // [toc] section
        trace!(
            act_count = self.act_order().len(),
            "Generating table of contents"
        );
        toml.push_str("[toc]\n");
        toml.push_str("order = [");
        for (i, act_name) in self.act_order().iter().enumerate() {
            if i > 0 {
                toml.push_str(", ");
            }
            toml.push_str(&format!("\"{}\"", act_name));
        }
        toml.push_str("]\n\n");

        // [acts] section
        trace!("Generating acts section");
        toml.push_str("[acts]\n");
        for act_name in self.act_order() {
            if let Some(act) = self.acts().get(act_name) {
                trace!(act_name, prompt_length = act.prompt().len(), "Adding act");
                toml.push_str(&format!(
                    "{} = \"{}\"\n",
                    act_name,
                    NarrativeHelper::escape_toml_string(act.prompt())
                ));
            } else {
                warn!(act_name, "Act in order but not found in acts map");
            }
        }

        debug!(toml_length = toml.len(), "TOML generation complete");
        Ok(toml)
    }

    /// Attempt to convert to a complete Narrative.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    #[instrument(skip(self), fields(name = ?self.name()))]
    pub fn try_into_narrative(&self) -> McpResult<botticelli_narrative::Narrative> {
        debug!("Attempting to convert to complete narrative");

        let toml = self.to_toml()?;
        let validation = self.validate()?;

        if !validation.is_valid() {
            let error_count = validation.errors().len();
            warn!(errors = error_count, "Validation failed, cannot convert");
            return Err(McpError::invalid_input(format!(
                "Narrative validation failed: {} errors",
                error_count
            )));
        }

        debug!("Validation passed, creating Narrative");
        botticelli_narrative::Narrative::from_toml_str(&toml, self.name().as_deref()).map_err(|e| {
            warn!(error = %e, "Failed to create Narrative from TOML");
            McpError::from(e)
        })
    }
}

impl RegistryOperations for PartialNarrative {
    type Key = String;
    type Error = McpError;

    #[instrument(skip(self))]
    fn registry_key(&self) -> Self::Key {
        let key = self.name().clone().unwrap_or_else(|| "unnamed".to_string());
        trace!(key, "Generated registry key");
        key
    }

    #[instrument(skip(args), fields(args_type = ?args.as_object().map(|_| "object")))]
    fn from_json_args(args: Value) -> Result<Self, Self::Error> {
        debug!("Deserializing PartialNarrative from JSON");

        serde_json::from_value(args).map_err(|e| {
            warn!(error = %e, "Failed to deserialize PartialNarrative");
            McpError::from(e)
        })
    }

    #[instrument(skip(self), fields(
        name = ?self.name(),
        acts_count = self.acts().len()
    ))]
    fn to_json(&self) -> Result<Value, Self::Error> {
        debug!("Serializing PartialNarrative to JSON");

        serde_json::to_value(self).map_err(|e| {
            warn!(error = %e, "Failed to serialize PartialNarrative");
            McpError::from(e)
        })
    }

    #[instrument(skip(self, args), fields(
        name = ?self.name(),
        fields_in_update = args.as_object().map(|o| o.len())
    ))]
    fn update_from_json(&mut self, args: Value) -> Result<(), Self::Error> {
        debug!("Updating PartialNarrative from JSON");

        let obj = args.as_object().ok_or_else(|| {
            warn!("Expected JSON object, got different type");
            McpError::invalid_input("Expected JSON object".to_string())
        })?;

        let mut updated_fields = Vec::new();

        // Update fields present in JSON
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            trace!(name, "Updating name");
            self.with_name(Some(name.to_string()));
            updated_fields.push("name");
        }
        if let Some(desc) = obj.get("description").and_then(|v| v.as_str()) {
            trace!(description_length = desc.len(), "Updating description");
            self.with_description(Some(desc.to_string()));
            updated_fields.push("description");
        }
        if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
            trace!(model, "Updating model");
            self.with_model(Some(model.to_string()));
            updated_fields.push("model");
        }
        if let Some(temp) = obj.get("temperature").and_then(|v| v.as_f64()) {
            trace!(temperature = temp, "Updating temperature");
            self.with_temperature(Some(temp));
            updated_fields.push("temperature");
        }
        if let Some(max) = obj.get("max_tokens").and_then(|v| v.as_u64()) {
            trace!(max_tokens = max, "Updating max_tokens");
            self.with_max_tokens(Some(max as u32));
            updated_fields.push("max_tokens");
        }

        debug!(
            fields_updated = updated_fields.len(),
            fields = ?updated_fields,
            "Update complete"
        );

        Ok(())
    }
}
