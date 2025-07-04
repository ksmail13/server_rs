pub mod server;

use clap::Parser;
use server::ServerArgs;

use crate::server::Server;

fn main() {
    let args = ServerArgs::parse();
    println!("Open Server: {:?}", args);

    let server = Server::init(args);
}
