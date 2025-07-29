use rocket::{get, response, serde::json::Json, Responder, State};
use serde::Serialize;
use sqlx::PgPool;

use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::handlers::guard::AuthenticatedUser;


#[derive(Serialize, sqlx::FromRow)]
pub struct MessageRecord {
    id: i64,
    user_id: Uuid,
    username: String, // We'll get this with a JOIN
    room_id: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Responder)]
pub enum HistoryError{
    #[response(status = 500)]
    InternalError(String),
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