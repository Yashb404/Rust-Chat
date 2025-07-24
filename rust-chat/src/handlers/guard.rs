use jsonwebtoken::{decode, DecodingKey, Validation};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request, Outcome};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;


//this will exist in a handlers signature only if user is authenticated
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

//expected struct of jwt claims payload 
///must be same as claims in auth.rs
#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp:i64,
}

//this represents the reasons for auth failure, it is the error type associated with 'FromRequest' impl
#[derive(Debug)]
pub enum GuardError{
    Missing, //header not found
    Invalid, // token malformed , invalid , expired 
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser{
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>)-> request::Outcome<Self, Self::Error>{
        // Check if the Authorization header is present
        // if not, return an error
        // if it is present, decode the JWT and extract the user ID
        let auth_header = match req.headers().get_one("Authorization"){
        Some(header) => header,
    None => return Outcome::Error((Status::Unauthorized, GuardError::Missing)),
        };
        // Split the header into parts and check if it is a Bearer token which is the format 
        // we expect for JWTs
        // If it is not a Bearer token, return an error
        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        if parts.len() != 2 || parts[0] != "Bearer" {
            return Outcome::Error((Status::Unauthorized, GuardError::Invalid));
        }
        let token = parts[1]; // Extract the token from the header if correct
        //TODO: too look into making this more robust in future than just calling env
        let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env file");
        
        // Decode the token using the secret key
        // If the token is valid, extract the user ID from the claims and return it
        let token_data = match decode::<Claims>(
            token,//deserializes into claims struct
            &DecodingKey::from_secret(secret.as_ref()), //verifies using provided decoding key
            &Validation::default(),//validates standard claims
        ){
            Ok(data) => data,
            Err(_) => return Outcome::Error((Status::Unauthorized, GuardError::Invalid)),     
        };

        //if all decoding successful , validate contents of custom claims 
        match Uuid::parse_str(&token_data.claims.sub){
            Ok(user_id)=>{
                //if pass rocket with inject this AuthenticatedUser into handler
                Outcome::Success(AuthenticatedUser { user_id })
            }
            Err(_) => Outcome::Error((Status::Unauthorized, GuardError::Invalid)),
        }
    }
}