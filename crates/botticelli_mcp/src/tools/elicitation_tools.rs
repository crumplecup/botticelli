//! Elicitation tool implementations.

use crate::{
    ElicitBoolParams, ElicitBoolResult, ElicitNumberParams, ElicitNumberResult,
    ElicitSelectParams, ElicitSelectResult, ElicitTextParams, ElicitTextResult,
};
use rmcp::tool;
use tracing::instrument;

/// Elicit a boolean (yes/no) confirmation from the user.
///
/// Presents a prompt and returns the user's boolean response.
#[tool]
#[instrument(skip(_params), fields(prompt = %_params.prompt, default = _params.default))]
pub async fn elicit_bool(_params: ElicitBoolParams) -> Result<ElicitBoolResult, rmcp::ErrorData> {
    // TODO: Implement elicitation using elicitation crate
    // This requires:
    // 1. Integration with elicitation session management
    // 2. Proper async handling
    // 3. Transport layer communication
    
    tracing::warn!("elicit_bool not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "elicit_bool requires elicitation session integration",
        None,
    ))
}

/// Elicit free-form text input from the user.
///
/// Presents a prompt and returns the user's text response.
#[tool]
#[instrument(skip(_params), fields(prompt = %_params.prompt))]
pub async fn elicit_text(_params: ElicitTextParams) -> Result<ElicitTextResult, rmcp::ErrorData> {
    // TODO: Implement elicitation using elicitation crate
    
    tracing::warn!("elicit_text not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "elicit_text requires elicitation session integration",
        None,
    ))
}

/// Elicit a number within a specified range from the user.
///
/// Presents a prompt with min/max constraints and returns the user's numeric input.
#[tool]
#[instrument(skip(_params), fields(prompt = %_params.prompt, min = _params.min, max = _params.max))]
pub async fn elicit_number(_params: ElicitNumberParams) -> Result<ElicitNumberResult, rmcp::ErrorData> {
    // TODO: Implement elicitation using elicitation crate
    
    tracing::warn!("elicit_number not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "elicit_number requires elicitation session integration",
        None,
    ))
}

/// Elicit a selection from a list of options.
///
/// Presents options to the user and returns their selected choice.
#[tool]
#[instrument(skip(_params), fields(prompt = %_params.prompt, option_count = _params.options.len()))]
pub async fn elicit_select(_params: ElicitSelectParams) -> Result<ElicitSelectResult, rmcp::ErrorData> {
    // TODO: Implement elicitation using elicitation crate
    
    tracing::warn!("elicit_select not yet implemented");
    Err(rmcp::ErrorData::new(
        rmcp::model::ErrorCode::METHOD_NOT_FOUND,
        "elicit_select requires elicitation session integration",
        None,
    ))
}
