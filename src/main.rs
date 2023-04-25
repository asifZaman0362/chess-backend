use actix::{io::FramedWrite, spawn, Actor, Addr, StreamHandler};
use actix_web::{
    get,
    http::StatusCode,
    middleware::Logger,
    web::{Data, Payload},
    App, Error, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use actix_web_actors::ws;
use codec::FrameCodec;
use env_logger;

mod chessclient;
mod codec;
mod game;
mod message;
mod server;

use chessclient::{ChessClient, TcpClient};
use serde_json::to_string;
use server::Server;
use tokio::{io::split, net::TcpListener};
use tokio_util::codec::FramedRead;

#[get("/game")]
async fn game_stream(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Server>>,
) -> Result<HttpResponse, Error> {
    let server = srv.get_ref().clone();
    log::info!("connected to stream!");
    ws::start(ChessClient::new(server), &req, stream)
}

#[get("/players")]
async fn get_players(_req: HttpRequest, srv: Data<Addr<Server>>) -> impl Responder {
    let result = srv.send(server::GetPlayers {}).await;
    let string = match result {
        Ok(players) => to_string(&players).unwrap(),
        Err(_) => "empty".to_string(),
    };
    HttpResponseBuilder::new(StatusCode::OK).body(string)
}

fn start_tcp_server(srv: Addr<Server>) {
    spawn(async move {
        log::info!("Started tcp server at 127.0.0.1:9000");
        let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            let server = srv.clone();
            TcpClient::create(|ctx| {
                let (r, w) = split(stream);
                TcpClient::add_stream(FramedRead::new(r, FrameCodec), ctx);
                TcpClient::new(server, FramedWrite::new(w, FrameCodec, ctx))
            });
        }
    });
}

#[actix::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let srv = Server::new().start();
    start_tcp_server(srv.clone());
    log::info!("Started at http://localhost:3000");
    HttpServer::new(move || {
        App::new()
            .service(game_stream)
            .service(get_players)
            .app_data(Data::new(srv.clone()))
            .wrap(Logger::default())
    })
    .bind(("localhost", 3000))?
    .workers(2)
    .run()
    .await
}
