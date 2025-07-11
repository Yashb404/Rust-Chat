//TODO: We'll define our data structures here

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;
pub type RoomId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}