use std::{collections::HashMap, sync::Arc};

use crate::responses;
use crate::server::WsConnections;
use crate::{requests, server::CLIENT_NOT_FOUND};
use chrono::Utc;
use futures::future;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    room_manager::RoomManager,
    types::{serde_error_to_chat_error, ChatError},
    ws_client_connection::WsClientConnection,
};

fn create_response_str<T: Serialize>(
    response_type: responses::ResponseType,
    value: T,
) -> Result<String, ChatError> {
    serde_json::to_string(&responses::Response {
        response_type,
        data: serde_json::to_string(&value).unwrap(),
    })
    .map_err(serde_error_to_chat_error)
}

async fn send(
    ws_connections: Arc<Mutex<WsConnections>>,
    predicate: impl Fn(&WsClientConnection) -> bool,
    response_str: &str,
) -> Result<(), ChatError> {
    // send reponses
    let results = {
        let mut lock_ws_connections = ws_connections.lock().await;
        future::join_all(
            lock_ws_connections
                .iter_mut()
                .filter(|(_, c)| predicate(c))
                .map(|(conn_id, c)| async move { (conn_id.clone(), c.send(&response_str).await) }),
        )
        .await
        .into_iter()
        .collect::<Vec<(Uuid, Result<(), ChatError>)>>()
    };

    // connections that no longer active
    let mut connections_to_remove = vec![];
    // collect failed connections
    for (client_id, result) in results {
        if let Err(error) = result {
            println!("Send error: {}", error);
            connections_to_remove.push(client_id);
        }
    }

    if !connections_to_remove.is_empty() {
        // remove dead connections
        {
            ws_connections
                .lock()
                .await
                .retain(|id, _| !connections_to_remove.contains(&id));
        }

        // sent to each alive connectin offline status
        let mut lock_ws_connections_to_remove = ws_connections.lock().await;
        for conn_to_remove_id in connections_to_remove {
            let offline_response = &create_response_str(
                responses::ResponseType::Offline,
                responses::Offline {
                    id: conn_to_remove_id.clone(),
                },
            )?;

            future::join_all(
                lock_ws_connections_to_remove
                    .iter_mut()
                    .map(|(_, c)| c.send(&offline_response)),
            )
            .await;
        }
    }

    Ok(())
}

async fn all(
    ws_connections: Arc<Mutex<WsConnections>>,
    response_str: &str,
) -> Result<(), ChatError> {
    send(Arc::clone(&ws_connections), |_| true, response_str).await
}

async fn direct(
    ws_connections: Arc<Mutex<WsConnections>>,
    receiver_id: Uuid,
    response_str: &str,
) -> Result<(), ChatError> {
    send(
        Arc::clone(&ws_connections),
        |c| c.id == receiver_id,
        response_str,
    )
    .await
}

async fn other(
    client_id: Uuid,
    ws_connections: Arc<Mutex<WsConnections>>,
    response_str: &str,
) -> Result<(), ChatError> {
    send(
        Arc::clone(&ws_connections),
        |conn| conn.id != client_id,
        response_str,
    )
    .await
}

async fn get_id(conn_id: Uuid, ws_connections: Arc<Mutex<WsConnections>>) -> Result<(), ChatError> {
    let lock_connections = &mut ws_connections.lock().await;
    let connection = lock_connections.get_mut(&conn_id).ok_or(CLIENT_NOT_FOUND)?;

    // Send client id
    connection
        .send(&create_response_str(
            responses::ResponseType::GetId,
            responses::GetId { id: conn_id },
        )?)
        .await?;

    Ok(())
}

async fn online(conn_id: Uuid, ws_connections: Arc<Mutex<WsConnections>>) -> Result<(), ChatError> {
    let name: String;
    {
        let lock_ws_connections = ws_connections.lock().await;
        let conn = lock_ws_connections.get(&conn_id).ok_or(CLIENT_NOT_FOUND)?;
        name = conn.name.to_owned().unwrap();
    }

    all(
        Arc::clone(&ws_connections),
        &create_response_str(
            responses::ResponseType::Online,
            responses::Online { id: conn_id, name },
        )?,
    )
    .await?;

    Ok(())
}

async fn set_nickname(
    client_id: Uuid,
    req: &requests::SetNickname,
    ws_connections: Arc<Mutex<WsConnections>>,
) -> Result<(), ChatError> {
    {
        let lock_connections = &mut ws_connections.lock().await;
        let connection = lock_connections
            .get_mut(&client_id)
            .ok_or(CLIENT_NOT_FOUND)?;
        connection.name = Some(req.name.to_owned());
    }

    all(
        Arc::clone(&ws_connections),
        &create_response_str(
            responses::ResponseType::SetNickname,
            responses::SetNickname {
                id: client_id,
                name: req.name.to_owned(),
            },
        )?,
    )
    .await?;

    Ok(())
}

async fn user_message(
    conn_id: Uuid,
    receiver_id: Uuid,
    message: &str,
    ws_connections: Arc<Mutex<WsConnections>>,
) -> Result<(), ChatError> {
    let name: String;
    {
        let lock_clients = &mut ws_connections.lock().await;
        let connection = lock_clients.get(&conn_id).ok_or(CLIENT_NOT_FOUND)?;
        name = connection.name.clone().unwrap();
    }
    direct(
        Arc::clone(&ws_connections),
        receiver_id,
        &create_response_str(
            responses::ResponseType::Message,
            responses::Message {
                id: conn_id,
                name,
                message: message.to_owned(),
                created_at: Utc::now(),
            },
        )?,
    )
    .await?;
    Ok(())
}

async fn room_message(
    conn_id: Uuid,
    room_id: Uuid,
    message: &str,
    ws_connections: Arc<Mutex<WsConnections>>,
    room_manager: Arc<Mutex<RoomManager>>,
) -> Result<(), ChatError> {
    let name: String;
    {
        let lock_clients = &mut ws_connections.lock().await;
        let connection = lock_clients.get(&conn_id).ok_or(CLIENT_NOT_FOUND)?;
        name = connection.name.clone().unwrap();
    }

    room_manager
        .lock()
        .await
        .all(
            &room_id,
            &create_response_str(
                responses::ResponseType::Message,
                responses::Message {
                    id: conn_id,
                    name,
                    message: message.to_owned(),
                    created_at: Utc::now(),
                },
            )?,
        )
        .await
}

async fn disconnected(
    conn_id: Uuid,
    ws_connections: Arc<Mutex<WsConnections>>,
) -> Result<(), ChatError> {
    {
        ws_connections.lock().await.remove(&conn_id);
    }
    other(
        conn_id,
        Arc::clone(&ws_connections),
        &create_response_str(
            responses::ResponseType::Offline,
            responses::Offline { id: conn_id },
        )?,
    )
    .await?;

    Ok(())
}

async fn global_online(
    conn_id: Uuid,
    ws_connections: Arc<Mutex<WsConnections>>,
) -> Result<(), ChatError> {
    let lock_connections = &mut ws_connections.lock().await;
    let user_infos: Vec<responses::UserInfo> = lock_connections
        .iter_mut()
        .map(|(_, connection)| responses::UserInfo {
            id: connection.id,
            name: connection.name.clone().unwrap(),
        })
        .collect();

    let connection = lock_connections.get_mut(&conn_id).ok_or(CLIENT_NOT_FOUND)?;
    connection
        .send(&create_response_str(
            responses::ResponseType::GlobalOnline,
            responses::GlobalOnline { users: user_infos },
        )?)
        .await?;

    Ok(())
}

async fn create_room(
    conn_id: Uuid,
    req: &requests::CreateRoom,
    ws_connections: Arc<Mutex<WsConnections>>,
    room_manager: Arc<Mutex<RoomManager>>,
) -> Result<(), ChatError> {
    let room_id = Uuid::new_v4();
    room_manager
        .lock()
        .await
        .create(&conn_id, &room_id, &req.name, Arc::clone(&ws_connections))
        .await?;

    direct(
        Arc::clone(&ws_connections),
        conn_id,
        &create_response_str(
            responses::ResponseType::RoomCreated,
            responses::RoomCreated {
                id: room_id.clone(),
                name: req.name.to_owned(),
            },
        )?,
    )
    .await?;

    Ok(())
}

async fn join_room(
    conn_id: Uuid,
    req: &requests::JoinRoom,
    ws_connections: Arc<Mutex<WsConnections>>,
    room_manager: Arc<Mutex<RoomManager>>,
) -> Result<(), ChatError> {
    let uuid = Uuid::parse_str(&req.id).map_err(|e| e.to_string())?;
    let room_info = room_manager.lock().await.join(&uuid, &conn_id).await?;

    direct(
        Arc::clone(&ws_connections),
        conn_id,
        &create_response_str(
            responses::ResponseType::RoomJoined,
            responses::RoomJoined {
                id: room_info.id,
                name: room_info.name,
            },
        )?,
    )
    .await
}

pub async fn send_error(
    conn_id: Uuid,
    error_message: &str,
    ws_connections: Arc<Mutex<WsConnections>>,
) -> Result<(), ChatError> {
    direct(
        Arc::clone(&ws_connections),
        conn_id,
        &create_response_str(
            responses::ResponseType::Error,
            responses::ResponseError {
                message: error_message.to_owned(),
            },
        )?,
    )
    .await
}

pub async fn switch_request(
    conn_id: Uuid,
    request: requests::Request,
    ws_connections: Arc<Mutex<WsConnections>>,
    room_manager: Arc<Mutex<RoomManager>>,
) {
    let ws_connections = Arc::clone(&ws_connections);
    let error_ws_connections = Arc::clone(&ws_connections);
    let room_manager = Arc::clone(&room_manager);
    println!("{:?}", request);
    if let Err(error) = match &request {
        requests::Request::SetNickname(req) => set_nickname(conn_id, req, ws_connections).await,
        requests::Request::Message(req) => match req.message_type {
            requests::MessageType::User => {
                user_message(conn_id, req.receiver_id, &req.message, ws_connections).await
            }
            requests::MessageType::Room => {
                room_message(
                    conn_id,
                    req.receiver_id,
                    &req.message,
                    ws_connections,
                    room_manager,
                )
                .await
            }
        },
        requests::Request::GetId => get_id(conn_id, ws_connections).await,
        requests::Request::Online => online(conn_id, ws_connections).await,
        requests::Request::GlobalOnline => global_online(conn_id, ws_connections).await,
        requests::Request::Disconnected => disconnected(conn_id, ws_connections).await,
        requests::Request::CreateRoom(req) => {
            create_room(conn_id, req, ws_connections, room_manager).await
        }
        requests::Request::JoinRoom(req) => {
            join_room(conn_id, req, ws_connections, room_manager).await
        }
    } {
        let _ = send_error(conn_id, &error, error_ws_connections).await;
        
        println!("{:?} failed with msg: '{}'", request, error);
    }
}
