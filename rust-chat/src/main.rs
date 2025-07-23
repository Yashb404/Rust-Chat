// src/main.rs


use dotenvy::dotenv;
use sqlx::{postgres::PgPoolOptions, PgPool};
// --- End New Imports ---
use crate::handlers::auth;
use crate::state::ChatServerState;
use log::info;
use rocket::{routes, Build, Rocket};

mod models;
mod websocket;
mod handlers;
mod state;

#[tokio::main]
async fn main() -> Result<(), rocket::Error> {
  
    dotenv().ok();
    

    env_logger::init();
    info!("Starting chat server...");

  
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    println!("Attempting to connect with URL: {}", db_url);
    // Create a connection pool for PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create database connection pool.");
   

    let chat_state = ChatServerState::new();

    // --- Changed: Updated Rocket build chain ---
    let _rocket = rocket::build()
        .manage(pool) // Add the database pool to Rocket's state
        .manage(chat_state) // Keep your existing chat state
        .mount("/ws", routes![websocket::handler::ws_handler])
        .mount("/auth", routes![auth::register,auth::login])
        .launch() // The .ignite() and .launch() calls are combined here
        .await?;
    

    Ok(())
}