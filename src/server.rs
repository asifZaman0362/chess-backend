use actix::{Actor, Context, Handler, Message as ActixMessage, Recipient};
use std::collections::HashMap;

use crate::{
    chessclient::Message,
    game::Game,
    message::{Login, Logout},
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
    type Result = Result<(), ()>;
    fn handle(&mut self, msg: Login, _ctx: &mut Self::Context) -> Self::Result {
        if self.users.get(&msg.username).is_some() {
            Err(())
        } else {
            self.users.insert(msg.username, msg.client);
            Ok(())
        }
    }
}

impl Handler<Logout> for Server {
    type Result = ();
    fn handle(&mut self, msg: Logout, _ctx: &mut Self::Context) -> Self::Result {
        self.users.remove(&msg.username);
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
            let game = Game::new([other.clone(), msg.0.clone()]).start();
            other.do_send(Message {
                inner: crate::message::OutgoingMessage::GameStarted,
                game: Some(game.clone()),
            });
            msg.0.do_send(Message {
                inner: crate::message::OutgoingMessage::GameStarted,
                game: Some(game),
            });
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
