use actix::Addr;
use actix_web::{
    get,
    web::{Data, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;

mod chessclient;
mod game;
mod message;
mod server;

use chessclient::ChessClient;
use server::Server;

#[get("/game")]
async fn game_stream(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Server>>,
) -> Result<HttpResponse, Error> {
    let server = srv.get_ref().clone();
    ws::start(ChessClient::new(server), &req, stream)
}

#[actix::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || App::new())
        .bind(("localhost", 3000))?
        .run()
        .await
}
