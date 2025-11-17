//! Tests for error-specific retry strategies (Phase 2)

use boticelli::{GeminiError, GeminiErrorKind, RetryableError};

#[test]
fn test_error_classification() {
    // Transient errors
    let err_503 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 503,
        message: "Service unavailable".to_string(),
    });
    assert!(err_503.kind.is_retryable());

    let err_429 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 429,
        message: "Too many requests".to_string(),
    });
    assert!(err_429.kind.is_retryable());

    let err_500 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 500,
        message: "Internal server error".to_string(),
    });
    assert!(err_500.kind.is_retryable());

    // Permanent errors
    let err_401 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 401,
        message: "Unauthorized".to_string(),
    });
    assert!(!err_401.kind.is_retryable());

    let err_400 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 400,
        message: "Bad request".to_string(),
    });
    assert!(!err_400.kind.is_retryable());

    let err_404 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 404,
        message: "Not found".to_string(),
    });
    assert!(!err_404.kind.is_retryable());
}

#[test]
fn test_error_specific_retry_strategies() {
    // 429 (Rate Limit): Longer initial delay, fewer retries
    let err_429 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 429,
        message: "Too many requests".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_429.kind.retry_strategy_params();
    assert_eq!(initial_ms, 5000, "429 should start with 5s delay");
    assert_eq!(max_retries, 3, "429 should have 3 max retries");
    assert_eq!(max_delay_secs, 40, "429 should cap at 40s");

    // 503 (Server Overload): Standard delay, more patient
    let err_503 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 503,
        message: "Service unavailable".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_503.kind.retry_strategy_params();
    assert_eq!(initial_ms, 2000, "503 should start with 2s delay");
    assert_eq!(max_retries, 5, "503 should have 5 max retries");
    assert_eq!(max_delay_secs, 60, "503 should cap at 60s");

    // 500 (Server Error): Quick retries, fail fast
    let err_500 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 500,
        message: "Internal server error".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_500.kind.retry_strategy_params();
    assert_eq!(initial_ms, 1000, "500 should start with 1s delay");
    assert_eq!(max_retries, 3, "500 should have 3 max retries");
    assert_eq!(max_delay_secs, 8, "500 should cap at 8s");

    // 502 (Bad Gateway): Same as 500
    let err_502 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 502,
        message: "Bad gateway".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_502.kind.retry_strategy_params();
    assert_eq!(initial_ms, 1000, "502 should start with 1s delay");
    assert_eq!(max_retries, 3, "502 should have 3 max retries");
    assert_eq!(max_delay_secs, 8, "502 should cap at 8s");

    // 504 (Gateway Timeout): Same as 500
    let err_504 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 504,
        message: "Gateway timeout".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_504.kind.retry_strategy_params();
    assert_eq!(initial_ms, 1000, "504 should start with 1s delay");
    assert_eq!(max_retries, 3, "504 should have 3 max retries");
    assert_eq!(max_delay_secs, 8, "504 should cap at 8s");

    // 408 (Request Timeout): Moderate strategy
    let err_408 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 408,
        message: "Request timeout".to_string(),
    });
    let (initial_ms, max_retries, max_delay_secs) = err_408.kind.retry_strategy_params();
    assert_eq!(initial_ms, 2000, "408 should start with 2s delay");
    assert_eq!(max_retries, 4, "408 should have 4 max retries");
    assert_eq!(max_delay_secs, 30, "408 should cap at 30s");
}

#[test]
fn test_websocket_error_strategies() {
    let ws_conn_err = GeminiError::new(GeminiErrorKind::WebSocketConnection(
        "Connection failed".to_string(),
    ));
    assert!(ws_conn_err.kind.is_retryable());
    let (initial_ms, max_retries, max_delay_secs) = ws_conn_err.kind.retry_strategy_params();
    assert_eq!(initial_ms, 2000, "WebSocket connection should use standard delay");
    assert_eq!(max_retries, 5, "WebSocket connection should have 5 max retries");
    assert_eq!(max_delay_secs, 60, "WebSocket connection should cap at 60s");

    let ws_handshake_err = GeminiError::new(GeminiErrorKind::WebSocketHandshake(
        "Handshake failed".to_string(),
    ));
    assert!(ws_handshake_err.kind.is_retryable());
    let (initial_ms, max_retries, max_delay_secs) = ws_handshake_err.kind.retry_strategy_params();
    assert_eq!(initial_ms, 2000, "WebSocket handshake should use standard delay");
    assert_eq!(max_retries, 5, "WebSocket handshake should have 5 max retries");
    assert_eq!(max_delay_secs, 60, "WebSocket handshake should cap at 60s");

    let stream_err = GeminiError::new(GeminiErrorKind::StreamInterrupted(
        "Stream interrupted".to_string(),
    ));
    assert!(stream_err.kind.is_retryable());
    let (initial_ms, max_retries, max_delay_secs) = stream_err.kind.retry_strategy_params();
    assert_eq!(initial_ms, 1000, "Stream should use quick retry");
    assert_eq!(max_retries, 3, "Stream should have 3 max retries");
    assert_eq!(max_delay_secs, 10, "Stream should cap at 10s");
}

#[test]
fn test_retryable_error_trait_delegation() {
    let err_503 = GeminiError::new(GeminiErrorKind::HttpError {
        status_code: 503,
        message: "Service unavailable".to_string(),
    });

    // Verify trait methods delegate correctly
    assert!(err_503.is_retryable());
    let params = err_503.retry_strategy_params();
    assert_eq!(params, (2000, 5, 60));
}
