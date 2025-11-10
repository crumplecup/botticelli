use async_trait::async_trait;
use boticelli::{
    ActConfig, BoticelliDriver, BoticelliResult, GenerateRequest, GenerateResponse, Input,
    Narrative, NarrativeExecutor, NarrativeProvider, Output,
};

/// Helper to extract text from inputs for testing.
fn get_text_from_inputs(inputs: &[Input]) -> String {
    inputs
        .iter()
        .filter_map(|input| {
            if let Input::Text(text) = input {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mock LLM driver for testing that echoes the prompt with a prefix.
struct MockDriver {
    response_prefix: String,
}

impl MockDriver {
    fn new(response_prefix: &str) -> Self {
        Self {
            response_prefix: response_prefix.to_string(),
        }
    }
}

#[async_trait]
impl BoticelliDriver for MockDriver {
    async fn generate(&self, req: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
        // Extract the last user message (current prompt)
        let last_message = req
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, boticelli::Role::User));

        let response_text = if let Some(msg) = last_message {
            // Extract text from the message
            let texts: Vec<String> = msg
                .content
                .iter()
                .filter_map(|input| {
                    if let boticelli::Input::Text(text) = input {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect();

            format!("{}: {}", self.response_prefix, texts.join(" "))
        } else {
            format!("{}: (no prompt)", self.response_prefix)
        };

        Ok(GenerateResponse {
            outputs: vec![Output::Text(response_text)],
        })
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model-v1"
    }
}

#[tokio::test]
async fn test_execute_simple_narrative() {
    let toml_content = r#"
        [narration]
        name = "test_narrative"
        description = "A simple test narrative"

        [toc]
        order = ["act1", "act2"]

        [acts]
        act1 = "First prompt"
        act2 = "Second prompt"
    "#;

    let narrative: Narrative = toml_content.parse().expect("Failed to parse narrative");
    let driver = MockDriver::new("Response");
    let executor = NarrativeExecutor::new(driver);

    let result = executor
        .execute(&narrative)
        .await
        .expect("Execution failed");

    assert_eq!(result.narrative_name, "test_narrative");
    assert_eq!(result.act_executions.len(), 2);

    // Check first act
    let act1 = &result.act_executions[0];
    assert_eq!(act1.act_name, "act1");
    assert_eq!(get_text_from_inputs(&act1.inputs), "First prompt");
    assert_eq!(act1.response, "Response: First prompt");
    assert_eq!(act1.sequence_number, 0);

    // Check second act
    let act2 = &result.act_executions[1];
    assert_eq!(act2.act_name, "act2");
    assert_eq!(get_text_from_inputs(&act2.inputs), "Second prompt");
    assert_eq!(act2.response, "Response: Second prompt");
    assert_eq!(act2.sequence_number, 1);
}

#[tokio::test]
async fn test_execute_single_act_narrative() {
    let toml_content = r#"
        [narration]
        name = "single_act"
        description = "A narrative with just one act"

        [toc]
        order = ["only_act"]

        [acts]
        only_act = "The only prompt"
    "#;

    let narrative: Narrative = toml_content.parse().expect("Failed to parse narrative");
    let driver = MockDriver::new("Result");
    let executor = NarrativeExecutor::new(driver);

    let result = executor
        .execute(&narrative)
        .await
        .expect("Execution failed");

    assert_eq!(result.narrative_name, "single_act");
    assert_eq!(result.act_executions.len(), 1);

    let act = &result.act_executions[0];
    assert_eq!(act.act_name, "only_act");
    assert_eq!(get_text_from_inputs(&act.inputs), "The only prompt");
    assert_eq!(act.response, "Result: The only prompt");
    assert_eq!(act.sequence_number, 0);
}

#[tokio::test]
async fn test_context_passing_between_acts() {
    // Create a custom mock that records all messages it receives
    struct ContextTrackingDriver {
        call_count: std::sync::Arc<std::sync::Mutex<usize>>,
    }

    #[async_trait]
    impl BoticelliDriver for ContextTrackingDriver {
        async fn generate(&self, req: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            let call_num = *count;

            // Verify that each subsequent call has more messages (conversation history)
            let num_messages = req.messages.len();

            // First call should have 1 message (just the prompt)
            // Second call should have 3 messages (user, assistant, user)
            // Third call should have 5 messages (user, assistant, user, assistant, user)
            let expected_messages = call_num * 2 - 1;
            assert_eq!(
                num_messages, expected_messages,
                "Act {} should have {} messages, but had {}",
                call_num, expected_messages, num_messages
            );

            Ok(GenerateResponse {
                outputs: vec![Output::Text(format!("Response {}", call_num))],
            })
        }

        fn provider_name(&self) -> &'static str {
            "context_tracking"
        }

        fn model_name(&self) -> &str {
            "context-tracker-v1"
        }
    }

    let toml_content = r#"
        [narration]
        name = "context_test"
        description = "Test context passing"

        [toc]
        order = ["act1", "act2", "act3"]

        [acts]
        act1 = "First"
        act2 = "Second"
        act3 = "Third"
    "#;

    let narrative: Narrative = toml_content.parse().expect("Failed to parse narrative");
    let driver = ContextTrackingDriver {
        call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
    };
    let executor = NarrativeExecutor::new(driver);

    let result = executor
        .execute(&narrative)
        .await
        .expect("Execution failed");

    assert_eq!(result.act_executions.len(), 3);
    assert_eq!(result.act_executions[0].response, "Response 1");
    assert_eq!(result.act_executions[1].response, "Response 2");
    assert_eq!(result.act_executions[2].response, "Response 3");
}

#[tokio::test]
async fn test_executor_driver_access() {
    let driver = MockDriver::new("Test");
    let executor = NarrativeExecutor::new(driver);

    // Verify we can access the driver
    assert_eq!(executor.driver().provider_name(), "mock");
    assert_eq!(executor.driver().model_name(), "mock-model-v1");
}

#[tokio::test]
async fn test_trait_abstraction_with_simple_provider() {
    // This test demonstrates that the executor works with any NarrativeProvider,
    // not just TOML-based configurations. This proves the trait abstraction works.

    // Create a simple provider with hardcoded acts (no TOML parsing!)
    struct InMemoryProvider {
        narrative_name: String,
        act_order: Vec<String>,
        act_prompts: std::collections::HashMap<String, String>,
    }

    impl NarrativeProvider for InMemoryProvider {
        fn name(&self) -> &str {
            &self.narrative_name
        }

        fn act_names(&self) -> &[String] {
            &self.act_order
        }

        fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
            self.act_prompts
                .get(act_name)
                .map(|text| ActConfig::from_text(text.clone()))
        }
    }

    let provider = InMemoryProvider {
        narrative_name: "in_memory_test".to_string(),
        act_order: vec!["greeting".to_string(), "farewell".to_string()],
        act_prompts: [
            ("greeting".to_string(), "Say hello".to_string()),
            ("farewell".to_string(), "Say goodbye".to_string()),
        ]
        .into_iter()
        .collect(),
    };

    let driver = MockDriver::new("Mock");
    let executor = NarrativeExecutor::new(driver);

    let result = executor
        .execute(&provider)
        .await
        .expect("Execution failed");

    assert_eq!(result.narrative_name, "in_memory_test");
    assert_eq!(result.act_executions.len(), 2);
    assert_eq!(result.act_executions[0].act_name, "greeting");
    assert_eq!(
        get_text_from_inputs(&result.act_executions[0].inputs),
        "Say hello"
    );
    assert_eq!(result.act_executions[0].response, "Mock: Say hello");
    assert_eq!(result.act_executions[1].act_name, "farewell");
    assert_eq!(
        get_text_from_inputs(&result.act_executions[1].inputs),
        "Say goodbye"
    );
    assert_eq!(result.act_executions[1].response, "Mock: Say goodbye");
}

#[tokio::test]
async fn test_multimodal_and_per_act_config() {
    // This test demonstrates the new flexible configuration capabilities:
    // - Multimodal inputs (text, images, etc.)
    // - Per-act model selection
    // - Per-act temperature/max_tokens overrides

    struct FlexibleProvider {
        name: String,
        acts: Vec<String>,
        configs: std::collections::HashMap<String, ActConfig>,
    }

    impl NarrativeProvider for FlexibleProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn act_names(&self) -> &[String] {
            &self.acts
        }

        fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
            self.configs.get(act_name).cloned()
        }
    }

    // Build a narrative with different configurations per act
    let mut configs = std::collections::HashMap::new();

    // Act 1: Simple text with GPT-4, high temperature for creativity
    configs.insert(
        "creative".to_string(),
        ActConfig::from_text("Write a poem")
            .with_model("gpt-4")
            .with_temperature(0.9)
            .with_max_tokens(100),
    );

    // Act 2: Text only, Claude, lower temperature for analysis
    configs.insert(
        "analytical".to_string(),
        ActConfig::from_text("Analyze the poem")
            .with_model("claude-3-opus-20240229")
            .with_temperature(0.3),
    );

    // Act 3: Multimodal input (text + hypothetical image)
    configs.insert(
        "multimodal".to_string(),
        ActConfig::from_inputs(vec![
            Input::Text("Describe this image in relation to the poem".to_string()),
            Input::Image {
                mime: Some("image/png".to_string()),
                source: boticelli::MediaSource::Url("https://example.com/image.png".to_string()),
            },
        ])
        .with_model("gemini-pro-vision"),
    );

    let provider = FlexibleProvider {
        name: "multimodal_test".to_string(),
        acts: vec![
            "creative".to_string(),
            "analytical".to_string(),
            "multimodal".to_string(),
        ],
        configs,
    };

    let driver = MockDriver::new("Result");
    let executor = NarrativeExecutor::new(driver);

    let result = executor
        .execute(&provider)
        .await
        .expect("Execution failed");

    // Verify act 1 configuration was applied
    assert_eq!(result.act_executions[0].act_name, "creative");
    assert_eq!(result.act_executions[0].model, Some("gpt-4".to_string()));
    assert_eq!(result.act_executions[0].temperature, Some(0.9));
    assert_eq!(result.act_executions[0].max_tokens, Some(100));
    assert_eq!(get_text_from_inputs(&result.act_executions[0].inputs), "Write a poem");

    // Verify act 2 configuration was applied
    assert_eq!(result.act_executions[1].act_name, "analytical");
    assert_eq!(
        result.act_executions[1].model,
        Some("claude-3-opus-20240229".to_string())
    );
    assert_eq!(result.act_executions[1].temperature, Some(0.3));
    assert_eq!(result.act_executions[1].max_tokens, None);

    // Verify act 3 has multimodal inputs
    assert_eq!(result.act_executions[2].act_name, "multimodal");
    assert_eq!(
        result.act_executions[2].model,
        Some("gemini-pro-vision".to_string())
    );
    assert_eq!(result.act_executions[2].inputs.len(), 2);
    // First input is text
    assert!(matches!(&result.act_executions[2].inputs[0], Input::Text(_)));
    // Second input is image
    assert!(matches!(&result.act_executions[2].inputs[1], Input::Image { .. }));
}
