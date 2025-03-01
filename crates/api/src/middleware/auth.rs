//! # Authentication Module
//!
//! This module provides authentication-related utilities for the TimeSync API,
//! including password hashing and verification for schedules.
//!
//! The implementation uses Argon2, a secure password hashing algorithm,
//! to protect user passwords from common attacks like rainbow tables
//! and brute force attempts.

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use eyre::Result;
use uuid::Uuid;

/// Hashes a password using the Argon2 algorithm
///
/// This function securely hashes passwords before storage in the database,
/// automatically generating a random salt and using industry-standard
/// parameters for Argon2.
///
/// # Arguments
///
/// * `password` - The plain text password to hash
///
/// # Returns
///
/// * `Result<String>` - The hashed password string or an error
///
/// # Example
///
/// ```rust
/// let password = "user_password";
/// let hashed = hash_password(password)?;
/// // Store hashed in the database
/// ```
///
/// # Security Notes
///
/// - Uses a random salt for each password
/// - Uses default Argon2 parameters (memory: 19MiB, iterations: 3, parallelism: 4)
/// - Returns password in PHC string format (includes algorithm, version, parameters, salt, and hash)
pub fn hash_password(password: &str) -> Result<String> {
    // Generate a fresh, random salt
    let salt = SaltString::generate(&mut OsRng);
    
    // Create default Argon2 instance
    let argon2 = Argon2::default();
    
    // Hash the password with salt
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| eyre::eyre!("Error hashing password: {}", e))?
        .to_string();
    
    Ok(password_hash)
}

/// Verifies a password against the stored hash for a schedule
///
/// This function checks if the provided password matches the stored hash
/// for the specified schedule, providing a way to authenticate access
/// to password-protected schedules.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `schedule_id` - UUID of the schedule to authenticate
/// * `password` - Plain text password to verify
///
/// # Returns
///
/// * `Result<bool>` - True if password matches, false otherwise
///
/// # Example
///
/// ```rust
/// let schedule_id = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890")?;
/// let is_valid = verify_schedule_password(&pool, schedule_id, "user_provided_password").await?;
/// 
/// if is_valid {
///     // Allow access to the schedule
/// } else {
///     // Deny access
/// }
/// ```
///
/// # Security Notes
///
/// - Uses constant-time comparison to prevent timing attacks
/// - Delegates actual verification to the database layer
pub async fn verify_schedule_password(
    pool: &sqlx::PgPool,
    schedule_id: Uuid,
    password: &str,
) -> Result<bool> {
    // Delegate to the database repository for verification
    let is_valid = timesync_db::repositories::schedule::verify_password(pool, schedule_id, password).await?;
    Ok(is_valid)
}