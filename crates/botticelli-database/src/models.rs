//! Database models for storing model responses.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GenerateRequest, GenerateResponse};

use super::schema::model_responses;

/// A stored model response in the database.
#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Identifiable)]
#[diesel(table_name = model_responses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ModelResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub provider: String,
    pub model_name: String,
    pub request_messages: serde_json::Value,
    pub request_temperature: Option<f32>,
    pub request_max_tokens: Option<i32>,
    pub request_model: Option<String>,
    pub response_outputs: serde_json::Value,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

/// New model response for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = model_responses)]
pub struct NewModelResponse {
    pub provider: String,
    pub model_name: String,
    pub request_messages: serde_json::Value,
    pub request_temperature: Option<f32>,
    pub request_max_tokens: Option<i32>,
    pub request_model: Option<String>,
    pub response_outputs: serde_json::Value,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

impl NewModelResponse {
    /// Create a new model response record from a request and response.
    pub fn new(
        provider: impl Into<String>,
        model_name: impl Into<String>,
        request: &GenerateRequest,
        response: &GenerateResponse,
        duration_ms: Option<i32>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            provider: provider.into(),
            model_name: model_name.into(),
            request_messages: serde_json::to_value(&request.messages)?,
            request_temperature: request.temperature,
            request_max_tokens: request.max_tokens.map(|t| t as i32),
            request_model: request.model.clone(),
            response_outputs: serde_json::to_value(&response.outputs)?,
            duration_ms,
            error_message: None,
        })
    }

    /// Create a new error response record.
    pub fn error(
        provider: impl Into<String>,
        model_name: impl Into<String>,
        request: &GenerateRequest,
        error: impl std::fmt::Display,
        duration_ms: Option<i32>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            provider: provider.into(),
            model_name: model_name.into(),
            request_messages: serde_json::to_value(&request.messages)?,
            request_temperature: request.temperature,
            request_max_tokens: request.max_tokens.map(|t| t as i32),
            request_model: request.model.clone(),
            response_outputs: serde_json::json!([]),
            duration_ms,
            error_message: Some(error.to_string()),
        })
    }
}

/// Serializable version of ModelResponse for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableModelResponse {
    pub id: String,
    pub created_at: String,
    pub provider: String,
    pub model_name: String,
    pub request: GenerateRequest,
    pub response: Option<GenerateResponse>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

impl ModelResponse {
    /// Convert to a serializable format.
    pub fn to_serializable(&self) -> Result<SerializableModelResponse, serde_json::Error> {
        let request = GenerateRequest {
            messages: serde_json::from_value(self.request_messages.clone())?,
            temperature: self.request_temperature,
            max_tokens: self.request_max_tokens.map(|t| t as u32),
            model: self.request_model.clone(),
        };

        let response = if self.error_message.is_none() {
            Some(GenerateResponse {
                outputs: serde_json::from_value(self.response_outputs.clone())?,
            })
        } else {
            None
        };

        Ok(SerializableModelResponse {
            id: self.id.to_string(),
            created_at: self.created_at.to_string(),
            provider: self.provider.clone(),
            model_name: self.model_name.clone(),
            request,
            response,
            duration_ms: self.duration_ms,
            error_message: self.error_message.clone(),
        })
    }
}
