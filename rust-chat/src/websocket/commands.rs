use serde::Deserialize;

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
