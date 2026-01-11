//! Comprehensive tests for botticelli_security crate.

mod helpers;

use botticelli_security::*;
use std::collections::HashMap;

// ============================================================================
// Rate Limiter Tests
// ============================================================================

#[test]
fn test_rate_limit_allows_multiple() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing rate limiter allows multiple requests");
    
    let mut limiter = RateLimiter::new();
    let limit = RateLimit::strict(5, 60);
    limiter.add_limit("test.operation", limit);
    tracing::debug!("Configured rate limit: 5 per 60s");

    // First 5 should pass
    for i in 0..5 {
        assert!(limiter.check("test.operation").is_ok());
        tracing::debug!(request_num = i + 1, "Request allowed");
    }
    
    tracing::info!("Rate limit allows multiple test passed");
    Ok(())
}

#[test]
fn test_rate_limit_blocks_excess() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing rate limiter blocks excess requests");
    
    let mut limiter = RateLimiter::new();
    let limit = RateLimit::strict(2, 60);
    limiter.add_limit("test.operation", limit);
    tracing::debug!("Configured rate limit: 2 per 60s");

    // First 2 should pass
    limiter.check("test.operation")?;
    limiter.check("test.operation")?;
    tracing::debug!("First 2 requests passed");

    // Third should fail
    assert!(limiter.check("test.operation").is_err());
    tracing::debug!("Third request blocked as expected");
    
    tracing::info!("Rate limit blocks excess test passed");
    Ok(())
}

#[test]
fn test_rate_limit_with_burst() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing rate limit with burst capacity");
    
    let mut limiter = RateLimiter::new();
    let limit = RateLimit::new(5, 60, 2); // 5 per minute + 2 burst
    limiter.add_limit("test.operation", limit);
    tracing::debug!("Configured rate limit: 5 per 60s + 2 burst");

    // Should allow 7 requests (5 + 2 burst)
    for i in 0..7 {
        assert!(limiter.check("test.operation").is_ok());
        tracing::debug!(request_num = i + 1, "Request allowed (including burst)");
    }

    // 8th should fail
    assert!(limiter.check("test.operation").is_err());
    tracing::debug!("8th request blocked, burst exhausted");
    
    tracing::info!("Rate limit with burst test passed");
    Ok(())
}

#[test]
fn test_rate_limit_unconfigured_allows() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing unconfigured operation allows all requests");
    
    let mut limiter = RateLimiter::new();

    // No limit configured, should always pass
    assert!(limiter.check("unconfigured.operation").is_ok());
    assert!(limiter.check("unconfigured.operation").is_ok());
    tracing::debug!("Both requests allowed for unconfigured operation");
    
    tracing::info!("Unconfigured operation allows test passed");
    Ok(())
}

#[test]
fn test_rate_limit_available_tokens() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing available tokens tracking");
    
    let mut limiter = RateLimiter::new();
    let limit = RateLimit::strict(5, 60);
    limiter.add_limit("test.operation", limit);

    // Should have 5 tokens initially
    assert_eq!(limiter.available_tokens("test.operation"), Some(5));
    tracing::debug!(available = 5, "Initial token count");

    // Consume one
    limiter.check("test.operation")?;
    assert_eq!(limiter.available_tokens("test.operation"), Some(4));
    tracing::debug!(available = 4, "Token count after consumption");
    
    tracing::info!("Available tokens tracking test passed");
    Ok(())
}

// ============================================================================
// Content Filter Tests
// ============================================================================

#[test]
fn test_content_filter_clean_text() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing content filter allows clean text");
    
    let config = ContentFilterConfig::default();
    let filter = ContentFilter::new(config)?;
    assert!(filter.filter("This is clean text").is_ok());
    tracing::debug!("Clean text passed filter");
    
    tracing::info!("Content filter clean text test passed");
    Ok(())
}

#[test]
fn test_content_filter_max_length() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing content filter max length enforcement");
    
    let config = ContentFilterConfig::new().with_max_length(10);
    let filter = ContentFilter::new(config)?;
    tracing::debug!(max_length = 10, "Configured content filter");

    assert!(filter.filter("Short").is_ok());
    tracing::debug!("Short content passed");
    
    assert!(filter.filter("This is way too long for the limit").is_err());
    tracing::debug!("Long content blocked");
    
    tracing::info!("Content filter max length test passed");
    Ok(())
}

#[test]
fn test_content_filter_custom_pattern() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing content filter custom prohibited patterns");
    
    let patterns = vec!["\\b(secret|password)\\b".to_string()];
    let config = ContentFilterConfig::new().with_prohibited_patterns(patterns);
    let filter = ContentFilter::new(config)?;
    tracing::debug!("Configured prohibited patterns: secret, password");

    assert!(filter.filter("Safe content here").is_ok());
    tracing::debug!("Safe content passed");
    
    assert!(filter.filter("The secret code is 123").is_err());
    tracing::debug!("Content with 'secret' blocked");
    
    assert!(filter.filter("My password is hunter2").is_err());
    tracing::debug!("Content with 'password' blocked");
    
    tracing::info!("Content filter custom pattern test passed");
    Ok(())
}

#[test]
fn test_content_filter_mass_mentions() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing content filter blocks mass mentions");
    
    let config = ContentFilterConfig::default();
    let filter = ContentFilter::new(config)?;

    // @everyone and @here should be blocked
    assert!(filter.filter("Hey @everyone look at this!").is_err());
    tracing::debug!("@everyone mention blocked");
    
    assert!(filter.filter("Attention @here please").is_err());
    tracing::debug!("@here mention blocked");
    
    tracing::info!("Content filter mass mentions test passed");
    Ok(())
}

// ============================================================================
// Permission Checker Tests
// ============================================================================

#[test]
fn test_permission_default_deny() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing permission checker default deny");
    
    let config = PermissionConfig::default();
    let checker = PermissionChecker::new(config);
    let result = checker.check_command("test.command");
    assert!(result.is_err());
    tracing::debug!("Default deny enforced");
    
    tracing::info!("Permission default deny test passed");
    Ok(())
}

#[test]
fn test_permission_allow_by_default() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing permission checker allow by default");
    
    let config = PermissionConfig::new().with_allow_all_by_default(true);
    let checker = PermissionChecker::new(config);
    let result = checker.check_command("test.command");
    assert!(result.is_ok());
    tracing::debug!("Allow by default enabled");
    
    tracing::info!("Permission allow by default test passed");
    Ok(())
}

// ============================================================================
// Approval Workflow Tests
// ============================================================================

#[test]
fn test_approval_create_action() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing approval workflow creates pending action");
    
    let mut workflow = ApprovalWorkflow::new();
    let params = HashMap::new();

    let action_id = workflow
        .create_pending_action("narrative1", "test.command", params, None)?;
    tracing::debug!(action_id = %action_id, "Created pending action");

    assert!(workflow.get_pending_action(&action_id).is_some());
    tracing::debug!("Verified action exists");
    
    tracing::info!("Approval create action test passed");
    Ok(())
}

#[test]
fn test_approval_approve_action() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing approval workflow approves action");
    
    let mut workflow = ApprovalWorkflow::new();
    let params = HashMap::new();

    let action_id = workflow
        .create_pending_action("narrative1", "test.command", params, None)?;
    tracing::debug!(action_id = %action_id, "Created pending action");

    workflow
        .approve_action(&action_id, "admin", Some("Looks good".to_string()))?;
    tracing::debug!("Action approved");

    assert!(workflow.check_approval(&action_id).is_ok());
    tracing::debug!("Verified action is approved");
    
    tracing::info!("Approval approve action test passed");
    Ok(())
}

#[test]
fn test_approval_deny_action() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing approval workflow denies action");
    
    let mut workflow = ApprovalWorkflow::new();
    let params = HashMap::new();

    let action_id = workflow
        .create_pending_action("narrative1", "test.command", params, None)?;
    tracing::debug!(action_id = %action_id, "Created pending action");

    workflow
        .deny_action(&action_id, "admin", Some("Not allowed".to_string()))?;
    tracing::debug!("Action denied");

    assert!(workflow.check_approval(&action_id).is_err());
    tracing::debug!("Verified action is denied");
    
    tracing::info!("Approval deny action test passed");
    Ok(())
}

#[test]
fn test_approval_pending_blocks() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing approval workflow blocks pending actions");
    
    let mut workflow = ApprovalWorkflow::new();
    let params = HashMap::new();

    let action_id = workflow
        .create_pending_action("narrative1", "test.command", params, None)?;
    tracing::debug!(action_id = %action_id, "Created pending action");

    // Should block while pending
    assert!(workflow.check_approval(&action_id).is_err());
    tracing::debug!("Verified pending action is blocked");
    
    tracing::info!("Approval pending blocks test passed");
    Ok(())
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn test_validation_error_creation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing validation error creation");
    
    let error = ValidationError::new("test_field", "Invalid value");
    assert_eq!(error.field, "test_field");
    assert_eq!(error.reason, "Invalid value");
    tracing::debug!(field = %error.field, reason = %error.reason, "Validation error created");
    
    tracing::info!("Validation error creation test passed");
    Ok(())
}
