use server::Server;

mod requests;
mod responses;
mod room;
mod room_manager;
mod server;
mod service;
mod types;
mod ws_client_connection;

#[tokio::main]
async fn main() {
    let mut server = Server::new("127.0.0.1:3012").await.unwrap();
    server.start().await.unwrap();
}
