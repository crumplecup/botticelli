//! Conversions between domain types and database models for narrative executions.

use crate::{
    ActExecution, ActExecutionRow, ActInputRow, BoticelliError, BoticelliErrorKind,
    BoticelliResult, ExecutionStatus, Input, MediaSource, NarrativeExecution,
    NarrativeExecutionRow, NewActExecutionRow, NewActInputRow, NewNarrativeExecutionRow,
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
pub fn string_to_status(s: &str) -> BoticelliResult<ExecutionStatus> {
    s.parse().map_err(|e| {
        BoticelliError::new(BoticelliErrorKind::Backend(format!(
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
pub fn act_execution_to_new_row(
    act: &ActExecution,
    execution_id: i32,
) -> NewActExecutionRow {
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
pub fn input_to_new_row(input: &Input, act_execution_id: i32, order: usize) -> BoticelliResult<NewActInputRow> {
    let mut row = NewActInputRow {
        act_execution_id,
        input_order: order as i32,
        input_type: input_type_string(input),
        text_content: None,
        mime_type: None,
        source_type: None,
        source_url: None,
        source_base64: None,
        source_binary: None,
        source_size_bytes: None,
        content_hash: None,
        filename: None,
    };

    match input {
        Input::Text(text) => {
            row.text_content = Some(text.clone());
        }
        Input::Image { mime, source } => {
            row.mime_type = mime.clone();
            populate_media_source(&mut row, source)?;
        }
        Input::Audio { mime, source } => {
            row.mime_type = mime.clone();
            populate_media_source(&mut row, source)?;
        }
        Input::Video { mime, source } => {
            row.mime_type = mime.clone();
            populate_media_source(&mut row, source)?;
        }
        Input::Document {
            mime,
            source,
            filename,
        } => {
            row.mime_type = mime.clone();
            row.filename = filename.clone();
            populate_media_source(&mut row, source)?;
        }
    }

    Ok(row)
}

/// Helper to populate media source fields in ActInputRow.
fn populate_media_source(row: &mut NewActInputRow, source: &MediaSource) -> BoticelliResult<()> {
    match source {
        MediaSource::Url(url) => {
            row.source_type = Some("url".to_string());
            row.source_url = Some(url.clone());
            row.source_size_bytes = Some(url.len() as i64);
        }
        MediaSource::Base64(base64) => {
            row.source_type = Some("base64".to_string());
            row.source_base64 = Some(base64.clone());
            row.source_size_bytes = Some(base64.len() as i64);
            // Could compute content hash here if needed
        }
        MediaSource::Binary(bytes) => {
            row.source_type = Some("binary".to_string());
            row.source_binary = Some(bytes.clone());
            row.source_size_bytes = Some(bytes.len() as i64);
            // Could compute content hash here if needed
        }
    }
    Ok(())
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
) -> BoticelliResult<ActExecution> {
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
fn row_to_input(row: ActInputRow) -> BoticelliResult<Input> {
    match row.input_type.as_str() {
        "text" => {
            let text = row.text_content.ok_or_else(|| {
                BoticelliError::new(BoticelliErrorKind::Backend(
                    "Text input missing text_content".to_string(),
                ))
            })?;
            Ok(Input::Text(text))
        }
        "image" => {
            let source = row_to_media_source(&row)?;
            Ok(Input::Image {
                mime: row.mime_type,
                source,
            })
        }
        "audio" => {
            let source = row_to_media_source(&row)?;
            Ok(Input::Audio {
                mime: row.mime_type,
                source,
            })
        }
        "video" => {
            let source = row_to_media_source(&row)?;
            Ok(Input::Video {
                mime: row.mime_type,
                source,
            })
        }
        "document" => {
            let source = row_to_media_source(&row)?;
            Ok(Input::Document {
                mime: row.mime_type,
                source,
                filename: row.filename,
            })
        }
        _ => Err(BoticelliError::new(BoticelliErrorKind::Backend(format!(
            "Unknown input type: {}",
            row.input_type
        )))),
    }
}

/// Reconstruct MediaSource from ActInputRow.
fn row_to_media_source(row: &ActInputRow) -> BoticelliResult<MediaSource> {
    match row.source_type.as_deref() {
        Some("url") => {
            let url = row.source_url.clone().ok_or_else(|| {
                BoticelliError::new(BoticelliErrorKind::Backend(
                    "URL source missing source_url".to_string(),
                ))
            })?;
            Ok(MediaSource::Url(url))
        }
        Some("base64") => {
            let base64 = row.source_base64.clone().ok_or_else(|| {
                BoticelliError::new(BoticelliErrorKind::Backend(
                    "Base64 source missing source_base64".to_string(),
                ))
            })?;
            Ok(MediaSource::Base64(base64))
        }
        Some("binary") => {
            let binary = row.source_binary.clone().ok_or_else(|| {
                BoticelliError::new(BoticelliErrorKind::Backend(
                    "Binary source missing source_binary".to_string(),
                ))
            })?;
            Ok(MediaSource::Binary(binary))
        }
        Some(other) => Err(BoticelliError::new(BoticelliErrorKind::Backend(format!(
            "Unknown source type: {}",
            other
        )))),
        None => Err(BoticelliError::new(BoticelliErrorKind::Backend(
            "Media source type not specified".to_string(),
        ))),
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
