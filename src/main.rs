use clap::Parser;
use server::ServerArgs;

use crate::server::Server;

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
    let _ = server.open_server().err();
}
