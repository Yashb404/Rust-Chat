#![allow(unused_imports)]                                                          
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::response::stream::TextStream;
use rocket::{get, Request, Response, State};
use rocket::response::Responder;

use rocket::tokio::net::TcpStream;
use rocket::tokio::sync::mpsc;
use rocket::tokio::task;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
// use rocket::ws::{WebSocket, WebSocketUpgrade}; // Removed: not available in Rocket stable

use futures_util::{StreamExt, SinkExt};

use crate::state::ChatServerState;
use crate::models::{RoomId, User, UserId};
use crate::websocket::commands::ChatCommand;

use uuid::Uuid;
use chrono::Utc;

// Placeholder WebSocket upgrade route
// Placeholder WebSocket upgrade route (to be implemented with a compatible crate)
#[get("/chat/<room>?<username>")]
pub async fn ws_handler(
    room: String,
    username: String,
    state: &State<ChatServerState>,
) -> Status {
    // You need to implement WebSocket upgrade using a compatible crate such as `rocket_ws` or handle the handshake manually.
    // For now, just return NotImplemented.
    Status::NotImplemented
}

async fn read_from_socket(
    mut socket: WebSocketStream<TcpStream>,
    user_id: UserId,
    state: ChatServerState,
) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Ok(text) = msg.to_text() {
            match serde_json::from_str::<ChatCommand>(text) {
                Ok(command) => {
                    match command {
                        ChatCommand::JoinRoom { room_id, username } => {
                            // TODO: Add user to room
                            println!("User {} joining room {}", username, room_id);
                        }

                        ChatCommand::SendMessage { room_id, content } => {
                            // TODO: Broadcast message to room
                            println!("User sent message to {}: {}", room_id, content);
                        }
                    }
                }

                Err(err) => {
                    println!("❌ Failed to parse message: {:?}", err);
                }
            }
        }

        println!("User {} disconnected", user_id);
        //TODO: remove user from state.room,state.connections
    }
}