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

use rocket_ws as ws;

use futures_util::{StreamExt, SinkExt};

use crate::state::ChatServerState;
use crate::models::{RoomId, User, UserId};
use crate::websocket::commands::ChatCommand;

use uuid::Uuid;
use chrono::Utc;
use serde::Serialize;

use dashmap::DashSet;
use dashmap::DashMap;

#[derive(Serialize)]
struct OutboundMessage {
    #[serde(rename = "type")]
    r#type: &'static str,
    from: String,
    content: String,
}

// Register a new user in the chat system
fn register_user(username: String, room: &str, state: &ChatServerState) -> (UserId, User) {
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: username.clone(),
        connected_at: Utc::now(),
    };

    state.room_members
        .entry(room.to_string())
        .or_insert_with(DashSet::new)
        .insert(user_id);

    println!("{} joined room '{}'", username, room);
    (user_id, user)
}

// Clean up user when they disconnect
fn cleanup_user(user_id: UserId, username: &str, room: &str, state: &ChatServerState) {
    // Remove user from all rooms
    let room_keys: Vec<_> = state.room_members.iter().map(|r| r.key().clone()).collect();
    for room_id in room_keys {
        if let Some(mut members) = state.room_members.get_mut(&room_id) {
            members.remove(&user_id);
        }
    }
    state.connections.remove(&user_id);
    println!("{} left room '{}'", username, room);
}

#[get("/chat/<room>?<username>")]
pub fn ws_handler<'a>(
    ws: ws::WebSocket,
    room: &'a str,
    username: &'a str,
    state: &'a State<ChatServerState>,
) -> ws::Channel<'a> {
    let room = room.to_string();
    let username = username.to_string();
    let state = state.inner().clone();

    ws.channel(move |mut stream| Box::pin(async move {
        // TODO: Validate username and room name
        // TODO: Add input validation for security

        let (user_id, _user) = register_user(username.clone(), &room, &state);
        let (tx, mut rx) = mpsc::unbounded_channel();
        state.connections.insert(user_id, tx);

        // Handle WebSocket communication inline (full-duplex)
        loop {
            tokio::select! {
                // Read incoming messages from client
                msg = stream.next() => {
                    match msg {
                        Some(Ok(ws::Message::Text(text))) => {
                            if let Ok(cmd) = serde_json::from_str::<ChatCommand>(&text) {
                                match cmd {
                                    ChatCommand::SendMessage { room_id, content } => {
                                        // TODO: Validate message length and content
                                        println!(" {} → {}: {}", username, room_id, content);
                                        
                                        let outbound_msg = OutboundMessage {
                                            r#type: "new_message",
                                            from: username.clone(),
                                            content,
                                        };
                                        
                                        if let Ok(message) = serde_json::to_string(&outbound_msg) {
                                            broadcast_to_room(&state, &room_id, &message);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(ws::Message::Close(_))) => {
                            println!("🔌 {} closed connection", username);
                            break;
                        }
                        Some(Err(e)) => {
                            println!(" WebSocket error: {:?}", e);
                            break;
                        }
                        None => {
                            println!("{} disconnected", username);
                            break;
                        }
                        _ => {} // Ignore other message types
                    }
                }
                
                // Write outgoing messages to client
                msg = rx.recv() => {
                    match msg {
                        Some(message) => {
                            if let Err(e) = stream.send(ws::Message::Text(message)).await {
                                println!("Failed to send message: {:?}", e);
                                break;
                            }
                        }
                        None => {
                            println!("Channel closed for {}", username);
                            break;
                        }
                    }
                }
            }
        }

        // Clean up on disconnect
        cleanup_user(user_id, &username, &room, &state);
        Ok(())
    }))
}

fn broadcast_to_room(state: &ChatServerState, room_id: &RoomId, message: &str) {
    if let Some(members) = state.room_members.get(room_id) {
        // Collect user IDs into a Vec to avoid RefMulti issues
        let users: Vec<_> = members.iter().map(|id| *id).collect();
        
        for user_id in users {
            if let Some(sender) = state.connections.get(&user_id) {
                let _ = sender.send(message.to_string());
            }
        }
    }
}