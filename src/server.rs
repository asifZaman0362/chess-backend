use actix::{Actor, Context, Handler, Message as ActixMessage, Recipient};
use std::collections::HashMap;

use crate::{
    chessclient::Message,
    game::Game,
    message::{ClientResult, Disconnect, Login, Logout, OutgoingMessage},
};

pub struct Server {
    users: HashMap<String, Recipient<Message>>,
    waiting_for_game: Option<Recipient<Message>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            waiting_for_game: None,
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Login> for Server {
    type Result = ();
    fn handle(&mut self, msg: Login, _ctx: &mut Self::Context) -> Self::Result {
        let res = Message {
            inner: match self.users.get(&msg.username) {
                Some(_) => OutgoingMessage::Result(ClientResult::LoginError),
                None => OutgoingMessage::Result(ClientResult::Ok),
            },
            game: None,
        };
        msg.client.do_send(res);
    }
}

impl Handler<Logout> for Server {
    type Result = ();
    fn handle(&mut self, msg: Logout, _ctx: &mut Self::Context) -> Self::Result {
        self.users.remove(&msg.username);
    }
}

impl Handler<Disconnect> for Server {
    type Result = ();
    fn handle(&mut self, msg: Disconnect, ctx: &mut Self::Context) -> Self::Result {
        if Some(msg.player) == self.waiting_for_game {
            self.waiting_for_game = None;
        }
    }
}

#[derive(ActixMessage)]
#[rtype(result = "Vec<String>")]
pub struct GetPlayers {}

impl Handler<GetPlayers> for Server {
    type Result = Vec<String>;
    fn handle(&mut self, _msg: GetPlayers, _ctx: &mut Self::Context) -> Self::Result {
        self.users
            .keys()
            .into_iter()
            .map(|name| name.to_string())
            .collect::<Vec<String>>()
    }
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct FindGame(pub Recipient<Message>);

impl Handler<FindGame> for Server {
    type Result = ();
    fn handle(&mut self, msg: FindGame, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(other) = &self.waiting_for_game {
            if other != &msg.0.clone() {
                let game = Game::new([other.clone(), msg.0.clone()]).start();
                other.do_send(Message {
                    inner: crate::message::OutgoingMessage::GameStarted(
                        crate::message::Color::White,
                    ),
                    game: Some(game.clone()),
                });
                msg.0.do_send(Message {
                    inner: crate::message::OutgoingMessage::GameStarted(
                        crate::message::Color::Black,
                    ),
                    game: Some(game),
                });
                self.waiting_for_game = None;
            }
        } else {
            self.waiting_for_game = Some(msg.0);
        }
    }
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct CancelSearch(pub Recipient<Message>);

impl Handler<CancelSearch> for Server {
    type Result = ();
    fn handle(&mut self, msg: CancelSearch, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(other) = &self.waiting_for_game {
            if other == &msg.0 {
                self.waiting_for_game = None;
            }
        }
    }
}
