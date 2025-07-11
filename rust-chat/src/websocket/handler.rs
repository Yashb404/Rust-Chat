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
// Removed: not available in Rocket stable

use futures_util::{StreamExt, SinkExt};

use crate::state::ChatServerState;
use crate::models::{RoomId, User, UserId};
use crate::websocket::commands::ChatCommand;

use uuid::Uuid;
use chrono::Utc;

use dashmap::DashSet;
use dashmap::DashMap;
// Placeholder WebSocket upgrade route
// Placeholder WebSocket upgrade route (to be implemented with a compatible crate)
#[get("/chat/<room>?<username>")]
pub fn ws_handler<'a>(
    ws: ws::WebSocket,
    room: &'a str,
    username: &'a str,
    state: &'a State<ChatServerState>,
) -> ws::Channel<'a>{
    let room = room.to_string();
    let username = username.to_string();
    let state = state.inner().clone();

    ws.channel(move |mut stream| Box::pin(async move {
        let user_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let user = User {
            id: user_id,
            username: username.clone(),
            connected_at: Utc::now(),
        };

        state.connections.insert(user_id, tx);
        state.room_members
            .entry(room.clone())
            .or_insert_with(DashSet::new)
            .insert(user_id);

        println!("✅ {} joined room '{}'", username, room);

        let state_clone = state.clone();
        let room_clone = room.clone();
        let username_clone = username.clone();
        let read_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Ok(text) = msg.to_text() {
                    if let Ok(cmd) = serde_json::from_str::<ChatCommand>(text) {
                        match cmd {
                            ChatCommand::SendMessage { room_id, content } => {
                                println!("💬 {} → {}: {}", username_clone, room_id, content);
                                let message = format!(
                                    "{{\"type\":\"new_message\",\"from\":\"{}\",\"content\":\"{}\"}}",
                                    username_clone, content
                                );
                                broadcast_to_room(&state_clone, &room_id, &message);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Clean up on disconnect: remove user from all rooms
            let room_keys: Vec<_> = state_clone.room_members.iter().map(|r| r.key().clone()).collect();
            for room_id in room_keys {
                if let Some(mut members) = state_clone.room_members.get_mut(&room_id) {
                    members.remove(&user_id);
                }
            }
            state_clone.connections.remove(&user_id);
            println!("❌ {} left room '{}'", username_clone, room_clone);
        });

        // ✉️ WRITING to client
        while let Some(msg) = rx.recv().await {
            stream.send(ws::Message::Text(msg)).await?;
        }

        read_task.abort();
        Ok(())
    }))
}

fn broadcast_to_room(state: &ChatServerState, room_id: &RoomId, message: &str) {
    if let Some(members) = state.room_members.get(room_id) {
        for user_id in members.iter() {
            if let Some(sender) = state.connections.get(user_id) {
                let _ = sender.send(message.to_string());
            }
        }
    }
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