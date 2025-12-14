//! Conversation history retention utilities.
//!
//! This module provides functions for summarizing and managing conversation history
//! to optimize token usage in multi-act narratives.

use botticelli_core::Input;
use tracing::{debug, instrument};

/// Auto-summary threshold for large inputs (10KB).
///
/// Inputs larger than this threshold will automatically be summarized
/// even if history_retention is set to Full, as a safety measure.
pub const AUTO_SUMMARY_THRESHOLD: usize = 10_000;

/// Generate a concise summary for an input.
///
/// The summary is designed to be informative while being much smaller than
/// the original input, optimizing token usage in conversation history.
///
/// # Format
///
/// - **Table**: `[Table: {name}, {limit} rows queried{offset}]`
/// - **Text (large)**: `[Text: ~{size}KB]`
/// - **Narrative**: `[Nested narrative: {name}]`
/// - **BotCommand**: `[Bot command: {platform}.{command}]`
/// - **Image/Audio/Video/Document**: `[{type}: {mime or "unknown"}]`
///
/// # Examples
///
/// ```
/// use botticelli_core::Input;
/// use botticelli_narrative::summarize_input;
///
/// let input = Input::Text("a".repeat(5000));
/// let summary = summarize_input(&input);
/// assert!(summary.contains("[Text:"));
/// assert!(summary.len() < 50); // Much smaller than 5000 chars
/// ```
#[instrument(skip(input), fields(input_type = ?std::mem::discriminant(input)))]
pub fn summarize_input(input: &Input) -> String {
    match input {
        Input::Table {
            table_name,
            limit,
            offset,
            ..
        } => {
            let rows_info = limit
                .map(|l| format!("{} rows queried", l))
                .unwrap_or_else(|| "all rows".to_string());
            let offset_info = offset
                .map(|o| format!(", offset {}", o))
                .unwrap_or_default();
            let summary = format!("[Table: {}, {}{}]", table_name, rows_info, offset_info);
            debug!(summary = %summary, "Generated table summary");
            summary
        }
        Input::Text(content) => {
            if content.len() > 1000 {
                let size_kb = content.len() / 1024;
                let summary = format!("[Text: ~{}KB]", size_kb);
                debug!(summary = %summary, original_size = content.len(), "Generated text summary");
                summary
            } else {
                debug!(size = content.len(), "Text is small, keeping as-is");
                content.clone()
            }
        }
        Input::Narrative { name, .. } => {
            let summary = format!("[Nested narrative: {}]", name);
            debug!(summary = %summary, "Generated narrative summary");
            summary
        }
        Input::BotCommand {
            platform, command, ..
        } => {
            let summary = format!("[Bot command: {}.{}]", platform, command);
            debug!(summary = %summary, "Generated bot command summary");
            summary
        }
        Input::Image { mime, .. } => {
            let mime_str = mime.as_deref().unwrap_or("unknown");
            let summary = format!("[Image: {}]", mime_str);
            debug!(summary = %summary, "Generated image summary");
            summary
        }
        Input::Audio { mime, .. } => {
            let mime_str = mime.as_deref().unwrap_or("unknown");
            let summary = format!("[Audio: {}]", mime_str);
            debug!(summary = %summary, "Generated audio summary");
            summary
        }
        Input::Video { mime, .. } => {
            let mime_str = mime.as_deref().unwrap_or("unknown");
            let summary = format!("[Video: {}]", mime_str);
            debug!(summary = %summary, "Generated video summary");
            summary
        }
        Input::Document { mime, .. } => {
            let mime_str = mime.as_deref().unwrap_or("unknown");
            let summary = format!("[Document: {}]", mime_str);
            debug!(summary = %summary, "Generated document summary");
            summary
        }
        Input::ToolCall { name, .. } => {
            // Tool calls are structural and should not be summarized
            let summary = format!("[Tool call: {}]", name);
            debug!(summary = %summary, "Tool call (no summarization)");
            summary
        }
        Input::ToolResult { tool_call_id, is_error, .. } => {
            // Tool results are structural and should not be summarized
            let status = if *is_error { "error" } else { "success" };
            let summary = format!("[Tool result: {} ({})]", tool_call_id, status);
            debug!(summary = %summary, "Tool result (no summarization)");
            summary
        }
    }
}

/// Check if an input should be auto-summarized based on size.
///
/// Returns `true` if the input exceeds `AUTO_SUMMARY_THRESHOLD` (10KB).
///
/// # Examples
///
/// ```
/// use botticelli_core::Input;
/// use botticelli_narrative::should_auto_summarize;
///
/// let small = Input::Text("small".to_string());
/// assert!(!should_auto_summarize(&small));
///
/// let large = Input::Text("a".repeat(15000));
/// assert!(should_auto_summarize(&large));
/// ```
#[instrument(skip(input), fields(input_type = ?std::mem::discriminant(input)))]
pub fn should_auto_summarize(input: &Input) -> bool {
    let size = estimate_input_size(input);
    let should_summarize = size > AUTO_SUMMARY_THRESHOLD;

    if should_summarize {
        debug!(
            size,
            threshold = AUTO_SUMMARY_THRESHOLD,
            "Input exceeds auto-summary threshold"
        );
    }

    should_summarize
}

/// Estimate the size of an input in bytes.
///
/// This is used to determine if an input should be auto-summarized.
fn estimate_input_size(input: &Input) -> usize {
    match input {
        Input::Text(content) => content.len(),
        Input::Table { .. } => {
            // Estimate table size (conservative: assume 1KB per row * limit)
            // Actual size will be determined after query execution
            // For now, return 0 to avoid premature summarization
            0
        }
        Input::BotCommand { .. } => {
            // Bot commands are typically small
            100
        }
        Input::Narrative { .. } => {
            // Narrative size unknown until execution
            0
        }
        Input::Image { source, .. }
        | Input::Audio { source, .. }
        | Input::Video { source, .. }
        | Input::Document { source, .. } => {
            use botticelli_core::MediaSource;
            match source {
                MediaSource::Binary(data) => data.len(),
                MediaSource::Base64(data) => data.len(),
                MediaSource::Url(_) => 0, // URL itself is small
            }
        }
        Input::ToolCall { arguments, .. } => {
            // Estimate size of arguments JSON
            arguments.to_string().len()
        }
        Input::ToolResult { content, .. } => {
            // Tool results are typically small
            content.len()
        }
    }
}

/// Apply retention policies to a vec of inputs, returning a new vec with policies applied.
///
/// This function processes each input according to its retention policy:
/// - **Full**: Keep the input as-is
/// - **Summary**: Replace with a concise summary
/// - **Drop**: Remove the input entirely
///
/// # Examples
///
/// ```
/// use botticelli_core::{Input, HistoryRetention};
/// use botticelli_narrative::apply_retention_to_inputs;
///
/// let inputs = vec![
///     Input::Text("Keep this".to_string()),
///     Input::Text("a".repeat(5000)), // Will be summarized if retention = Summary
/// ];
///
/// let result = apply_retention_to_inputs(&inputs);
/// assert_eq!(result.len(), 2);
/// ```
#[instrument(skip(inputs), fields(input_count = inputs.len()))]
pub fn apply_retention_to_inputs(inputs: &[Input]) -> Vec<Input> {
    let mut result = Vec::new();

    for input in inputs {
        let retention = input.history_retention();

        match retention {
            botticelli_core::HistoryRetention::Full => {
                // Check if auto-summary is needed
                if should_auto_summarize(input) {
                    debug!(
                        input_type = ?std::mem::discriminant(input),
                        "Auto-summarizing large input"
                    );
                    let summary = summarize_input(input);
                    result.push(Input::Text(summary));
                } else {
                    result.push(input.clone());
                }
            }
            botticelli_core::HistoryRetention::Summary => {
                debug!(
                    input_type = ?std::mem::discriminant(input),
                    "Summarizing input per retention policy"
                );
                let summary = summarize_input(input);
                result.push(Input::Text(summary));
            }
            botticelli_core::HistoryRetention::Drop => {
                debug!(
                    input_type = ?std::mem::discriminant(input),
                    "Dropping input per retention policy"
                );
                // Don't add to result - effectively drops it
            }
        }
    }

    debug!(
        original_count = inputs.len(),
        result_count = result.len(),
        "Applied retention policies to inputs"
    );

    result
}
