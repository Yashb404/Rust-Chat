use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::{get, Request, Response, State};
use rocket::response::Responder;

use rocket::tokio::net::TcpStream;
use rocket::tokio::sync::mpsc;
use rocket::tokio::task;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;

use futures_util::{StreamExt, SinkExt};

use crate::state::ChatServerState;
use crate::models::{RoomId, User, UserId};

use uuid::Uuid;
use chrono::Utc;

// Placeholder WebSocket upgrade route
#[get("/chat/<room>?<username>")]
pub async fn ws_handler(
    room: String,
    username: String,
    state: &State<ChatServerState>,
    request: &Request<'_>,
) -> Result<impl Responder<'static>, Status> {
    // 👇 In the next steps, we’ll:
    // - Check username validity
    // - Upgrade connection to WebSocket
    // - Register user
    // - Spawn reader/writer tasks

    // For now, just return 501 Not Implemented
    Err(Status::NotImplemented)
}
