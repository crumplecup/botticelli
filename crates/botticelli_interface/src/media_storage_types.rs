//! Wrapper types for MediaStorage trait tool generation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for store method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreParams<M, D> {
    /// Binary data to store
    pub data: D,
    /// Metadata about the media
    pub metadata: M,
}

/// Result from store method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreResult<R> {
    /// Storage reference
    pub reference: R,
}

/// Parameters for retrieve method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveParams<R> {
    /// Storage reference
    pub reference: R,
}

/// Result from retrieve method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveResult {
    /// Binary data
    pub data: Vec<u8>,
}

/// Parameters for get_url method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetUrlParams<R> {
    /// Storage reference
    pub reference: R,
    /// Expiry duration in seconds
    pub expires_in_secs: u64,
}

/// Result from get_url method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetUrlResult {
    /// Public URL (if supported)
    pub url: Option<String>,
}

/// Parameters for delete method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteParams<R> {
    /// Storage reference
    pub reference: R,
}

/// Result from delete method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteResult {
    /// Success status
    pub success: bool,
}

/// Parameters for exists method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExistsParams<R> {
    /// Storage reference
    pub reference: R,
}

/// Result from exists method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExistsResult {
    /// Whether media exists
    pub exists: bool,
}
