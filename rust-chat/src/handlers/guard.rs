// src/handlers/guard.rs

use jsonwebtoken::{decode, DecodingKey, Validation};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request, Outcome};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AppConfig;


pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp:i64,
}

#[derive(Debug)]
pub enum GuardError{
    Missing,
    Invalid,
}


#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let auth_header = match req.headers().get_one("Authorization") {
            Some(header) => header,
            None => return Outcome::Error((Status::Unauthorized, GuardError::Missing)),
        };

        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        if parts.len() != 2 || parts[0] != "Bearer" {
            return Outcome::Error((Status::Unauthorized, GuardError::Invalid));
        }
        let token = parts[1];

        // Retrieve the managed AppConfig from the request's Rocket instance.
        // This is the standard way to get managed state within a request guard.
        let config = match req.rocket().state::<AppConfig>() {
            Some(config) => config,
            None => {
                // This would be a server misconfiguration, so a 500 error is appropriate.
                return Outcome::Error((Status::InternalServerError, GuardError::Invalid));
            }
        };

        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt_secret.as_ref()), 
            &Validation::default(),
        ) {
            Ok(data) => data,
            Err(_) => return Outcome::Error((Status::Unauthorized, GuardError::Invalid)),
        };

        match Uuid::parse_str(&token_data.claims.sub) {
            Ok(user_id) => Outcome::Success(AuthenticatedUser { user_id }),
            Err(_) => Outcome::Error((Status::Unauthorized, GuardError::Invalid)),
        }
    }
}