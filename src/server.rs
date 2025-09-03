use std::{net::TcpListener, process::exit, rc::Rc, time::Duration};

use clap::Parser;

use crate::{
    process::{echo_process::EchoProcess, process::Process},
    worker::{Worker, WorkerGroup, WorkerManager},
};

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct ServerArgs {
    #[arg(short, long, default_value_t = 10080)]
    port: u16,
    #[arg(long, default_value_t = 10079)]
    reserve_port: u16,
    #[arg(long, default_value_t = DEFAULT_HOST.to_string())]
    host: String,
    #[arg(short, long, default_value_t = 60)]
    worker: u32,
    #[arg(short, long, default_value_t = 4)]
    reserve: u32,
    #[arg(short, long, default_value_t = 500)]
    timeout_ms: u64,
}

pub struct Server {
    config: ServerArgs,
}

struct TcpWorker {
    timeout_ms: u64,
    tcp_listener: TcpListener,
    tcp_process: Box<dyn Process>,
}

impl Worker for TcpWorker {
    fn run(&self) {
        let pid = nix::unistd::getpid();
        let process = &self.tcp_process.name();
        log::trace!(target: "TcpWorker.run", "TcpWorker start[{pid}:{process}]");
        loop {
            let stream_result = self.tcp_listener.accept();

            match stream_result {
                Ok((stream, client)) => {
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(self.timeout_ms)));
                    let _ = self.tcp_process.process(stream, client);
                }
                Err(err) => {
                    log::error!("Accept failed {err}");
                    exit(1);
                }
            }
        }
    }
}

impl Server {
    pub fn new(config: ServerArgs) -> Self {
        return Self { config };
    }

    pub fn open_server(&mut self) {
        let config = &self.config;
        let main_connect = TcpListener::bind(format!("{}:{}", config.host, config.port)).unwrap();
        let reserve_connect = if self.config.reserve > 0 {
            Some(TcpListener::bind(format!("{}:{}", config.host, config.reserve_port)).unwrap())
        } else {
            None
        };

        let mut group = vec![];

        let main_manager = WorkerGroup::new(
            self.config.worker,
            Rc::new(TcpWorker {
                timeout_ms: config.timeout_ms,
                tcp_listener: main_connect,
                tcp_process: Box::new(EchoProcess { prefix: None }),
            }),
        );
        group.push(main_manager);

        if let Some(conn) = reserve_connect {
            let reserve_manager = WorkerGroup::new(
                self.config.reserve,
                Rc::new(TcpWorker {
                    timeout_ms: config.timeout_ms,
                    tcp_listener: conn,
                    tcp_process: Box::new(EchoProcess {
                        prefix: Some("reserve: ".to_string()),
                    }),
                }),
            );
            group.push(reserve_manager);
        }

        let manager = WorkerManager::new(group, None);
        let mut group_list = manager.start();

        manager.run(&mut group_list);
    }
}
