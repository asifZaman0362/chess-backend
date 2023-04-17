use std::time::{Duration, Instant};

use crate::message::{
    ClientMessage::{self, *},
    Login, Logout,
};
use crate::server::Server;
use actix::{Actor, ActorContext, Addr, AsyncContext, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};
use log::{info, warn};
use ws::Message::{Close, Ping, Text};

static HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
static HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ChessClient {
    username: Option<String>,
    heartbeat: Instant,
    server: Addr<Server>,
}

impl ChessClient {
    pub fn new(server: Addr<Server>) -> Self {
        Self {
            username: None,
            heartbeat: Instant::now(),
            server,
        }
    }

    fn handle_message(&mut self, message: &str, ctx: &mut WebsocketContext<Self>) {
        if let Ok(message) = serde_json::from_str::<ClientMessage>(message) {
            match message {
                Login(username) => self.server.do_send(Login {
                    username,
                    client: ctx.address(),
                }),
                Enqueue => {}
                Dequeue => {}
                LeaveGame => {}
                PlayAgain => {}
            }
        }
    }

    fn hb(&self, ctx: &mut WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > HEARTBEAT_TIMEOUT {
                if act.username.is_some() {
                    let username = act.username.clone().unwrap();
                    info!("Client {username} timeout! Disconnecting!");
                    act.server.do_send(Logout { username });
                    ctx.stop();
                }
            }
        });
    }
}

impl Actor for ChessClient {
    type Context = WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChessClient {
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }

    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(Text(text)) => {
                self.heartbeat = Instant::now();
                self.handle_message(&text, ctx);
            }
            Ok(Ping(message)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&message);
            }
            Ok(Close(reason)) => ctx.close(reason),
            _ => {
                self.heartbeat = Instant::now();
                warn!("Received unrecognised message!");
            }
        }
    }
}
