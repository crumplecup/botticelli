//! Narrative session management tools.
//!
//! Create and manage narrative generation sessions.

use crate::rmcp_server::BotticelliServer;
use crate::tools::NarrativeHelper;
use crate::{CreateNarrativeSessionParams, CreateNarrativeSessionResult, NarrativeAnalysis};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

impl BotticelliServer {
    /// Create a new narrative session from a description.
    #[instrument(skip(self, params), fields(description_len = params.description().len()))]
    pub async fn create_narrative_session(
        &self,
        Parameters(params): Parameters<CreateNarrativeSessionParams>,
    ) -> Result<Json<CreateNarrativeSessionResult>, rmcp::ErrorData> {
        let description = params.description().clone();
        debug!(?description, "Creating narrative session");

        // Analyze description
        let acts = NarrativeHelper::extract_acts_from_description(&description);
        let suggested_name = NarrativeHelper::suggest_name_from_description(&description);

        let complexity = if acts.len() == 1 {
            "simple"
        } else if acts.len() <= 3 {
            "moderate"
        } else {
            "complex"
        };

        // Initialize session state
        let mut partial = crate::PartialNarrative::new();
        partial.with_description(Some(description.clone()));
        partial.with_name(Some(suggested_name.clone()));

        // Add acts
        for act in &acts {
            partial.acts_mut().insert(
                act.name.clone(),
                crate::PartialAct::new(act.prompt.clone(), None, None, vec![], None),
            );
            partial.act_order_mut().push(act.name.clone());
        }

        // Store in registry (returns the narrative name as the key/ID)
        let narrative_id = self.narrative_registry().add(partial);

        debug!(narrative_id = %narrative_id, acts = acts.len(), "Session created");

        Ok(Json(CreateNarrativeSessionResult::new(
            narrative_id,
            suggested_name,
            NarrativeAnalysis::new(
                acts.iter().map(|a| a.name.clone()).collect(),
                complexity.to_string(),
                acts.len(),
            ),
        )))
    }
}
