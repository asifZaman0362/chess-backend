use actix::{Actor, Addr, Context, Handler, Message};
use std::collections::HashMap;

use crate::{
    chessclient::ChessClient,
    message::{Login, Logout},
};

pub struct Server {
    users: HashMap<String, Addr<ChessClient>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
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

#[derive(Message)]
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
