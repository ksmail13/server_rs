use args::Args;
use clap::Parser;
use std::io::Write;
use std::rc::Rc;

use crate::{
    http::{handler::Handler, http::Http1, response::HeaderSetter, value::HttpResponseCode},
    server::{Server, ServerArgs, WorkerInfo},
};

mod args;
mod http;
mod process;
mod server;
mod worker;

struct SimpleHandler;

impl Handler for SimpleHandler {
    fn handle(&self, req: &mut http::request::HttpRequest, res: &mut http::response::HttpResponse) {
        res.set_response_code(HttpResponseCode::Ok);

        if let Err(e) = writeln!(res, "response") {
            log::error!("error {}", e);
        }

        for (k, v) in req.header().iter() {
            if let Err(e) = writeln!(res, "{}: {}", k, v.join(";")) {
                log::error!("error {}", e);
            }
        }

        res.set_header("Content-Type", "text/plain".to_string());
    }
}

fn main() {
    colog::basic_builder()
        .default_format()
        .filter_level(log::LevelFilter::Info)
        .format_line_number(true)
        .write_style(env_logger::fmt::WriteStyle::Always)
        .init();

    let arg = Args::parse();
    log::info!("server_rs: {:?}", arg);

    let worker_infos = vec![WorkerInfo {
        host: arg.host.clone(),
        port: arg.port,
        worker: arg.worker,
        process: Rc::new(Http1::new(SimpleHandler)),
    }];

    let mut server = Server::new(ServerArgs {
        worker_infos: worker_infos,
        timeout_ms: arg.timeout_ms,
    });
    server.open_server();
}
