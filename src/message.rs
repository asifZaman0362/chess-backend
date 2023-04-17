use actix::{Addr, Message};
use serde::Deserialize;

use crate::chessclient::ChessClient;

#[derive(Deserialize)]
pub enum ClientMessage {
    Login(String),
    Enqueue,
    Dequeue,
    LeaveGame,
    PlayAgain,
}

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct Login {
    pub username: String,
    pub client: Addr<ChessClient>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Logout {
    pub username: String,
}
