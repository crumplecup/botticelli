//! Social media primitive delegation wrappers.
//!
//! Orchestrator wrappers for social media bot command primitives.

use crate::rmcp_server::BotticelliServer;
use botticelli_error::{BotCommandError, BotCommandResult, SecurityError};
use botticelli_social::{hashmap_to_params, BotCommandRegistryImpl, CommandCache, SecureBotCommandExecutor};
use elicitation::Elicit;
use rmcp::tool;
use rmcp::tool_router;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tracing::instrument;

/// Parameters for converting HashMap to params.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct HashmapToParamsParams {
    /// Arguments as JSON values
    pub args: HashMap<String, JsonValue>,
}

/// Result from hashmap conversion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct HashmapToParamsResult {
    /// Converted params as strings
    pub params: HashMap<String, String>,
}

/// Parameters for converting args to strings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ConvertArgsToStringsParams {
    /// Arguments as JSON values
    pub args: HashMap<String, JsonValue>,
}

/// Result from args conversion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ConvertArgsToStringsResult {
    /// Converted args as strings
    pub args: HashMap<String, String>,
}

/// Parameters for converting security error.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ConvertSecurityErrorParams {
    /// Security error to convert
    pub error: SecurityError,
    /// Command name for context
    pub command_name: String,
}

/// Parameters for creating bot command registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryNewParams {
    /// Placeholder field (unused)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _unused: Option<()>,
}

/// Parameters for creating registry with cache.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryWithCacheParams {
    /// Command cache
    pub cache: CommandCache,
}

/// Parameters for listing platforms.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryPlatformsParams {
    /// Registry instance (placeholder for state)
    pub registry_id: String,
}

/// Result from listing platforms.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryPlatformsResult {
    /// List of platform names
    pub platforms: Vec<String>,
}

/// Parameters for checking platform existence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryHasPlatformParams {
    /// Registry instance (placeholder for state)
    pub registry_id: String,
    /// Platform to check
    pub platform: String,
}

/// Result from platform check.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct BotRegistryHasPlatformResult {
    /// Whether platform exists
    pub exists: bool,
}

#[tool_router(router = social_tool_router, vis = "pub")]
impl BotticelliServer {
    /// Convert HashMap<String, JsonValue> to HashMap<String, String> for security checks.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_hashmap_to_params", arg_count = params.args.len()))]
    pub fn social_hashmap_to_params(&self, params: HashmapToParamsParams) -> BotCommandResult<HashmapToParamsResult> {
        tracing::debug!("Delegating to botticelli_social::hashmap_to_params");
        
        let result = hashmap_to_params(&params.args);
        
        match &result {
            Ok(p) => tracing::debug!(param_count = p.len(), "Conversion succeeded"),
            Err(e) => tracing::error!(error = ?e, "Conversion failed"),
        }
        
        result.map(|params| HashmapToParamsResult { params })
    }

    /// Convert JSON arguments to string arguments for security pipeline.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_convert_args_to_strings", arg_count = params.args.len()))]
    pub fn social_convert_args_to_strings(&self, params: ConvertArgsToStringsParams) -> BotCommandResult<ConvertArgsToStringsResult> {
        tracing::debug!("Delegating to SecureBotCommandExecutor::convert_args_to_strings");
        
        let result = SecureBotCommandExecutor::<botticelli_security::DiscordValidator>::convert_args_to_strings(&params.args);
        
        match &result {
            Ok(args) => tracing::debug!(arg_count = args.len(), "Conversion succeeded"),
            Err(e) => tracing::error!(error = ?e, "Conversion failed"),
        }
        
        result.map(|args| ConvertArgsToStringsResult { args })
    }

    /// Convert security error to bot command error.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_convert_security_error", command = %params.command_name))]
    pub fn social_convert_security_error(&self, params: ConvertSecurityErrorParams) -> BotCommandError {
        tracing::debug!("Delegating to SecureBotCommandExecutor::convert_security_error");
        
        let error = SecureBotCommandExecutor::<botticelli_security::DiscordValidator>::convert_security_error(
            params.error,
            &params.command_name
        );
        
        tracing::debug!("Security error converted");
        error
    }

    /// Create a new empty bot command registry with default cache.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "social_bot_registry_new"))]
    pub fn social_bot_registry_new(&self, _params: BotRegistryNewParams) -> BotCommandRegistryImpl {
        tracing::debug!("Delegating to BotCommandRegistryImpl::new");
        
        let registry = BotCommandRegistryImpl::new();
        
        tracing::debug!("Registry created");
        registry
    }

    /// Create a new bot command registry with custom cache.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_bot_registry_with_cache"))]
    pub fn social_bot_registry_with_cache(&self, params: BotRegistryWithCacheParams) -> BotCommandRegistryImpl {
        tracing::debug!("Delegating to BotCommandRegistryImpl::with_cache");
        
        let registry = BotCommandRegistryImpl::with_cache(params.cache);
        
        tracing::debug!("Registry created with cache");
        registry
    }

    /// List all registered platforms in a bot command registry.
    ///
    /// Note: This is a stateless demonstration. In a real implementation,
    /// you'd need state management to track registry instances.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_bot_registry_platforms", registry_id = %params.registry_id))]
    pub fn social_bot_registry_platforms(&self, params: BotRegistryPlatformsParams) -> BotRegistryPlatformsResult {
        tracing::debug!("Delegating to BotCommandRegistryImpl::platforms");
        tracing::warn!("Stateless wrapper - creating temporary registry");
        
        // Create a temporary registry for demonstration
        let registry = BotCommandRegistryImpl::new();
        let platforms = registry.platforms();
        
        tracing::debug!(platform_count = platforms.len(), "Platforms listed");
        BotRegistryPlatformsResult { platforms }
    }

    /// Check if a platform is registered in a bot command registry.
    ///
    /// Note: This is a stateless demonstration. In a real implementation,
    /// you'd need state management to track registry instances.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "social_bot_registry_has_platform", registry_id = %params.registry_id, platform = %params.platform))]
    pub fn social_bot_registry_has_platform(&self, params: BotRegistryHasPlatformParams) -> BotRegistryHasPlatformResult {
        tracing::debug!("Delegating to BotCommandRegistryImpl::has_platform");
        tracing::warn!("Stateless wrapper - creating temporary registry");
        
        // Create a temporary registry for demonstration
        let registry = BotCommandRegistryImpl::new();
        let exists = registry.has_platform(&params.platform);
        
        tracing::debug!(exists, "Platform existence checked");
        BotRegistryHasPlatformResult { exists }
    }
}
