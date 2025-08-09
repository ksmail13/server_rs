use clap::Parser;
use server::ServerArgs;

pub mod runner;
pub mod server;

fn main() {
    let args = ServerArgs::parse();

    println!("Open Server: {:?}", args);
}
