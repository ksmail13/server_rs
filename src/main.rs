use clap::Parser;
use server::ServerArgs;

use crate::server::Server;
use crate::runner::Runner;

pub mod server;
pub mod runner;

fn main() {
    let args = ServerArgs::parse();
    let runner = Runner {};
    println!("Open Server: {:?}", args);

    let server = Server::init(args, runner);

    server.run_server();
}
