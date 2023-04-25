use std::time::{Duration, Instant};

use crate::codec::{FrameCodec, FrameError};
use crate::game::{ForfeitGame, Game, MakeMove, MoveError};
use crate::message::OutgoingMessage;
use crate::message::{
    ClientMessage::{self, *},
    Login, Logout,
};
use crate::server::{CancelSearch, FindGame, Server};
use actix::io::{FramedWrite, WriteHandler};
use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message as ActixMessage, Running,
    StreamHandler,
};
use actix_web_actors::ws::{self, WebsocketContext};
use log::{info, warn};
use serde_json::to_string;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
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
        let addr = ctx.address().recipient();
        if let Ok(message) = serde_json::from_str::<ClientMessage>(message) {
            match message {
                Login(username) => self.server.do_send(Login {
                    username,
                    client: addr,
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

pub struct TcpClient {
    username: Option<String>,
    heartbeat: Instant,
    server: Addr<Server>,
    game: Option<Addr<Game>>,
    framed: FramedWrite<OutgoingMessage, WriteHalf<TcpStream>, FrameCodec>,
}

impl StreamHandler<Result<ClientMessage, FrameError>> for TcpClient {
    fn handle(&mut self, item: Result<ClientMessage, FrameError>, ctx: &mut Self::Context) {
        match item {
            Ok(message) => self.handle_message(message, ctx),
            Err(_) => ctx.stop(),
        }
    }
}

impl TcpClient {
    pub fn new(
        srv: Addr<Server>,
        writer: FramedWrite<OutgoingMessage, WriteHalf<TcpStream>, FrameCodec>,
    ) -> Self {
        TcpClient {
            username: None,
            heartbeat: Instant::now(),
            server: srv,
            game: None,
            framed: writer,
        }
    }
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
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

    fn handle_message(&mut self, message: ClientMessage, ctx: &mut <Self as Actor>::Context) {
        let addr = ctx.address().recipient();
        match message {
            Login(username) => self.server.do_send(Login {
                username,
                client: addr,
            }),
            Enqueue => {
                self.server.do_send(FindGame(addr));
            }
            Dequeue => {
                self.server.do_send(CancelSearch(addr));
            }
            LeaveGame => {
                if let Some(game) = &self.game {
                    game.do_send(ForfeitGame(addr));
                }
            }
            MakeMove(move_details) => match &self.game {
                Some(game) => game.do_send(MakeMove {
                    move_details,
                    player: addr,
                }),
                None => self.framed.write(OutgoingMessage::Result(Err(
                    crate::game::MoveError::NotInGame,
                ))),
            },
            PlayAgain => {}
        }
    }
}

impl Actor for TcpClient {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl WriteHandler<FrameError> for TcpClient {}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Message {
    pub inner: OutgoingMessage,
    pub game: Option<Addr<Game>>,
}

impl Handler<Message> for ChessClient {
    type Result = ();
    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) -> Self::Result {
        match msg.inner {
            OutgoingMessage::GameStarted => {
                self.game = msg.game;
            }
            OutgoingMessage::WinGame(_) => {
                self.game = None;
            }
            OutgoingMessage::LoseGame(_) => {
                self.game = None;
            }
            _ => (),
        }
        ctx.text(to_string(&msg.inner).unwrap());
    }
}

impl Handler<Message> for TcpClient {
    type Result = ();
    fn handle(&mut self, msg: Message, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(game) = msg.game {
            self.game = Some(game);
        }
        self.framed.write(msg.inner);
    }
}
