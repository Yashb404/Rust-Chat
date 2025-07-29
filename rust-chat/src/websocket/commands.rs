use core::error;
use std::sync::Arc;

use log::{info, error};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize, Serialize)]
pub struct OutboundMessage {
    pub room_id: String,
    pub content: String,
    r#type: &'static str,
    username: String,
    
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
                info!("User {} is joining room {}", user_id, room_id);

                state.room_members.entry(room_id).or_default().insert(user_id);      
            }
            ChatCommand::SendMessage { room_id, content }=> {
                let room_uuid = match Uuid::parse_str(&room_id) {
                    Ok(uuid) => uuid,
                    Err(e) => {
                        error!("Invalid room_id UUID: {}: {}", room_id, e);
                        return;
                    }
                };
                let result = sqlx::query!(
                    "INSERT INTO messages (room_id, user_id, content) VALUES ($1, $2, $3)",
                    room_uuid,
                    user_id,
                    content.clone()
                )
                .execute(pool)
                .await;

            if let Err(e) = result{
                error!("Failed to insert message: {}", e);
                return;
            }

            //TODO: add broadcast logic here after db write succeeds

            let sender = match sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
                .fetch_one(pool)
                .await {
                    Ok(record) => record,
                    Err(e) => {
                        error!("Failed to fetch username: {}: {}",user_id, e);
                        return; // cannot proceed without sender info
                    }
                };

                let outbound_msg = OutboundMessage {
                    r#type: "new message",
                    username: sender.username,
                    room_id: room_id.clone(),
                    content,
                };
                if let Ok(message_json) = serde_json::to_string(&outbound_msg){
                    if let Some(members) = state.room_members.get(&room_id){

                        for member_id_ref in members.iter() {
                            let member_id = member_id_ref.key();

                            if let Some(connection) =  state.connections.get(member_id){

                                let _ = connection.send(message_json.clone());
                            }
                        }
                    }
                }
            }

        }
    }
}