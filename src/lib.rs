use bytes::Bytes;
use std::collections::HashMap;

pub type GameState = HashMap<String, Bytes>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerMessage {
    GameState(GameState),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientMessage {
    Set(String, Bytes),
}
