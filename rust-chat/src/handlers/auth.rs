// src/handlers/auth.rs

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rocket::{
    http::Status, post, request::Request, response::{self, Responder, Response}, serde::json::Json, State
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
// Import our new config struct
use crate::config::AppConfig;

// ... (AuthPayload, AuthResponse, ErrorResponse structs remain the same) ...

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
}
#[derive(Deserialize)]
pub struct AuthPayload {
    username: String,
    password: String,
}
#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
}
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Username already exists")]
    UsernameExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Server error")]
    ServerError,
    #[error("Could not create token")]
    TokenCreationError,
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for AuthError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let error_message = self.to_string();
        let status = match self {
            AuthError::UsernameExists => Status::Conflict,
            AuthError::InvalidCredentials => Status::Unauthorized,
            AuthError::ServerError => Status::InternalServerError,
            AuthError::TokenCreationError => Status::InternalServerError,
        };

        let json = Json(ErrorResponse {
            error: error_message,
        });

        Response::build()
            .status(status)
            .merge(json.respond_to(req)?)
            .ok()
    }
}
impl From<sqlx::Error> for AuthError {
    fn from(_: sqlx::Error) -> Self { AuthError::ServerError }
}
impl From<argon2::password_hash::Error> for AuthError {
    fn from(_: argon2::password_hash::Error) -> Self { AuthError::InvalidCredentials }
}
impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        AuthError::TokenCreationError
    }
}


// The function now takes a reference to the AppConfig.
fn create_token(user_id: Uuid, config: &AppConfig) -> Result<String, AuthError> {
    // The secret is retrieved from the config struct, not from the environment.
    let secret = &config.jwt_secret;
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("Failed to set expiration time")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).map_err(Into::into)
}

#[post("/register", format = "json", data = "<payload>")]
pub async fn register(
    pool: &State<PgPool>,
    payload: Json<AuthPayload>,
    // The AppConfig is now injected by Rocket as managed state.
    config: &State<AppConfig>,
) -> Result<Json<AuthResponse>, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    let user_id = sqlx::query!(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2) ON CONFLICT (username) DO NOTHING RETURNING id",
        payload.username,
        password_hash
    )
    .fetch_optional(pool.inner())
    .await?
    .ok_or(AuthError::UsernameExists)?
    .id;

    // We pass the config to the create_token function.
    let token = create_token(user_id, config.inner())?;
    Ok(Json(AuthResponse { token }))
}

#[post("/login", format = "json", data = "<payload>")]
pub async fn login(
    pool: &State<PgPool>,
    payload: Json<AuthPayload>,
    // The AppConfig is also injected here.
    config: &State<AppConfig>,
) -> Result<Json<AuthResponse>, AuthError> {
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(pool.inner())
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    let parsed_hash = PasswordHash::new(&user.password_hash)?;
    Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash)?;
    
    // Pass the config to the create_token function.
    let token = create_token(user.id, config.inner())?;
    Ok(Json(AuthResponse { token }))
}