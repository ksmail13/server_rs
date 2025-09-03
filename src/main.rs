use clap::Parser;
use server::ServerArgs;

use crate::server::Server;

mod process;
mod server;
mod worker;

fn main() {
    colog::basic_builder()
        .default_format()
        .filter_level(log::LevelFilter::Trace)
        .format_line_number(true)
        .write_style(env_logger::fmt::WriteStyle::Always)
        .init();

    let args = ServerArgs::parse();
    println!("Open Server: {:?}", args);

    let mut server = Server::new(args);
    server.open_server();
}
