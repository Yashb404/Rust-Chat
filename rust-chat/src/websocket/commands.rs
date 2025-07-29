use std::sync::Arc;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::ChatServerState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ChatCommand {
    #[serde(rename = "join_room")]
    JoinRoom {
        room_id: String,
        username: String,
    },

    #[serde(rename = "send_message")]
    SendMessage {
        room_id: String,
        content: String,
    },
}

impl ChatCommand{
    pub async fn execute(
        cmd: ChatCommand,
        user_id: Uuid,
        state: &ChatServerState,
        pool: &PgPool,
    ){
        match cmd {
            ChatCommand:: JoinRoom{room_id, username }=>{
                //joining a room
            }
            ChatCommand::SendMessage { room_id, content }=> {
                //sending a message
            }

        }
    }
}