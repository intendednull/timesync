//! # Error Handling Middleware
//!
//! This module provides a standardized way to handle errors in the TimeSync API.
//! It maps domain-specific errors to appropriate HTTP status codes and JSON
//! error responses, ensuring a consistent error handling experience across
//! the entire API.
//!
//! The implementation is based on Axum's error handling mechanisms and integrates
//! with TimeSync's custom error types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use timesync_core::errors::TimeError;

/// Application error wrapper that provides HTTP status code mapping
///
/// `AppError` wraps domain-specific `TimeError` instances and implements
/// `IntoResponse` to convert them into HTTP responses with appropriate
/// status codes and JSON payloads.
///
/// # Example
///
/// ```rust
/// async fn handler() -> Result<Json<ScheduleResponse>, AppError> {
///     let schedule = repository.get_schedule(id)
///         .await
///         .map_err(|e| AppError(TimeError::NotFound(e.to_string())))?;
///     
///     Ok(Json(schedule.into()))
/// }
/// ```
#[derive(Debug)]
pub struct AppError(pub TimeError);

/// Converts application errors to HTTP responses
///
/// This implementation maps each error type to the appropriate HTTP status code
/// and formats the error message into a JSON response body.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Map error types to HTTP status codes
        let status = match &self.0 {
            TimeError::NotFound(_) => StatusCode::NOT_FOUND,
            TimeError::Validation(_) => StatusCode::BAD_REQUEST,
            TimeError::Authentication(_) => StatusCode::UNAUTHORIZED,
            TimeError::Authorization(_) => StatusCode::FORBIDDEN,
            TimeError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TimeError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
    
        // Get the error message and format as JSON
        let message = self.0.to_string();
        let body = Json(json!({ "error": message }));
    
        // Combine status code and JSON body into a response
        (status, body).into_response()
    }
}

/// Automatic conversion from TimeError to AppError
///
/// This implementation allows using `?` operator with functions that return
/// `Result<T, TimeError>` in handler functions that return `Result<T, AppError>`.
impl From<TimeError> for AppError {
    fn from(err: TimeError) -> Self {
        AppError(err)
    }
}

/// Automatic conversion from eyre::Report to AppError
///
/// This implementation allows using `?` operator with functions that return
/// `Result<T, eyre::Report>` in handler functions that return `Result<T, AppError>`.
/// It wraps the eyre error in a TimeError::Database variant.
impl From<eyre::Report> for AppError {
    fn from(err: eyre::Report) -> Self {
        AppError(TimeError::Database(err))
    }
}

/// Maps a TimeError to an HTTP response
///
/// This function is provided for backwards compatibility with code
/// that directly uses the error mapping function.
///
/// # Arguments
///
/// * `err` - The TimeError to convert
///
/// # Returns
///
/// * `Response` - An HTTP response with appropriate status code and body
///
/// # Example
///
/// ```rust
/// async fn legacy_handler() -> impl IntoResponse {
///     match repository.get_schedule(id).await {
///         Ok(schedule) => (StatusCode::OK, Json(schedule)),
///         Err(err) => map_error(TimeError::NotFound(err.to_string())),
///     }
/// }
/// ```
pub fn map_error(err: TimeError) -> Response {
    AppError(err).into_response()
}