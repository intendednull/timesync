use std::error::Error;
use timesync_core::errors::{TimeError, TimeResult};

#[test]
fn test_time_error_display() {
    let not_found = TimeError::NotFound("Schedule not found".to_string());
    let validation = TimeError::Validation("Invalid input".to_string());
    let authentication = TimeError::Authentication("Invalid password".to_string());
    let authorization = TimeError::Authorization("Not authorized".to_string());
    let database = TimeError::Database(eyre::eyre!("Database connection failed"));
    let internal = TimeError::Internal(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Internal error",
    )));

    assert_eq!(
        not_found.to_string(),
        "Resource not found: Schedule not found"
    );
    assert_eq!(validation.to_string(), "Validation error: Invalid input");
    assert_eq!(
        authentication.to_string(),
        "Authentication error: Invalid password"
    );
    assert_eq!(
        authorization.to_string(),
        "Authorization error: Not authorized"
    );
    assert!(database.to_string().contains("Database error:"));
    assert!(internal.to_string().contains("Internal server error:"));
}

#[test]
fn test_error_conversion() {
    let io_error = std::io::Error::new(std::io::ErrorKind::Other, "IO error");
    let time_error = TimeError::Internal(Box::new(io_error));

    assert!(time_error.source().is_some());
}

#[test]
fn test_time_result() {
    let result: TimeResult<i32> = Ok(42);
    assert_eq!(result.unwrap(), 42);

    let result: TimeResult<i32> = Err(TimeError::NotFound("Not found".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_from_trait_implementation() {
    let eyre_error = eyre::eyre!("Database error");
    let time_error = TimeError::Database(eyre_error);

    assert!(time_error.to_string().contains("Database error"));
}

#[test]
fn test_box_error_conversion() {
    let io_error = std::io::Error::new(std::io::ErrorKind::Other, "IO error");
    let boxed_error: Box<dyn Error + Send + Sync> = Box::new(io_error);
    let time_error = TimeError::Internal(boxed_error);

    assert!(time_error.to_string().contains("IO error"));
}