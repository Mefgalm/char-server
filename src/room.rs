use std::{collections::{HashMap, HashSet}, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{server::{CLIENT_NOT_FOUND, WsConnections}, types::ChatError, ws_client_connection::WsClientConnection};

pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub room_clients: Mutex<HashSet<Uuid>>,
    pub clients: Arc<Mutex<WsConnections>>,
}

pub struct RoomInfo {
    pub id: Uuid,
    pub name: String
}

impl Room {
    pub async fn add_client(&mut self, conn_id: &Uuid) {
        self.room_clients
            .lock()
            .await
            .insert(conn_id.clone());
    }

    pub async fn remove_client(&mut self, client_id: &Uuid) {
        self.room_clients.lock().await.remove(client_id);
    }

    pub fn room_info(&self) -> RoomInfo {
        RoomInfo {
            id: self.id,
            name: self.name.to_owned(),
        }
    }

    pub async fn send(
        &mut self,
        predicate: impl Fn(&WsClientConnection) -> bool,
        response_str: &str,
    ) -> Result<(), ChatError> {
        let mut client_ids_to_remove = vec![];

        let room_clients_lock = self.room_clients.lock().await;
        let mut clients_lock = self.clients.lock().await;
        for conn_id in room_clients_lock.iter() {
            let client_lock = clients_lock.get_mut(&conn_id).ok_or(CLIENT_NOT_FOUND)?;
            if predicate(&client_lock) {
                if let Err(err) = client_lock.send(response_str).await {
                    println!("Error while sending message: {}", err);
                    client_ids_to_remove.push(conn_id);
                }
            }
        }

        self.room_clients
            .lock()
            .await
            .retain(|k| !client_ids_to_remove.contains(&k));

        Ok(())
    }

    pub async fn all(&mut self, response_str: &str) -> Result<(), ChatError> {
        Room::send(self, |_| true, response_str).await
    }
}
