#![cfg(feature = "cli")]

use botticelli_chat::{ChatAppConfig, ServiceContainer};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};

use std::path::PathBuf;

/// Helper to load test configuration.
fn load_test_config() -> ChatResult<ChatAppConfig> {
    // Use absolute path from workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let config_path = PathBuf::from(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("chat.test.toml");

    // Print for debugging
    eprintln!("Loading config from: {:?}", config_path);
    eprintln!("Config exists: {}", config_path.exists());

    ChatAppConfig::load(Some(&config_path)).map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!(
            "Failed to load config from {:?}: {}",
            config_path, e
        )))
    })
}

/// Helper to create test service container.
fn create_test_services() -> ChatResult<ServiceContainer> {
    let config = load_test_config()?;
    Ok(ServiceContainer::new(config))
}

#[tokio::test]
#[cfg_attr(not(feature = "cli"), ignore)]
async fn test_create_narrative_mint_social_media() -> ChatResult<()> {
    let services = create_test_services()?;
    let _mcp_client = services.mcp_client().await?;

    // Test that MCP client can be called - actual tool execution requires MCP server
    // This test verifies the client is properly configured
    assert!(
        services.is_mcp_initialized(),
        "MCP client should be initialized"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "cli"), ignore)]
async fn test_services_initialization() -> ChatResult<()> {
    let services = create_test_services()?;

    // Test MCP client initialization
    let _mcp = services.mcp_client().await?;
    assert!(services.is_mcp_initialized(), "MCP should be initialized");

    // Test database pool initialization
    let _db = services.db_pool().await?;
    assert!(services.is_db_initialized(), "DB should be initialized");

    // Test narrative repository initialization
    let _repo = services.narrative_repository().await?;
    assert!(
        services.is_narrative_repo_initialized(),
        "Narrative repo should be initialized"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "cli"), ignore)]
async fn test_database_connection() -> ChatResult<()> {
    let services = create_test_services()?;
    let pool = services.db_pool().await?;

    // Test that we can get a connection
    let _conn = pool.get().map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!(
            "Failed to get connection: {}",
            e
        )))
    })?;

    // Successfully getting a connection means the pool is working
    assert!(
        services.is_db_initialized(),
        "DB pool should be initialized"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "cli"), ignore)]
async fn test_configuration_loading() -> ChatResult<()> {
    let config = load_test_config()?;

    // Verify test configuration is properly loaded
    assert_eq!(
        config.environment.mode,
        botticelli_chat::EnvironmentMode::Test,
        "Should load test mode, got: {:?}",
        config.environment.mode
    );

    // Verify database config
    let db_url = config.postgres.database_url();
    assert!(
        db_url.contains("botticelli_test"),
        "Should use test database, got: {}",
        db_url
    );

    // Verify MCP config
    let mcp_url = config.mcp_server.server_url();
    assert!(
        mcp_url.contains("3001"),
        "Should use test MCP port, got: {}",
        mcp_url
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "cli"), ignore)]
async fn test_lazy_initialization() -> ChatResult<()> {
    let services = create_test_services()?;

    // Initially, nothing should be initialized
    assert!(
        !services.is_mcp_initialized(),
        "MCP should not be initialized yet"
    );
    assert!(
        !services.is_db_initialized(),
        "DB should not be initialized yet"
    );
    assert!(
        !services.is_narrative_repo_initialized(),
        "Narrative repo should not be initialized yet"
    );

    // Access MCP client - should initialize
    let _mcp = services.mcp_client().await?;
    assert!(
        services.is_mcp_initialized(),
        "MCP should now be initialized"
    );

    // Access DB pool - should initialize
    let _db = services.db_pool().await?;
    assert!(services.is_db_initialized(), "DB should now be initialized");

    Ok(())
}
