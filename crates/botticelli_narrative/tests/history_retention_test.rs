//! Tests for conversation history retention functionality.

mod helpers;

use botticelli_core::{HistoryRetention as HistoryRetentionPolicy, Input, TableFormat};
use botticelli_narrative::HistoryRetention;

#[test]
fn test_summarize_table_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing table input summarization");

    let input = Input::Table {
        table_name: "large_table".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(10),
        offset: None,
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Summary,
    };

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, "Generated table summary");

    assert!(summary.contains("Table: large_table"));
    assert!(summary.contains("10 rows queried"));
    assert!(summary.len() < 100);

    tracing::info!("Table input summarization test passed");
    Ok(())
}

#[test]
fn test_summarize_table_with_offset() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing table summarization with offset");

    let input = Input::Table {
        table_name: "test_table".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(5),
        offset: Some(20),
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Summary,
    };

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, "Generated table summary with offset");

    assert!(summary.contains("Table: test_table"));
    assert!(summary.contains("5 rows queried"));
    assert!(summary.contains("offset 20"));

    tracing::info!("Table summarization with offset test passed");
    Ok(())
}

#[test]
fn test_summarize_table_no_limit() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing table summarization with no limit");

    let input = Input::Table {
        table_name: "all_rows_table".to_string(),
        columns: None,
        where_clause: None,
        limit: None,
        offset: None,
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Summary,
    };

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, "Generated table summary (no limit)");

    assert!(summary.contains("Table: all_rows_table"));
    assert!(summary.contains("all rows"));

    tracing::info!("Table summarization with no limit test passed");
    Ok(())
}

#[test]
fn test_summarize_large_text() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing large text summarization");

    let large_text = "a".repeat(5000);
    let input = Input::Text(large_text);

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, summary_len = summary.len(), "Generated text summary");

    assert!(summary.contains("[Text:"));
    assert!(summary.contains("KB]"));
    assert!(summary.len() < 50);

    tracing::info!("Large text summarization test passed");
    Ok(())
}

#[test]
fn test_summarize_small_text_unchanged() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing small text remains unchanged");

    let small_text = "This is a short text.".to_string();
    let input = Input::Text(small_text.clone());

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(text = %small_text, "Small text should remain unchanged");

    assert_eq!(summary, small_text);

    tracing::info!("Small text unchanged test passed");
    Ok(())
}

#[test]
fn test_summarize_bot_command() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing bot command summarization");

    let input = Input::BotCommand {
        platform: "discord".to_string(),
        command: "server.get_stats".to_string(),
        args: std::collections::HashMap::new(),
        required: false,
        cache_duration: None,
        history_retention: HistoryRetentionPolicy::Summary,
    };

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, "Generated bot command summary");

    assert_eq!(summary, "[Bot command: discord.server.get_stats]");

    tracing::info!("Bot command summarization test passed");
    Ok(())
}

#[test]
fn test_summarize_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing narrative summarization");

    let input = Input::Narrative {
        name: "content_generation".to_string(),
        path: None,
        history_retention: HistoryRetentionPolicy::Summary,
    };

    let summary = HistoryRetention::summarize_input(&input);
    tracing::debug!(summary = %summary, "Generated narrative summary");

    assert_eq!(summary, "[Nested narrative: content_generation]");

    tracing::info!("Narrative summarization test passed");
    Ok(())
}

#[test]
fn test_should_auto_summarize_large_text() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing auto-summarize detection for large text");

    let large_text = "a".repeat(15000);
    let input = Input::Text(large_text);

    let should_summarize = HistoryRetention::should_auto_summarize(&input);
    tracing::debug!(
        should_summarize,
        "Checked if large text should auto-summarize"
    );

    assert!(should_summarize);

    tracing::info!("Auto-summarize large text detection test passed");
    Ok(())
}

#[test]
fn test_should_auto_summarize_small_text() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing auto-summarize detection for small text");

    let small_text = Input::Text("small".to_string());

    let should_summarize = HistoryRetention::should_auto_summarize(&small_text);
    tracing::debug!(
        should_summarize,
        "Checked if small text should auto-summarize"
    );

    assert!(!should_summarize);

    tracing::info!("Auto-summarize small text detection test passed");
    Ok(())
}

#[test]
fn test_apply_retention_full_keeps_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing retention policy 'Full' keeps input");

    let input = Input::Text("Keep this text".to_string());
    let inputs = vec![input.clone()];

    let result = HistoryRetention::apply_retention(&inputs);

    assert_eq!(result.len(), 1);
    let result = HistoryRetention::apply_retention(&inputs);
    tracing::debug!(
        input_count = inputs.len(),
        result_count = result.len(),
        "Applied retention policy 'Full'"
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], input);

    tracing::info!("Retention policy 'Full' keeps input test passed");
    Ok(())
}

#[test]
fn test_apply_retention_summary_replaces_large_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing retention policy 'Summary' replaces large input");

    let input = Input::Table {
        table_name: "test".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(100),
        offset: None,
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Summary,
    };
    let inputs = vec![input];

    let result = HistoryRetention::apply_retention(&inputs);
    tracing::debug!(
        result_count = result.len(),
        "Applied retention policy 'Summary'"
    );

    assert_eq!(result.len(), 1);
    match &result[0] {
        Input::Text(text) => {
            assert!(text.contains("[Table:"));
            assert!(text.len() < 100);
            tracing::debug!(summary = %text, summary_len = text.len(), "Input replaced with summary");
        }
        _ => panic!("Expected Text variant with summary"),
    }

    tracing::info!("Retention policy 'Summary' replaces large input test passed");
    Ok(())
}

#[test]
fn test_apply_retention_drop_removes_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing retention policy 'Drop' removes input");

    let input = Input::Table {
        table_name: "drop_me".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(10),
        offset: None,
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Drop,
    };
    let inputs = vec![input];

    let result = HistoryRetention::apply_retention(&inputs);
    tracing::debug!(
        input_count = inputs.len(),
        result_count = result.len(),
        "Applied retention policy 'Drop'"
    );

    assert_eq!(result.len(), 0);

    tracing::info!("Retention policy 'Drop' removes input test passed");
    Ok(())
}

#[test]
fn test_apply_retention_mixed_policies() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing mixed retention policies");

    let inputs = vec![
        Input::Text("Keep this".to_string()), // Default: Full
        Input::Table {
            table_name: "summarize_me".to_string(),
            columns: None,
            where_clause: None,
            limit: Some(10),
            offset: None,
            order_by: None,
            alias: None,
            format: TableFormat::Json,
            sample: None,
            destructive_read: false,
            history_retention: HistoryRetentionPolicy::Summary,
        },
        Input::Table {
            table_name: "drop_me".to_string(),
            columns: None,
            where_clause: None,
            limit: Some(5),
            offset: None,
            order_by: None,
            alias: None,
            format: TableFormat::Json,
            sample: None,
            destructive_read: false,
            history_retention: HistoryRetentionPolicy::Drop,
        },
    ];

    let result = HistoryRetention::apply_retention(&inputs);

    // First input kept, second summarized, third dropped
    assert_eq!(result.len(), 2);

    // First input should be unchanged
    match &result[0] {
        Input::Text(text) => assert_eq!(text, "Keep this"),
        _ => panic!("Expected Text variant"),
    }

    // Second input should be summarized
    match &result[1] {
        Input::Text(text) => {
            assert!(text.contains("[Table: summarize_me"));
            tracing::debug!(summary = %text, "Second input summarized");
        }
        _ => panic!("Expected Text variant with summary"),
    }
    tracing::debug!(
        result_count = result.len(),
        "Applied mixed retention policies"
    );

    tracing::info!("Mixed retention policies test passed");
    Ok(())
}

#[test]
fn test_apply_retention_auto_summarize_large_full_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing auto-summarize for large input with 'Full' retention");

    let large_text = "a".repeat(15000);
    let input = Input::Text(large_text);
    let inputs = vec![input];

    let result = HistoryRetention::apply_retention(&inputs);
    tracing::debug!(
        result_count = result.len(),
        "Applied retention with auto-summarize"
    );

    // Should be auto-summarized even though retention is Full (default)
    assert_eq!(result.len(), 1);
    match &result[0] {
        Input::Text(text) => {
            assert!(text.contains("[Text:"));
            assert!(text.contains("KB]"));
            assert!(text.len() < 50);
            tracing::debug!(summary = %text, summary_len = text.len(), "Large input auto-summarized");
        }
        _ => panic!("Expected Text variant with summary"),
    }

    tracing::info!("Auto-summarize large input with 'Full' retention test passed");
    Ok(())
}

#[test]
fn test_history_retention_default_is_full() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing history retention default is 'Full'");

    let input = Input::Text("test".to_string());
    let retention = input.history_retention();
    tracing::debug!(retention = ?retention, "Checked default history retention");

    assert_eq!(retention, HistoryRetentionPolicy::Full);

    tracing::info!("History retention default is 'Full' test passed");
    Ok(())
}

#[test]
fn test_with_history_retention_modifies_table() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing with_history_retention modifies table input");

    let input = Input::Table {
        table_name: "test".to_string(),
        columns: None,
        where_clause: None,
        limit: Some(10),
        offset: None,
        order_by: None,
        alias: None,
        format: TableFormat::Json,
        sample: None,
        destructive_read: false,
        history_retention: HistoryRetentionPolicy::Full,
    };

    let modified = input.with_history_retention(HistoryRetentionPolicy::Summary);
    tracing::debug!(retention = ?modified.history_retention(), "Modified table history retention");

    assert_eq!(
        modified.history_retention(),
        HistoryRetentionPolicy::Summary
    );

    tracing::info!("with_history_retention modifies table test passed");
    Ok(())
}

#[test]
fn test_with_history_retention_modifies_bot_command() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing with_history_retention modifies bot command");

    let input = Input::BotCommand {
        platform: "discord".to_string(),
        command: "test".to_string(),
        args: std::collections::HashMap::new(),
        required: false,
        cache_duration: None,
        history_retention: HistoryRetentionPolicy::Full,
    };

    let modified = input.with_history_retention(HistoryRetentionPolicy::Drop);
    tracing::debug!(retention = ?modified.history_retention(), "Modified bot command history retention");

    assert_eq!(modified.history_retention(), HistoryRetentionPolicy::Drop);

    tracing::info!("with_history_retention modifies bot command test passed");
    Ok(())
}

#[test]
fn test_with_history_retention_modifies_narrative() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing with_history_retention modifies narrative");

    let input = Input::Narrative {
        name: "test".to_string(),
        path: None,
        history_retention: HistoryRetentionPolicy::Full,
    };

    let modified = input.with_history_retention(HistoryRetentionPolicy::Summary);
    tracing::debug!(retention = ?modified.history_retention(), "Modified narrative history retention");

    assert_eq!(
        modified.history_retention(),
        HistoryRetentionPolicy::Summary
    );

    tracing::info!("with_history_retention modifies narrative test passed");
    Ok(())
}

#[test]
fn test_with_history_retention_ignores_text_input() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing with_history_retention ignores text input");

    let input = Input::Text("test".to_string());

    let modified = input.with_history_retention(HistoryRetentionPolicy::Summary);
    tracing::debug!(retention = ?modified.history_retention(), "Text input retention unchanged");

    // Text inputs don't support history_retention, should still be Full
    assert_eq!(modified.history_retention(), HistoryRetentionPolicy::Full);

    tracing::info!("with_history_retention ignores text input test passed");
    Ok(())
}
