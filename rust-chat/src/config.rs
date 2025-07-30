// src/config.rs

use serde::Deserialize;

// This struct must match the structure of the `[default.app]` table
// in `Rocket.toml`. Rocket will automatically deserialize the configuration
// into this struct.
#[derive(Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
}