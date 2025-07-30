use rocket::{get, post, response, serde::json::Json, Responder, State};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{handlers::guard::AuthenticatedUser, state::ChatServerState};


#[derive(Serialize, sqlx::FromRow)]
pub struct MessageRecord {
    id: i64,
    user_id: Uuid,
    username: String, // We'll get this with a JOIN
    room_id: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct UserRecord {
    id: Uuid,
    username: String,
}

#[derive(Responder)]
#[response(status = 500, content_type = "json")]
pub struct RoomError(String);


#[derive(Responder, Debug)]
pub enum ApiError {
    #[response(status = 500, content_type = "json")]
    DatabaseError(String),
    #[response(status = 404, content_type = "json")]
    NotFound(String),
}

#[get("/rooms")]
pub async fn list_rooms(
    _user: AuthenticatedUser,
    pool: &State<PgPool>,

)-> Result<Json<Vec<RoomRecord>>, RoomError>{
    let rooms = sqlx::query_as!(
        RoomRecord,
        "SELECT id, name FROM rooms ORDER BY name"
    ).fetch_all(pool.inner())
        .await
        .map_err(|e| RoomError(e.to_string()))?;
    Ok(Json(rooms))
    }


#[derive(Serialize, sqlx::FromRow)]
pub struct RoomRecord{
    id: Uuid,
    name:String,
}

#[derive(Responder)]
pub enum HistoryError{
    #[response(status = 500)]
    InternalError(String),
}

#[derive(Deserialize)]
pub struct CreateRoomPayload {
    name: String,
}
// GET /api/history/<room_id>
// The <room_id> in the path is captured and passed as an argument.
#[get("/history/<room_id>")]
pub async fn get_history(
    room_id: String,
    _user: AuthenticatedUser,
    pool: &State<PgPool>,
) -> Result<Json<Vec<MessageRecord>>, HistoryError> {
    let room_uuid = Uuid::parse_str(&room_id)
        .map_err(|e| HistoryError::InternalError(format!("Invalid room_id: {}", e)))?;
    let messages = sqlx::query_as!(
        MessageRecord,
        r#"
        SELECT m.id, m.user_id, u.username, m.room_id, m.content, m.created_at
        FROM messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.room_id = $1
        ORDER BY m.created_at DESC
        LIMIT 50
        "#,
        room_uuid
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| HistoryError::InternalError(e.to_string()))?;
    Ok(Json(messages))
}

#[post("/rooms", data = "<payload>")]
pub async fn create_room(
    payload: Json<CreateRoomPayload>,
    _user: AuthenticatedUser, // Guard ensures the user is logged in.
    pool: &State<PgPool>,
) -> Result<Json<RoomRecord>, RoomError> {
    
    // We use `query_as!` to execute the INSERT and immediately return the newly
    // created row, which we then map directly into our `RoomRecord` struct.
    // The `ON CONFLICT (name) DO NOTHING` clause prevents duplicate room names.
    // If a conflict occurs, the query does nothing and returns no rows.
    let new_room = sqlx::query_as!(
        RoomRecord,
        "INSERT INTO rooms (name) VALUES ($1) RETURNING id, name",
        payload.name
    )
    .fetch_optional(pool.inner()) // Use `fetch_optional` because a conflict returns no row.
    .await
    .map_err(|e| RoomError(e.to_string()))?
    .ok_or_else(|| RoomError("A room with this name already exists.".to_string()))?;

    // On success, return a 200 OK with the JSON of the newly created room record.
    Ok(Json(new_room))
}

#[get("/rooms/<room_id>/members")]
pub async fn get_room_members(
    room_id: String,
    _user: AuthenticatedUser,       // Ensures the requester is logged in.
    chat_state: &State<ChatServerState>, // Access to in-memory state.
    pool: &State<PgPool>,            // Access to the database.
) -> Result<Json<Vec<UserRecord>>, ApiError> { // Updated error type
    
    // Get the list of active user IDs from the in-memory state.
    let member_ids = match chat_state.room_members.get(&room_id) {
        Some(members) => {
            members.iter().map(|id_ref| *id_ref.key()).collect::<Vec<Uuid>>()
        }
        None => {
            // If the room doesn't exist in memory, it's effectively not found.
            return Err(ApiError::NotFound("Room not found or is empty.".to_string()));
        }
    };
  // Both branches must return the same type: Result<Json<...>, ApiError>.
    if member_ids.is_empty() {
        // If the room is empty, return an empty JSON array.
        Ok(Json(vec![]))
    } else {
        // If there are members, query the database for their details.
        let members = sqlx::query_as!(
            UserRecord,
            "SELECT id, username FROM users WHERE id = ANY($1)",
            &member_ids
        )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(Json(members))
    }
}