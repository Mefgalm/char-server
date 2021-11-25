use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetId {
    pub id : Uuid
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetNickname {
    pub id : Uuid,
    pub name: String
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Uuid,
    pub name: String,
    pub message: String,
    pub created_at: DateTime<Utc>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Offline {
    pub id: Uuid
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Online {
    pub id: Uuid,
    pub name: String
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GlobalOnline {
    pub users: Vec<UserInfo>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoomCreated {
    pub id: Uuid,
    pub name: String
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoomJoined {
    pub id: Uuid,
    pub name: String
}

#[derive(Serialize, Debug)]
pub enum ResponseType {
    GetId,
    Online,
    Offline,
    SetNickname,
    Message,
    GlobalOnline,
    RoomCreated,
    RoomJoined
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub response_type: ResponseType,
    pub data: String
}