#![cfg(feature = "database")]

use boticelli::{
    ActExecution, ActProcessor, ContentGenerationProcessor, Input, NarrativeMetadata,
    ProcessorContext, establish_connection,
};
use std::sync::{Arc, Mutex};

fn create_test_execution(act_name: &str, response: &str) -> ActExecution {
    ActExecution {
        act_name: act_name.to_string(),
        inputs: vec![Input::Text("test input".to_string())],
        model: None,
        temperature: None,
        max_tokens: None,
        response: response.to_string(),
        sequence_number: 0,
    }
}

fn create_test_metadata(name: &str, template: Option<String>) -> NarrativeMetadata {
    NarrativeMetadata {
        name: name.to_string(),
        description: "Test narrative".to_string(),
        template,
        skip_content_generation: false,
    }
}

fn create_test_metadata_with_skip(name: &str, skip: bool) -> NarrativeMetadata {
    NarrativeMetadata {
        name: name.to_string(),
        description: "Test narrative".to_string(),
        template: None,
        skip_content_generation: skip,
    }
}

fn create_test_context<'a>(
    execution: &'a ActExecution,
    metadata: &'a NarrativeMetadata,
    narrative_name: &'a str,
) -> ProcessorContext<'a> {
    ProcessorContext {
        execution,
        narrative_metadata: metadata,
        narrative_name,
    }
}

fn create_test_processor() -> ContentGenerationProcessor {
    let conn = Arc::new(Mutex::new(
        establish_connection().expect("DB connection failed"),
    ));
    ContentGenerationProcessor::new(conn)
}

#[test]
fn test_should_process_with_template() {
    let execution = create_test_execution("test", "test response");
    let metadata = create_test_metadata("test_table", Some("discord_channels".to_string()));
    let context = create_test_context(&execution, &metadata, "test_narrative");

    let processor = create_test_processor();

    assert!(processor.should_process(&context));
}

#[test]
fn test_should_process_without_template_for_inference() {
    let execution = create_test_execution("test", "test response");
    let metadata = create_test_metadata("test_table", None);
    let context = create_test_context(&execution, &metadata, "test_narrative");

    let processor = create_test_processor();

    // Now processes even without template (inference mode)
    assert!(processor.should_process(&context));
}

#[test]
fn test_should_not_process_when_opted_out() {
    let execution = create_test_execution("test", "test response");
    let metadata = create_test_metadata_with_skip("test_table", true);
    let context = create_test_context(&execution, &metadata, "test_narrative");

    let processor = create_test_processor();

    // Should not process when skip_content_generation is true
    assert!(!processor.should_process(&context));
}

#[test]
fn test_processor_name() {
    let processor = create_test_processor();
    assert_eq!(processor.name(), "ContentGenerationProcessor");
}
