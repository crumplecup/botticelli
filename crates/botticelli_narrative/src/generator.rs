//! `NarrativeGenerator` — interactive creator for `TomlNarrativeFile`.
//!
//! Implements the `Generator` pattern from the elicitation framework:
//! elicit the generator once (a short, human-friendly Q&A), then call
//! `.generate()` to produce the full `TomlNarrativeFile`.
//!
//! This keeps the 40-field `TomlNarrativeFile` elicitation out of the wizard;
//! only the fields a human actually needs to provide are asked.

use std::collections::HashMap;

use elicitation::Generator;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    CarouselConfig, TomlAct, TomlActConfig, TomlActInput, TomlInput, TomlNarrative,
    TomlNarrativeData, TomlNarrativeFile, TomlToc,
};

// ── Act content variants ──────────────────────────────────────────────────────

/// Content of a single act in the wizard.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("What type of act is this?")]
pub enum NarrativeActContent {
    /// A plain text prompt sent directly to the LLM.
    #[prompt("Write the prompt text that will be sent to the LLM for this act:")]
    Prompt(String),

    /// Delegate to another narrative file (enter its key, e.g. `research`).
    #[prompt("Enter the key of the narrative to run as this act (the file stem, e.g. 'research'):")]
    NarrativeRef(String),

    /// Repeat this act over a dataset with budget-aware iteration.
    #[prompt("Configure the carousel (repeated execution over a dataset):")]
    Carousel(NarrativeCarouselSpec),
}

/// Minimal carousel configuration for the wizard.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("Configure the carousel act:")]
pub struct NarrativeCarouselSpec {
    /// Prompt template run on each carousel iteration.
    #[prompt("Prompt to use for each carousel iteration:")]
    pub prompt: String,

    /// Maximum number of iterations.
    #[prompt("Maximum number of iterations (e.g. 10):")]
    pub iterations: u32,
}

// ── Act spec ──────────────────────────────────────────────────────────────────

/// A single act as specified in the wizard.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("Define an act:")]
pub struct NarrativeActSpec {
    /// Act key (used as the TOML table key and in TOC ordering).
    #[prompt(
        "Act name — a short identifier used as the key (e.g. 'research', 'draft', 'review'):"
    )]
    pub name: String,

    /// What this act should do.
    #[prompt("What should this act do?")]
    pub content: NarrativeActContent,
}

// ── Top-level generator ───────────────────────────────────────────────────────

/// Generator for `TomlNarrativeFile`.
///
/// Elicit this type for interactive narrative creation. After all fields are
/// collected, call `.generate()` to obtain the ready-to-serialise
/// `TomlNarrativeFile`.
///
/// Only the fields a human needs to supply are asked; implementation details
/// (table columns, WHERE clauses, history retention, etc.) are inferred.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("Let's create a narrative. I'll ask for the name, description, and acts in order.")]
pub struct NarrativeGenerator {
    /// Narrative name — used as the file key.
    #[prompt("Narrative name — a short identifier used as the file key (e.g. 'article_draft'):")]
    pub name: String,

    /// Human-readable purpose of this narrative.
    #[prompt("Describe what this narrative does (shown in logs and tooling):")]
    pub description: String,

    /// Acts in execution order — the framework will ask "add another?" until done.
    #[prompt(
        "Define the acts (AI prompt steps). You will be asked after each whether to add another."
    )]
    pub acts: Vec<NarrativeActSpec>,

    /// Default LLM model for all acts (optional).
    #[prompt(
        "Default LLM model for all acts? (optional, e.g. claude-sonnet-4-6 — press Enter to skip)"
    )]
    pub model: Option<String>,
}

// ── Generator impl ────────────────────────────────────────────────────────────

impl Generator for NarrativeGenerator {
    type Target = TomlNarrativeFile;

    #[instrument(skip(self), fields(name = %self.name, act_count = self.acts.len()))]
    fn generate(&self) -> TomlNarrativeFile {
        let toc: Vec<String> = self.acts.iter().map(|a| a.name.clone()).collect();

        let mut acts: HashMap<String, TomlAct> = HashMap::new();
        for act_spec in &self.acts {
            let act = build_act(act_spec);
            acts.insert(act_spec.name.clone(), act);
        }

        let narrative = TomlNarrative {
            name: self.name.clone(),
            description: self.description.clone(),
            model: self.model.clone(),
            ..Default::default()
        };

        TomlNarrativeFile {
            acts,
            narrative_data: TomlNarrativeData::Single {
                narrative: Box::new(Some(narrative)),
                toc: Some(TomlToc::Array(toc)),
            },
            ..Default::default()
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_act(spec: &NarrativeActSpec) -> TomlAct {
    match &spec.content {
        NarrativeActContent::Prompt(text) => TomlAct::Simple(text.clone()),

        NarrativeActContent::NarrativeRef(key) => {
            TomlAct::Array(vec![TomlActInput::String(format!("narratives.{}", key))])
        }

        NarrativeActContent::Carousel(carousel) => TomlAct::Structured(TomlActConfig {
            input: vec![TomlInput {
                input_type: Some("text".to_string()),
                content: Some(carousel.prompt.clone()),
                ..Default::default()
            }],
            carousel: Some(CarouselConfig::new(carousel.iterations, 1000)),
            ..Default::default()
        }),
    }
}
