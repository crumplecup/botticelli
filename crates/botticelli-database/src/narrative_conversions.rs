//! Conversions between domain types and database models for narrative executions.

use crate::{
    ActExecution, ActExecutionRow, ActInputRow, BackendError, BotticelliError, BotticelliResult,
    ExecutionStatus, Input, NarrativeExecution, NarrativeExecutionRow, NewActExecutionRow,
    NewActInputRow, NewNarrativeExecutionRow,
};
use chrono::Utc;

/// Convert ExecutionStatus to database string.
pub fn status_to_string(status: ExecutionStatus) -> String {
    match status {
        ExecutionStatus::Running => "running".to_string(),
        ExecutionStatus::Completed => "completed".to_string(),
        ExecutionStatus::Failed => "failed".to_string(),
    }
}

/// Convert database string to ExecutionStatus.
pub fn string_to_status(s: &str) -> BotticelliResult<ExecutionStatus> {
    s.parse().map_err(|e| {
        BotticelliError::from(BackendError::new(format!(
            "Invalid execution status: {}",
            e
        )))
    })
}

/// Convert NarrativeExecution to NewNarrativeExecutionRow.
pub fn execution_to_new_row(
    execution: &NarrativeExecution,
    status: ExecutionStatus,
) -> NewNarrativeExecutionRow {
    let now = Utc::now().naive_utc();
    let completed = matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed);

    NewNarrativeExecutionRow {
        narrative_name: execution.narrative_name.clone(),
        narrative_description: None, // Not available in current NarrativeExecution
        started_at: now,
        completed_at: if completed { Some(now) } else { None },
        status: status_to_string(status),
        error_message: None,
    }
}

/// Convert ActExecution to NewActExecutionRow.
pub fn act_execution_to_new_row(act: &ActExecution, execution_id: i32) -> NewActExecutionRow {
    NewActExecutionRow {
        execution_id,
        act_name: act.act_name.clone(),
        sequence_number: act.sequence_number as i32,
        model: act.model.clone(),
        temperature: act.temperature,
        max_tokens: act.max_tokens.map(|t| t as i32),
        response: act.response.clone(),
    }
}

/// Convert Input to NewActInputRow.
pub fn input_to_new_row(
    input: &Input,
    act_execution_id: i32,
    order: usize,
) -> BotticelliResult<NewActInputRow> {
    let row = match input {
        Input::Text(text) => NewActInputRow {
            act_execution_id,
            input_order: order as i32,
            input_type: "text".to_string(),
            text_content: Some(text.clone()),
            mime_type: None,
            filename: None,
            media_ref_id: None,
        },
        Input::Image { mime, .. } | Input::Audio { mime, .. } | Input::Video { mime, .. } => {
            // Media inputs require special handling via media storage system
            // media_ref_id should be populated by the caller after storing media
            NewActInputRow {
                act_execution_id,
                input_order: order as i32,
                input_type: input_type_string(input),
                text_content: None,
                mime_type: mime.clone(),
                filename: None,
                media_ref_id: None, // Will be populated by caller
            }
        }
        Input::Document { mime, filename, .. } => {
            // Document inputs require special handling via media storage system
            NewActInputRow {
                act_execution_id,
                input_order: order as i32,
                input_type: "document".to_string(),
                text_content: None,
                mime_type: mime.clone(),
                filename: filename.clone(),
                media_ref_id: None, // Will be populated by caller
            }
        }
    };
    Ok(row)
}

/// Get input type string for database.
fn input_type_string(input: &Input) -> String {
    match input {
        Input::Text(_) => "text".to_string(),
        Input::Image { .. } => "image".to_string(),
        Input::Audio { .. } => "audio".to_string(),
        Input::Video { .. } => "video".to_string(),
        Input::Document { .. } => "document".to_string(),
    }
}

/// Reconstruct ActExecution from database rows.
pub fn rows_to_act_execution(
    act_row: ActExecutionRow,
    input_rows: Vec<ActInputRow>,
) -> BotticelliResult<ActExecution> {
    let mut inputs = Vec::with_capacity(input_rows.len());

    // Sort by input_order to maintain correct sequence
    let mut sorted_inputs = input_rows;
    sorted_inputs.sort_by_key(|row| row.input_order);

    for input_row in sorted_inputs {
        inputs.push(row_to_input(input_row)?);
    }

    Ok(ActExecution {
        act_name: act_row.act_name,
        inputs,
        model: act_row.model,
        temperature: act_row.temperature,
        max_tokens: act_row.max_tokens.map(|t| t as u32),
        response: act_row.response,
        sequence_number: act_row.sequence_number as usize,
    })
}

/// Convert ActInputRow to Input.
fn row_to_input(row: ActInputRow) -> BotticelliResult<Input> {
    match row.input_type.as_str() {
        "text" => {
            let text = row.text_content.ok_or_else(|| {
                BotticelliError::from(BackendError::new(
                    "Text input missing text_content".to_string(),
                ))
            })?;
            Ok(Input::Text(text))
        }
        "image" | "audio" | "video" | "document" => {
            // Media inputs require loading via media_ref_id
            Err(BotticelliError::from(BackendError::new(format!(
                "Media input reconstruction not yet implemented for type: {} (use media_ref_id)",
                row.input_type
            ))))
        }
        _ => Err(BotticelliError::from(BackendError::new(format!(
            "Unknown input type: {}",
            row.input_type
        )))),
    }
}

/// Reconstruct NarrativeExecution from database rows.
pub fn rows_to_narrative_execution(
    _execution_row: &NarrativeExecutionRow,
    narrative_name: String,
    act_executions: Vec<ActExecution>,
) -> NarrativeExecution {
    NarrativeExecution {
        narrative_name,
        act_executions,
    }
}
