use std::{net::TcpListener, process::exit, rc::Rc, time::Duration};

use crate::{
    process::process::Process,
    worker::{Worker, WorkerGroup, WorkerManager},
};

pub struct WorkerInfo {
    pub host: String,
    pub port: u16,
    pub worker: u32,
    pub process: Rc<dyn Process>,
}

pub struct ServerArgs {
    pub worker_infos: Vec<WorkerInfo>,
    pub timeout_ms: u64,
}

struct TcpWorker {
    timeout_ms: u64,
    tcp_listener: TcpListener,
    tcp_process: Rc<dyn Process>,
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

pub struct Server {
    config: ServerArgs,
}

impl Server {
    pub fn new(config: ServerArgs) -> Self {
        return Self { config };
    }

    pub fn open_server(&mut self) {
        let config = &self.config;

        let group: Vec<WorkerGroup> = config
            .worker_infos
            .iter()
            .map(|i| {
                let listener = TcpListener::bind(format!("{}:{}", i.host, i.port)).unwrap();
                return WorkerGroup::new(
                    i.worker,
                    Rc::new(TcpWorker {
                        timeout_ms: config.timeout_ms,
                        tcp_listener: listener,
                        tcp_process: i.process.clone(),
                    }),
                );
            })
            .collect();

        let manager = WorkerManager::new(group, None);
        let mut group_list = manager.start();

        manager.run(&mut group_list);
    }
}
