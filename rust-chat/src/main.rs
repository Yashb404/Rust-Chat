mod models;
mod websocket;
mod handlers;
mod state;

use crate::state::ChatServerState;
use log::info;
use rocket::{routes, Build, Rocket};

#[tokio::main]
async fn main() -> Result<(), rocket::Error> {
    env_logger::init();
    info!("Starting chat server...");

    let chat_state = ChatServerState::new();

    let _rocket = rocket::build()
        .manage(chat_state)
        .mount("/ws", routes![websocket::handler::ws_handler]) // We'll add routes soon
        .ignite()
        .await?
        .launch()
        .await?;
    

    Ok(())
}
