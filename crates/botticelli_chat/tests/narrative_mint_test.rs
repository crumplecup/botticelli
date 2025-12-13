use botticelli_chat::{
    ChatAppConfig, Command, CommandExecutor, EnvironmentMode, NarrativeCommand, ServiceContainer,
};
use std::sync::Arc;

#[tokio::test]
async fn test_create_mint_narrative() -> Result<(), Box<dyn std::error::Error>> {
    // Load local configuration
    let config = ChatAppConfig::load(None)?;
    assert_eq!(config.environment.mode, EnvironmentMode::Local);
    
    // Create services and executor
    let services = Arc::new(ServiceContainer::new(config));
    let executor = CommandExecutor::with_services(services);
    
    // The actual prompt from the user
    let prompt = "Create a narrative to generate social media posts for the MINT navigation center in Grants Pass, Oregon.".to_string();
    
    // Execute create narrative command
    let command = Command::Narrative(NarrativeCommand::Create { prompt });
    let response = executor.execute(command).await?;
    
    // Verify we got a response
    let response_text = response.as_text();
    assert!(!response_text.is_empty(), "Response should not be empty");
    
    println!("✅ Response:\n{}", response_text);
    
    // Verify key elements are mentioned
    let lower = response_text.to_lowercase();
    assert!(
        lower.contains("mint") || lower.contains("navigation"),
        "Response should mention MINT or navigation center"
    );
    
    Ok(())
}
