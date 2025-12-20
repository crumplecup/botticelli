use botticelli_chat::{ChatAppConfig, ConfigBuilder, EnvironmentMode};
use std::env;

#[test]
fn test_default_config() {
    let config = ChatAppConfig::default();

    assert_eq!(config.environment.mode, EnvironmentMode::Local);
    assert_eq!(config.postgres.host, "localhost");
    assert_eq!(config.postgres.port, 5432);
    assert_eq!(config.mcp_server.host, "localhost");
    assert_eq!(config.mcp_server.port, 3000);
}

#[test]
fn test_container_mode_defaults() {
    let config = ConfigBuilder::default()
        .mode(EnvironmentMode::Container)
        .build()
        .expect("Failed to build config");

    assert_eq!(config.environment.mode, EnvironmentMode::Container);
    assert_eq!(config.postgres.host, "postgres");
    assert_eq!(config.mcp_server.host, "mcp-server");
}

#[test]
fn test_local_mode_defaults() {
    let config = ConfigBuilder::default()
        .mode(EnvironmentMode::Local)
        .build()
        .expect("Failed to build config");

    assert_eq!(config.environment.mode, EnvironmentMode::Local);
    assert_eq!(config.postgres.host, "localhost");
    assert_eq!(config.mcp_server.host, "localhost");
}

#[test]
fn test_builder_overrides() {
    let config = ConfigBuilder::default()
        .mode(EnvironmentMode::Container)
        .postgres_host("custom-postgres")
        .postgres_port(5433)
        .mcp_host("custom-mcp")
        .mcp_port(3001)
        .build()
        .expect("Failed to build config");

    assert_eq!(config.environment.mode, EnvironmentMode::Container);
    assert_eq!(config.postgres.host, "custom-postgres");
    assert_eq!(config.postgres.port, 5433);
    assert_eq!(config.mcp_server.host, "custom-mcp");
    assert_eq!(config.mcp_server.port, 3001);
}

#[test]
fn test_database_url_generation() {
    let config = ChatAppConfig::default();
    let url = config.postgres.database_url();

    assert_eq!(
        url,
        "postgres://botticelli:botticelli@localhost:5432/botticelli"
    );
}

#[test]
fn test_mcp_server_url_generation() {
    let config = ChatAppConfig::default();
    let url = config.mcp_server.server_url();

    assert_eq!(url, "http://localhost:3000");
}

#[test]
fn test_environment_mode_serialization() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct Wrapper {
        mode: EnvironmentMode,
    }

    let local = Wrapper {
        mode: EnvironmentMode::Local,
    };
    let container = Wrapper {
        mode: EnvironmentMode::Container,
    };

    let local_str = toml::to_string(&local).expect("Failed to serialize");
    let container_str = toml::to_string(&container).expect("Failed to serialize");

    assert!(local_str.contains("local"));
    assert!(container_str.contains("container"));
}

#[test]
fn test_environment_from_env_var() {
    unsafe {
        env::set_var("BOTTICELLI__ENVIRONMENT__MODE", "container");
        env::set_var("BOTTICELLI__POSTGRES__HOST", "env-postgres");
    }

    let config = ChatAppConfig::load(None).expect("Failed to load config");

    assert_eq!(config.environment.mode, EnvironmentMode::Container);
    assert_eq!(config.postgres.host, "env-postgres");

    unsafe {
        env::remove_var("BOTTICELLI__ENVIRONMENT__MODE");
        env::remove_var("BOTTICELLI__POSTGRES__HOST");
    }
}

#[test]
fn test_observability_defaults() {
    let config = ChatAppConfig::default();

    assert_eq!(config.observability.rust_log, "info");
    assert_eq!(config.observability.otel_exporter, "stdout");
    assert_eq!(config.observability.otel_endpoint, "http://localhost:4318");
}

#[test]
fn test_mcp_client_defaults() {
    let config = ChatAppConfig::default();

    assert_eq!(config.mcp_client.timeout_seconds, 30);
    assert_eq!(config.mcp_client.retry_attempts, 3);
}
