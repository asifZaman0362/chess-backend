use std::time::{Duration, Instant};

use crate::game::{Game, MakeMove, MoveError};
use crate::message::{
    ClientMessage::{self, *},
    Login, Logout,
};
use crate::server::Server;
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, Running, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};
use log::{info, warn};
use serde::Serialize;
use serde_json::to_string;
use ws::Message::{Close, Ping, Text};

static HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
static HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ChessClient {
    username: Option<String>,
    heartbeat: Instant,
    server: Addr<Server>,
    game: Option<Addr<Game>>,
}

impl ChessClient {
    pub fn new(server: Addr<Server>) -> Self {
        Self {
            username: None,
            heartbeat: Instant::now(),
            server,
            game: None,
        }
    }

    fn handle_message(&mut self, message: &str, ctx: &mut WebsocketContext<Self>) {
        let addr = ctx.address();
        if let Ok(message) = serde_json::from_str::<ClientMessage>(message) {
            match message {
                Login(username) => self.server.do_send(Login {
                    username,
                    client: ctx.address(),
                }),
                Enqueue => {}
                Dequeue => {}
                LeaveGame => {}
                MakeMove(move_details) => match &self.game {
                    Some(game) => game.do_send(MakeMove {
                        move_details,
                        player: addr,
                    }),
                    None => ctx.text(
                        to_string(&Err::<(), MoveError>(crate::game::MoveError::NotInGame))
                            .unwrap(),
                    ),
                },
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
                } else {
                    info!("Client timeout! Disconnecting!");
                    ctx.stop();
                }
            }
        });
    }
}

impl Actor for ChessClient {
    type Context = WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        match &self.username {
            Some(username) => self.server.do_send(Logout {
                username: username.to_string(),
            }),
            None => {}
        };
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChessClient {
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

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct TakePiece {
    pub at: usize,
}

impl Handler<TakePiece> for ChessClient {
    type Result = ();
    fn handle(&mut self, msg: TakePiece, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(to_string(&msg).unwrap());
    }
}
