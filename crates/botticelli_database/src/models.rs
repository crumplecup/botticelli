//! Database models for storing model responses.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use botticelli_core::{GenerateRequest, GenerateResponse, Message, Output};

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
#[derive(Debug, Clone, Insertable, elicitation::Elicit)]
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
    #[tracing::instrument(skip_all, fields(
        message_count = request.messages().len(),
        output_count = response.outputs().len(),
        duration_ms
    ))]
    pub fn new(
        provider: impl Into<String>,
        model_name: impl Into<String>,
        request: &GenerateRequest,
        response: &GenerateResponse,
        duration_ms: Option<i32>,
    ) -> Result<Self, serde_json::Error> {
        tracing::debug!("Creating NewModelResponse from request/response");
        let result = Self {
            provider: provider.into(),
            model_name: model_name.into(),
            request_messages: serde_json::to_value(request.messages())?,
            request_temperature: *request.temperature(),
            request_max_tokens: request.max_tokens().map(|t| t as i32),
            request_model: request.model().clone(),
            response_outputs: serde_json::to_value(response.outputs())?,
            duration_ms,
            error_message: None,
        };
        tracing::debug!("Created NewModelResponse");
        Ok(result)
    }

    /// Create a new error response record.
    #[tracing::instrument(skip_all, fields(
        message_count = request.messages().len(),
        duration_ms
    ))]
    pub fn error(
        provider: impl Into<String>,
        model_name: impl Into<String>,
        request: &GenerateRequest,
        error: impl std::fmt::Display,
        duration_ms: Option<i32>,
    ) -> Result<Self, serde_json::Error> {
        tracing::debug!(error = %error, "Creating error NewModelResponse");
        let result = Self {
            provider: provider.into(),
            model_name: model_name.into(),
            request_messages: serde_json::to_value(request.messages())?,
            request_temperature: *request.temperature(),
            request_max_tokens: request.max_tokens().map(|t| t as i32),
            request_model: request.model().clone(),
            response_outputs: serde_json::json!([]),
            duration_ms,
            error_message: Some(error.to_string()),
        };
        tracing::debug!("Created error NewModelResponse");
        Ok(result)
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
    #[tracing::instrument(skip(self), fields(id = %self.id, has_error = self.error_message.is_some()))]
    pub fn to_serializable(&self) -> Result<SerializableModelResponse, serde_json::Error> {
        tracing::debug!("Converting ModelResponse to serializable format");
        let messages: Vec<Message> = serde_json::from_value(self.request_messages.clone())?;

        let mut request_builder = GenerateRequest::builder().messages(messages);

        if let Some(temp) = self.request_temperature {
            request_builder = request_builder.temperature(temp);
        }
        if let Some(tokens) = self.request_max_tokens {
            request_builder = request_builder.max_tokens(tokens as u32);
        }
        if let Some(ref model) = self.request_model {
            request_builder = request_builder.model(model.clone());
        }

        let request = request_builder
            .build()
            .map_err(|e| serde_json::Error::custom(e.to_string()))?;

        let response = if self.error_message.is_none() {
            let outputs: Vec<Output> = serde_json::from_value(self.response_outputs.clone())?;
            Some(
                GenerateResponse::builder()
                    .outputs(outputs)
                    .build()
                    .map_err(|e| serde_json::Error::custom(e.to_string()))?,
            )
        } else {
            None
        };

        tracing::debug!(id = %self.id, has_response = response.is_some(), "Converted to serializable");
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
