use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;
use uuid::Uuid;

use crate::requests::Request;
use crate::room_manager::RoomManager;
use crate::service;
use crate::ws_client_connection::WsClientConnection;

pub type WsConnections = HashMap<Uuid, WsClientConnection>;

pub const CLIENT_NOT_FOUND: &str = "Client not found";

pub struct Server {
    clients: Arc<Mutex<WsConnections>>,
    tcp_listener: Arc<Mutex<TcpListener>>,
    room_manager: Arc<Mutex<RoomManager>>,
}

impl Server {
    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        let tcp_listener = TcpListener::bind(addr).await?;

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            tcp_listener: Arc::new(Mutex::new(tcp_listener)),
            room_manager: Arc::new(Mutex::new(RoomManager::new())),
        })
    }

    pub fn start_listen(
        tcp_listener: Arc<Mutex<TcpListener>>,
        clients: Arc<Mutex<HashMap<Uuid, WsClientConnection>>>,
        sender: Sender<(Uuid, Request)>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let stream_result = tcp_listener.lock().await.accept().await;
                match stream_result {
                    Ok((stream, _)) => match accept_async(stream).await {
                        Ok(web_socket) => {
                            let mut clients = clients.lock().await;
                            let client_id = Uuid::new_v4();
                            clients.insert(
                                client_id,
                                WsClientConnection::new(client_id, web_socket, sender.clone()),
                            );
                            println!("Client connected: Count: {}", clients.len());
                        }
                        Err(err) => {
                            println!("Accept websocket: {}", err)
                        }
                    },
                    Err(err) => {
                        println!("Stream error: {}", err)
                    }
                }
            }
        })
    }

    pub fn start_receiver(
        clients: Arc<Mutex<WsConnections>>,
        room_manager: Arc<Mutex<RoomManager>>,
        mut receiver: Receiver<(Uuid, Request)>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some((client_id, request)) = receiver.recv().await {
                service::switch_request(
                    client_id,
                    request,
                    Arc::clone(&clients),
                    Arc::clone(&room_manager),
                )
                .await;
            }
        })
    }

    pub fn start(&mut self) -> JoinHandle<()> {
        let (sender, receiver) = mpsc::channel::<(Uuid, Request)>(32);

        Server::start_receiver(
            Arc::clone(&self.clients),
            Arc::clone(&self.room_manager),
            receiver,
        );
        Server::start_listen(
            Arc::clone(&self.tcp_listener),
            Arc::clone(&self.clients),
            sender,
        )
    }
}
