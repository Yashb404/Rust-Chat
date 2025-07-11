mod models;
mod websocket;
mod handlers;

use log::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();
    
    info!("Starting chat server...");
    
    // TODO: We'll add server startup code here
    println!("Chat server is ready!");
}