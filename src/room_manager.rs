use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{room::{Room, RoomInfo}, server::WsConnections, types::ChatError, ws_client_connection::WsClientConnection};

pub type Rooms = HashMap<Uuid, Room>;

pub struct RoomManager {
    pub rooms: Mutex<Rooms>,
}

pub const ROOM_NOT_FOUND: &str = "Room not found";

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
        }
    }

    pub async fn all(&self, id: &Uuid, response_str: &str) -> Result<(), ChatError> {
        let mut locked_rooms = self.rooms.lock().await;
        let room = locked_rooms.get_mut(id).ok_or(ROOM_NOT_FOUND.to_owned())?;
        room.all(response_str).await?;
        Ok(())
    }

    pub async fn create(
        &mut self,
        conn_id: &Uuid,
        id: &Uuid,
        name: &str,
        clients: Arc<Mutex<WsConnections>>,
    ) -> Result<(), ChatError> {
        let mut clients_map = HashSet::new();
        clients_map.insert(conn_id.clone());

        self.rooms.lock().await.insert(
            id.clone(),
            Room {
                id: id.clone(),
                name: name.to_owned(),
                room_clients: Mutex::new(clients_map),
                clients,
            },
        );

        Ok(())
    }

    pub async fn join(&mut self, room_id: &Uuid, conn_id: &Uuid) -> Result<RoomInfo, ChatError> {
        let mut room_lock = self.rooms.lock().await;
        let room = room_lock.get_mut(&room_id).ok_or(ROOM_NOT_FOUND)?;

        room.add_client(conn_id).await;

        Ok(room.room_info())
    }
}
