use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;
use uuid::Uuid;

use crate::ws_client_connection::WsClientConnection;

pub type ChatError = String;

pub fn tungstenite_error_to_chat_error(error: tungstenite::Error) -> ChatError {
    error.to_string()
}

pub fn serde_error_to_chat_error(error: serde_json::Error) -> ChatError {
    error.to_string()
}