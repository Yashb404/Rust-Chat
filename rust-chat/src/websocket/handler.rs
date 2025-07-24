// src/websocket/handler.rs

use futures_util::StreamExt;
use rocket::{get, State};
use rocket_ws as ws;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use log::{error, info};
use futures_util::SinkExt;

use crate::state::ChatServerState;
use crate::websocket::commands::ChatCommand;
use crate::handlers::guard::AuthenticatedUser;
use uuid::Uuid;
use sqlx::PgPool;

#[get("/")]
pub fn ws_handler<'a>(
    ws: ws::WebSocket,
    state: &'a State<ChatServerState>,
    user: AuthenticatedUser,
    db_pool: &'a State<PgPool>,
) -> ws::Channel<'a> {
    // Clone the application state and database pool to move them into the async block.
    // Cloning an Arc or a PgPool is cheap as it only increments a reference counter.
    let state = state.inner().clone();
    let pool = db_pool.inner().clone();

    // The `ws.channel` method takes a closure that will be executed for each new connection.
    ws.channel(move |mut stream| Box::pin(async move {
        // The user_id is extracted from the AuthenticatedUser guard.
        // Its presence here guarantees the user has provided a valid JWT.
        let user_id = user.user_id;
        info!("WebSocket opened for user {}", user_id);

        // Create a multi-producer, single-consumer channel for this specific user.
        // The `tx` (transmitter) end is stored in the global state, allowing other
        // parts of the application to send messages to this user.
        let (tx, mut rx) = unbounded_channel();
        state.connections.insert(user_id, tx);

        // Split the WebSocket stream into a sender and receiver half.
        // This allows for concurrent reading and writing.
        let (mut ws_sender, mut ws_receiver) = stream.split();

        // Spawn a dedicated asynchronous task to handle incoming messages from the client.
        let state_read = state.clone();
        let pool_read = pool.clone();
        let read_handle = tokio::spawn(async move {
            // Loop until the client disconnects or an error occurs.
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    // Handle incoming text messages.
                    Ok(ws::Message::Text(text)) => {
                        // Attempt to deserialize the JSON text into a defined ChatCommand.
                        match serde_json::from_str::<ChatCommand>(&text) {
                            Ok(cmd) => {
                                // If deserialization is successful, delegate the command
                                // to the central executor, providing verified context.
                                //TODO: add in websocket/commands.rs
                                ChatCommand::execute(cmd, user_id, &state_read, &pool_read).await;
                            }
                            // Log if the client sends a command that doesn't match the expected format.
                            Err(e) => error!("Malformed cmd from {}: {}", user_id, e),
                        }
                    }
                    // A clean close or any stream error are both terminal events for the read loop.
                    Ok(ws::Message::Close(_)) | Err(_) => break,
                    // Ignore other message types (Binary, Ping, Pong, etc.).
                    _ => {}
                }
            }
            // --- Cleanup Logic ---
            // This code runs only after the `while` loop has been broken.
            // Remove the user's connection sender from the global state.
            state_read.connections.remove(&user_id);
            // Iterate through all rooms and remove the user from the member list.
            // This is a temporary solution; a more efficient approach would be to track
            // which rooms the user is in.
            for mut room in state_read.room_members.iter_mut() {
                room.value_mut().remove(&user_id);
            }
            info!("Cleaned up user {}", user_id);
        });

        // Spawn a second dedicated task to handle outgoing messages to the client.
        let write_handle = tokio::spawn(async move {
            // Loop until the corresponding `tx` is dropped or the channel is closed.
            while let Some(msg) = rx.recv().await {
                // Forward the message from the MPSC channel to the WebSocket sink.
                if ws_sender.send(ws::Message::Text(msg)).await.is_err() {
                    // An error here indicates the client connection is broken.
                    // We break the loop to terminate the write task.
                    break;
                }
            }
        });

        // Await both tasks. This ensures the connection is held open until both
        // reading and writing are complete (or one of them fails).
        let _ = tokio::try_join!(read_handle, write_handle);
        Ok(())
    }))
}
