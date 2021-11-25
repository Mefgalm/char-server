use std::sync::Arc;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::requests::{self, RawRequest, Request, RequestType, SetNickname};
use crate::room_manager::RoomManager;
use crate::types::{serde_error_to_chat_error, tungstenite_error_to_chat_error, ChatError};
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use uuid::Uuid;

pub struct WsClientConnection {
    pub id: Uuid,
    pub name: Option<String>,
    write_sink: SplitSink<WebSocketStream<TcpStream>, Message>,
}

pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, ChatError> {
    serde_json::from_str::<T>(&s).map_err(serde_error_to_chat_error)
}

fn raw_msg_to_msg(message_text: &str) -> Result<Request, ChatError> {
    let raw_message = from_str::<RawRequest>(&message_text)?;
    match raw_message.request_type {
        RequestType::SetNickname => Ok(Request::SetNickname(from_str::<requests::SetNickname>(
            &raw_message.data,
        )?)),
        RequestType::Message => Ok(Request::Message(from_str::<requests::Message>(
            &raw_message.data,
        )?)),
        RequestType::Disconnected => Ok(Request::Disconnected),
        RequestType::GetId => Ok(Request::GetId),
        RequestType::Online => Ok(Request::Online),
        RequestType::GlobalOnline => Ok(Request::GlobalOnline),
        RequestType::CreateRoom => Ok(Request::CreateRoom(from_str::<requests::CreateRoom>(
            &raw_message.data,
        )?)),
        RequestType::JoinRoom => Ok(Request::JoinRoom(from_str::<requests::JoinRoom>(
            &raw_message.data,
        )?)),
    }
}

impl WsClientConnection {
    pub fn new(
        id: Uuid,
        web_socket: WebSocketStream<TcpStream>,
        sender: Sender<(Uuid, Request)>,
    ) -> Self {
        let (write_sink, mut read_stream) = web_socket.split();

        tokio::spawn(async move {
            while let Some(msg) = read_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => match raw_msg_to_msg(&text) {
                        Ok(msg) => sender.send((id, msg)).await.unwrap(),
                        Err(err) => println!("message_parse err {}", err),
                    },
                    Ok(Message::Close(_)) => {
                        println!("Message::Close! disconnected");
                        sender.send((id, Request::Disconnected)).await.unwrap();
                    }
                    Ok(msg) => {
                        println!("Unexpected msg: {:?}", msg);
                    }
                    Err(err) => {
                        println!("Err(err)! disconnected {}", err);
                        sender.send((id, Request::Disconnected)).await.unwrap();
                        break;
                    }
                }
            }
        });

        Self {
            id,
            name: None,
            write_sink,
        }
    }

    pub async fn send(&mut self, response_str: &str) -> Result<(), ChatError> {
        Ok(self
            .write_sink
            .send(Message::Text(response_str.to_owned()))
            .await
            .map_err(tungstenite_error_to_chat_error)?)
    }
}
