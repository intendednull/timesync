use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimeError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Database error: {0}")]
    Database(#[from] eyre::Report),
    
    #[error("Internal server error: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type TimeResult<T> = Result<T, TimeError>;