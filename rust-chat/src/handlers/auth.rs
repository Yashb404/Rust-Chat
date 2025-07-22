// src/handlers/auth.rs

// --- New and Updated Imports ---
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use rocket::{post, serde::json::Json, response::status, State};
use sqlx::PgPool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
}

// We've added a specific error for password hash parsing/verification
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Username already exists")]
    UsernameExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Internal server error")]
    ServerError,
}

// This allows us to convert different error types into our custom error response
impl From<sqlx::Error> for AuthError {
    fn from(_: sqlx::Error) -> Self { AuthError::ServerError }
}
impl From<argon2::password_hash::Error> for AuthError {
    fn from(_: argon2::password_hash::Error) -> Self { AuthError::InvalidCredentials }
}


// --- Refactored Registration Endpoint ---
#[post("/register", format = "json", data = "<payload>")]
pub async fn register(
    pool: &State<PgPool>,
    payload: Json<AuthPayload>,
) -> Result<Json<AuthResponse>, AuthError> {
    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password with the salt
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    // Insert user into the database
    let user_id = sqlx::query!(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2) ON CONFLICT (username) DO NOTHING RETURNING id",
        payload.username,
        password_hash
    )
    .fetch_optional(pool.inner())
    .await?
    .ok_or(AuthError::UsernameExists)?
    .id;

    // TODO: Replace with a real JWT
    let token = format!("placeholder_token_for_{}", user_id);
    Ok(Json(AuthResponse { token }))
}


// --- New Login Endpoint ---
#[post("/login", format = "json", data = "<payload>")]
pub async fn login(
    pool: &State<PgPool>,
    payload: Json<AuthPayload>,
) -> Result<Json<AuthResponse>, AuthError> {
    // 1. Fetch user from the database by username
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(pool.inner())
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    // 2. Parse the stored password hash
    let parsed_hash = PasswordHash::new(&user.password_hash)?;

    // 3. Verify the password against the hash
    Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash)?;

    // TODO: Replace with a real JWT
    let token = format!("placeholder_token_for_{}", user.id);
    Ok(Json(AuthResponse { token }))
}