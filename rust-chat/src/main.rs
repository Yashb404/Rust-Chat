// src/main.rs

use rocket::{routes, Build, Rocket, fs::FileServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use log::info;

// Import the new config and the state
use crate::config::AppConfig;
use crate::state::ChatServerState;

// Import all handlers
use crate::handlers::{auth, chat};


// Declare all modules
mod models;
mod state;
mod config;
mod handlers;
mod websocket;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    env_logger::init();
    info!("Starting chat server...");

    rocket::build()
        // A "fairing" is a piece of middleware that hooks into Rocket's launch
        // process. `on_ignite` runs after Rocket has loaded its configuration
        // but before it starts accepting requests.
        .attach(rocket::fairing::AdHoc::on_ignite("Application Setup", |rocket| async {
            // 1. Extract the custom `AppConfig` from Rocket's configuration.
            //    Figment has already loaded `Rocket.toml` and any `ROCKET_APP_` env vars.
            let app_config = match rocket.figment().extract::<AppConfig>() {
                Ok(config) => config,
                Err(e) => {
                    rocket::error!("Failed to extract AppConfig: {}", e);
                    return rocket;
                }
            };

            // 2. Set up the database pool using the URL from our loaded config.
            let pool = match PgPoolOptions::new()
                .max_connections(5)
                .connect(&app_config.database_url)
                .await
            {
                Ok(pool) => pool,
                Err(e) => {
                    rocket::error!("Failed to connect to the database: {}", e);
                    return rocket;
                }
            };

            // 3. Put the database pool, app configuration, and chat state into
            //    Rocket's managed state so handlers can access them.
            rocket.manage(pool).manage(app_config).manage(ChatServerState::new())
        }))
        // The route mounting remains the same.
        .mount("/ws", routes![websocket::handler::ws_handler])
        .mount("/auth", routes![auth::register, auth::login])
        .mount(
            "/api",
            routes![
                chat::get_history,
                chat::list_rooms,
                chat::create_room,
                chat::get_room_members
            ],
        )
        .mount("/", FileServer::from("public"))
        .launch()
        .await?;

    Ok(())
}