use crate::{
    chessclient::Message,
    game::{MoveDetails, MoveError},
};
use actix::{Message as ActixMessage, Recipient};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum ClientMessage {
    Login(String),
    Enqueue,
    Dequeue,
    LeaveGame,
    PlayAgain,
    MakeMove(MoveDetails),
    Disconnect,
    Ping,
}

#[derive(Serialize)]
pub enum OutgoingMessage {
    MovePiece { from: usize, to: usize },
    RemovePiece { at: usize },
    Check { checker: usize },
    Checkmate { winner: usize },
    Result(Result<(), MoveError>),
    GameStarted,
    WinGame(String),
    LoseGame(String),
}

#[derive(ActixMessage)]
#[rtype(result = "Result<(), ()>")]
pub struct Login {
    pub username: String,
    pub client: Recipient<Message>,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Logout {
    pub username: String,
}
