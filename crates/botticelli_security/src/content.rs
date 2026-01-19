//! Content filtering for AI-generated output.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use elicitation::{Elicitation, Prompt, Survey};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, instrument};

/// Content filter configuration.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_new::new,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
pub struct ContentFilterConfig {
    /// Maximum content length
    #[serde(default = "default_max_length")]
    #[new(value = "default_max_length()")]
    max_length: usize,

    /// Maximum number of mentions allowed
    #[serde(default = "default_max_mentions")]
    #[new(value = "default_max_mentions()")]
    max_mentions: u32,

    /// Maximum number of URLs allowed
    #[serde(default = "default_max_urls")]
    #[new(value = "default_max_urls()")]
    max_urls: u32,

    /// Prohibited regex patterns
    #[serde(default)]
    #[new(default)]
    prohibited_patterns: Vec<String>,

    /// Allowed URL domains (empty = all allowed)
    #[serde(default)]
    #[new(default)]
    allowed_domains: HashSet<String>,

    /// Denied URL domains (takes precedence)
    #[serde(default)]
    #[new(default)]
    denied_domains: HashSet<String>,

    /// Block @everyone and @here mentions
    #[serde(default = "default_true")]
    #[new(value = "true")]
    block_mass_mentions: bool,
}

fn default_max_length() -> usize {
    2000
}

fn default_max_mentions() -> u32 {
    5
}

fn default_max_urls() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

impl Default for ContentFilterConfig {
    fn default() -> Self {
        Self {
            max_length: default_max_length(),
            max_mentions: default_max_mentions(),
            max_urls: default_max_urls(),
            prohibited_patterns: vec![],
            allowed_domains: HashSet::new(),
            denied_domains: HashSet::new(),
            block_mass_mentions: true,
        }
    }
}

/// Content violation details.
#[derive(Debug, Clone, derive_getters::Getters, derive_new::new, elicitation::Elicit)]
pub struct ContentViolation {
    /// Type of violation
    violation_type: String,
    /// Reason for violation
    reason: String,
}

/// Content filter for validating AI-generated content.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct ContentFilter {
    config: ContentFilterConfig,
    prohibited_regex: Vec<Regex>,
    mention_regex: Regex,
    url_regex: Regex,
}

impl ContentFilter {
    /// Create a new content filter with the given configuration.
    #[tracing::instrument(skip(config), fields(
        pattern_count = config.prohibited_patterns.len(),
        max_length = config.max_length
    ))]
    pub fn new(config: ContentFilterConfig) -> SecurityResult<Self> {
        let mut prohibited_regex = Vec::new();
        for pattern in &config.prohibited_patterns {
            match Regex::new(pattern) {
                Ok(regex) => prohibited_regex.push(regex),
                Err(e) => {
                    tracing::error!(pattern = %pattern, error = %e, "Invalid regex pattern");
                    return Err(SecurityError::new(SecurityErrorKind::Configuration(
                        format!("Invalid regex pattern '{}': {}", pattern, e),
                    )));
                }
            }
        }
        tracing::debug!(
            pattern_count = prohibited_regex.len(),
            "Compiled prohibited patterns"
        );

        // Regex for Discord mentions: <@123456789012345678> or <@!123456789012345678>
        let mention_regex = Regex::new(r"<@!?\d{17,19}>").expect("Valid mention regex");

        // Simple URL regex
        let url_regex = Regex::new(r"https?://[^\s]+").expect("Valid URL regex");

        tracing::debug!("Content filter created successfully");
        Ok(Self {
            config,
            prohibited_regex,
            mention_regex,
            url_regex,
        })
    }

    /// Filter content and return violations if any.
    #[instrument(skip(self, content), fields(content_len = content.len()))]
    pub fn filter(&self, content: &str) -> SecurityResult<()> {
        debug!("Filtering content");

        // Check length
        if content.len() > self.config.max_length {
            debug!("Content exceeds maximum length");
            return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                reason: format!(
                    "Content exceeds maximum length of {} characters (got {})",
                    self.config.max_length,
                    content.len()
                ),
            }));
        }

        // Check for mass mentions
        if self.config.block_mass_mentions
            && (content.contains("@everyone") || content.contains("@here"))
        {
            debug!("Content contains mass mention");
            return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                reason: "Content contains prohibited mass mention (@everyone or @here)".to_string(),
            }));
        }

        // Check mention count
        let mention_count = self.mention_regex.find_iter(content).count() as u32;
        if mention_count > self.config.max_mentions {
            debug!(mention_count, "Content exceeds maximum mentions");
            return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                reason: format!(
                    "Content exceeds maximum mentions of {} (got {})",
                    self.config.max_mentions, mention_count
                ),
            }));
        }

        // Check URL count and domains
        let urls: Vec<_> = self.url_regex.find_iter(content).collect();
        if urls.len() > self.config.max_urls as usize {
            debug!(url_count = urls.len(), "Content exceeds maximum URLs");
            return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                reason: format!(
                    "Content exceeds maximum URLs of {} (got {})",
                    self.config.max_urls,
                    urls.len()
                ),
            }));
        }

        // Check URL domains
        for url in urls {
            let url_str = url.as_str();
            if let Some(domain) = self.extract_domain(url_str) {
                // Check denied domains (takes precedence)
                if self.config.denied_domains.contains(domain) {
                    debug!(domain, "URL domain is denied");
                    return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                        reason: format!("URL domain '{}' is not allowed", domain),
                    }));
                }

                // Check allowed domains (if allowlist is configured)
                if !self.config.allowed_domains.is_empty()
                    && !self.config.allowed_domains.contains(domain)
                {
                    debug!(domain, "URL domain not in allowlist");
                    return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                        reason: format!("URL domain '{}' is not in allowed list", domain),
                    }));
                }
            }
        }

        // Check prohibited patterns
        for (i, regex) in self.prohibited_regex.iter().enumerate() {
            if regex.is_match(content) {
                debug!(pattern_index = i, "Content matches prohibited pattern");
                return Err(SecurityError::new(SecurityErrorKind::ContentViolation {
                    reason: format!(
                        "Content matches prohibited pattern: {}",
                        self.config.prohibited_patterns[i]
                    ),
                }));
            }
        }

        debug!("Content passed all filters");
        Ok(())
    }

    /// Extract domain from URL.
    #[tracing::instrument(skip(self), fields(url))]
    fn extract_domain<'a>(&self, url: &'a str) -> Option<&'a str> {
        // Remove protocol
        let without_protocol = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;

        // Extract domain (before first slash or end of string)
        let domain = without_protocol.split('/').next()?;

        // Remove port if present
        let domain = domain.split(':').next()?;

        Some(domain)
    }
}
