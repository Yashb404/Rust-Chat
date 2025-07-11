use crate::models::{Room, RoomId, User, UserId};
use dashmap::{DashMap, DashSet};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct ChatServerState {
    pub rooms: Arc<DashMap<RoomId,Room>>,
    pub room_members: Arc<DashMap<RoomId, DashSet<UserId>>>,
    pub connections: Arc<DashMap<UserId, UnboundedSender<String>>>,
}

impl ChatServerState {
    pub fn new() -> Self{
        Self {
            rooms: Arc::new(DashMap::new()),
            room_members: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
        }
    }
}