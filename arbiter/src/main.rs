use coap::Server;
use tokio::sync::mpsc::channel;

use self::{request_handler::RequestHandler, state::run_state_loop};

mod request;
mod request_handler;
mod state;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:5683";
    let server = Server::new_udp(addr).unwrap();
    println!("Server up on {addr}");

    let (tx, rx) = channel(1000);

    let state_handle = tokio::spawn(async move { run_state_loop(rx).await });

    server.run(RequestHandler::new(tx)).await.unwrap();

    state_handle.await.unwrap();
}
