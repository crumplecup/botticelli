//! Input validation for command parameters.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Validation error details.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Reason for failure
    pub reason: String,
}

impl ValidationError {
    /// Create a new validation error.
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

/// Trait for validating command parameters.
pub trait CommandValidator {
    /// Validate command parameters.
    fn validate(&self, command: &str, params: &HashMap<String, String>) -> SecurityResult<()>;
}

/// Discord-specific command validator.
pub struct DiscordValidator;

impl DiscordValidator {
    /// Create a new Discord validator.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new() -> Self {
        Self
    }

    /// Validate a Discord snowflake ID.
    #[cfg_attr(not(test), allow(dead_code))]
    fn validate_snowflake(&self, value: &str) -> bool {
        // Discord snowflakes are 17-19 digit integers
        value.len() >= 17 && value.len() <= 19 && value.chars().all(|c| c.is_ascii_digit())
    }

    /// Validate message content length.
    #[cfg_attr(not(test), allow(dead_code))]
    fn validate_content_length(&self, content: &str) -> bool {
        // Discord message limit is 2000 characters
        !content.is_empty() && content.len() <= 2000
    }

    /// Validate channel name.
    #[cfg_attr(not(test), allow(dead_code))]
    fn validate_channel_name(&self, name: &str) -> bool {
        // Channel names: 1-100 chars, lowercase alphanumeric + hyphens/underscores
        !name.is_empty()
            && name.len() <= 100
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    }

    /// Validate role name.
    #[cfg_attr(not(test), allow(dead_code))]
    fn validate_role_name(&self, name: &str) -> bool {
        // Role names: 1-100 characters
        !name.is_empty() && name.len() <= 100
    }
}

impl Default for DiscordValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandValidator for DiscordValidator {
    #[instrument(skip(self, params), fields(command))]
    fn validate(&self, command: &str, params: &HashMap<String, String>) -> SecurityResult<()> {
        debug!("Validating Discord command parameters");

        match command {
            // Message commands
            "messages.send" | "channels.send_message" => {
                if let Some(channel_id) = params.get("channel_id")
                    && !self.validate_snowflake(channel_id)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "channel_id".to_string(),
                        reason: "Invalid Discord channel ID format".to_string(),
                    }));
                }

                if let Some(content) = params.get("content")
                    && !self.validate_content_length(content)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "content".to_string(),
                        reason: format!(
                            "Content must be 1-2000 characters (got {})",
                            content.len()
                        ),
                    }));
                }
            }

            // Delete commands
            "messages.delete" => {
                if let Some(message_id) = params.get("message_id")
                    && !self.validate_snowflake(message_id)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "message_id".to_string(),
                        reason: "Invalid Discord message ID format".to_string(),
                    }));
                }
            }

            // Channel commands
            "channels.create" => {
                if let Some(name) = params.get("name")
                    && !self.validate_channel_name(name)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "name".to_string(),
                        reason:
                            "Channel name must be 1-100 lowercase alphanumeric chars, hyphens, or underscores"
                                .to_string(),
                    }));
                }
            }

            // Role commands
            "roles.create" => {
                if let Some(name) = params.get("name")
                    && !self.validate_role_name(name)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "name".to_string(),
                        reason: "Role name must be 1-100 characters".to_string(),
                    }));
                }
            }

            // Member moderation
            "members.kick" | "members.ban" => {
                if let Some(user_id) = params.get("user_id")
                    && !self.validate_snowflake(user_id)
                {
                    return Err(SecurityError::new(SecurityErrorKind::ValidationFailed {
                        field: "user_id".to_string(),
                        reason: "Invalid Discord user ID format".to_string(),
                    }));
                }
            }

            _ => {
                debug!("No specific validation rules for command");
            }
        }

        debug!("Validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_snowflake() {
        let validator = DiscordValidator::new();
        assert!(validator.validate_snowflake("123456789012345678")); // 18 digits
        assert!(!validator.validate_snowflake("123")); // Too short
        assert!(!validator.validate_snowflake("abc")); // Not digits
    }

    #[test]
    fn test_validate_content_length() {
        let validator = DiscordValidator::new();
        assert!(validator.validate_content_length("Hello"));
        assert!(!validator.validate_content_length("")); // Empty
        assert!(!validator.validate_content_length(&"x".repeat(2001))); // Too long
    }

    #[test]
    fn test_validate_channel_name() {
        let validator = DiscordValidator::new();
        assert!(validator.validate_channel_name("general"));
        assert!(validator.validate_channel_name("dev-chat"));
        assert!(validator.validate_channel_name("bot_commands"));
        assert!(!validator.validate_channel_name("General")); // Uppercase
        assert!(!validator.validate_channel_name("dev chat")); // Space
        assert!(!validator.validate_channel_name("")); // Empty
    }

    #[test]
    fn test_validate_message_send() {
        let validator = DiscordValidator::new();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello, world!".to_string());

        assert!(validator.validate("messages.send", &params).is_ok());
    }

    #[test]
    fn test_validate_invalid_channel_id() {
        let validator = DiscordValidator::new();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "invalid".to_string());
        params.insert("content".to_string(), "Hello".to_string());

        let result = validator.validate("messages.send", &params);
        assert!(result.is_err());
    }
}
