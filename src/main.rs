use std::rc::Rc;

use args::Args;
use clap::Parser;

use crate::{
    process::echo_process::EchoProcess,
    server::Server,
    server::{ServerArgs, WorkerInfo},
};

mod args;
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

    let arg = Args::parse();
    println!("Open Server: {:?}", arg);

    let worker_infos = vec![
        WorkerInfo {
            host: arg.host.clone(),
            port: arg.port,
            worker: arg.worker,
            process: Rc::new(EchoProcess { prefix: None }),
        },
        WorkerInfo {
            host: arg.host.clone(),
            port: arg.reserve_port,
            worker: arg.reserve,
            process: Rc::new(EchoProcess {
                prefix: Some("Second: ".to_string()),
            }),
        },
    ];

    let mut server = Server::new(ServerArgs {
        worker_infos: worker_infos,
        timeout_ms: arg.timeout_ms,
    });
    server.open_server();
}
