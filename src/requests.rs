use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetNickname {
    pub name: String
}

#[derive(Deserialize, Debug, Clone)]
pub enum MessageType {
    User,
    Room
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_type: MessageType,
    pub receiver_id: Uuid,
    pub message: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoom {
    pub name: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoom {
    pub id: Uuid
}

#[derive(Deserialize, Debug, Clone)]
pub enum RequestType {
    GetId,
    SetNickname,
    Online,
    Message,
    Disconnected,
    GlobalOnline,
    CreateRoom,
    JoinRoom
}

#[derive(Debug, Clone)]
pub enum Request {
    GetId,    
    SetNickname(SetNickname),
    Online,
    Message(Message),
    CreateRoom(CreateRoom),
    JoinRoom(JoinRoom),
    Disconnected,
    GlobalOnline
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RawRequest {
    pub request_type: RequestType,
    pub data: String
}