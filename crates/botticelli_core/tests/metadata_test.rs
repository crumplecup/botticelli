//! Tests for metadata and capabilities types.

mod helpers;

use botticelli_core::{Capabilities, ModelMetadataBuilder};

#[test]
fn test_capabilities_from_metadata() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    
    let metadata = ModelMetadataBuilder::default()
        .provider("test")
        .model("test-model".to_string())
        .max_input_tokens(100000)
        .max_output_tokens(4096)
        .supports_streaming(true)
        .supports_vision(false)
        .supports_audio(true)
        .supports_video(false)
        .supports_documents(true)
        .supports_tool_use(true)
        .supports_json_mode(false)
        .supports_embeddings(false)
        .supports_batch(true)
        .build()?;

    let caps = Capabilities::from(&metadata);

    assert_eq!(*caps.streaming(), true);
    assert_eq!(*caps.tool_calling(), true);
    assert_eq!(*caps.vision(), false);
    assert_eq!(*caps.audio(), true);
    assert_eq!(*caps.video(), false);
    assert_eq!(*caps.embeddings(), false);
    assert_eq!(*caps.json_mode(), false);
    assert_eq!(*caps.batch_generation(), true);

    Ok(())
}

#[test]
fn test_capabilities_from_metadata_all_disabled() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    
    let metadata = ModelMetadataBuilder::default()
        .provider("basic")
        .model("basic-model".to_string())
        .max_input_tokens(1000)
        .max_output_tokens(500)
        .supports_streaming(false)
        .supports_vision(false)
        .supports_audio(false)
        .supports_video(false)
        .supports_documents(false)
        .supports_tool_use(false)
        .supports_json_mode(false)
        .supports_embeddings(false)
        .supports_batch(false)
        .build()?;

    let caps = Capabilities::from(&metadata);

    assert_eq!(*caps.streaming(), false);
    assert_eq!(*caps.tool_calling(), false);
    assert_eq!(*caps.vision(), false);
    assert_eq!(*caps.audio(), false);
    assert_eq!(*caps.video(), false);
    assert_eq!(*caps.embeddings(), false);
    assert_eq!(*caps.json_mode(), false);
    assert_eq!(*caps.batch_generation(), false);

    Ok(())
}

#[test]
fn test_capabilities_from_metadata_all_enabled() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    
    let metadata = ModelMetadataBuilder::default()
        .provider("advanced")
        .model("advanced-model".to_string())
        .max_input_tokens(200000)
        .max_output_tokens(8192)
        .supports_streaming(true)
        .supports_vision(true)
        .supports_audio(true)
        .supports_video(true)
        .supports_documents(true)
        .supports_tool_use(true)
        .supports_json_mode(true)
        .supports_embeddings(true)
        .supports_batch(true)
        .build()?;

    let caps = Capabilities::from(&metadata);

    assert_eq!(*caps.streaming(), true);
    assert_eq!(*caps.tool_calling(), true);
    assert_eq!(*caps.vision(), true);
    assert_eq!(*caps.audio(), true);
    assert_eq!(*caps.video(), true);
    assert_eq!(*caps.embeddings(), true);
    assert_eq!(*caps.json_mode(), true);
    assert_eq!(*caps.batch_generation(), true);

    Ok(())
}
