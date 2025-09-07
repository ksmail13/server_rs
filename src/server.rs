use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::{net::TcpListener, process::exit, rc::Rc, time::Duration};

use nix::libc::close;

use crate::process::process::Process;
use crate::worker::Worker;
use crate::worker::group::WorkerGroup;
use crate::worker::manager::WorkerManager;

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
    listeners: Rc<HashMap<String, TcpListener>>,
    host: String,
    tcp_process: Rc<dyn Process>,
}

impl Worker for TcpWorker {
    fn run(&self) {
        let tcp_listener_opt = self.listeners.get(&self.host);
        if tcp_listener_opt.is_none() {
            return;
        }

        let tcp_listener = tcp_listener_opt.unwrap();
        loop {
            let stream_result = tcp_listener.accept();

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

    fn init(&self) {
        let pid = nix::unistd::getpid();
        let process = &self.tcp_process.name();
        log::trace!(target: "TcpWorker.init", "TcpWorker start[{pid}:{process}]");
        // close unnecessary sockets
        self.listeners
            .iter()
            .filter(|(host, _)| **host != self.host)
            .for_each(|(_, listener)| {
                let fd = listener.as_raw_fd();
                unsafe { close(fd) };
            });
    }

    fn cleanup(&self) {
        let pid = nix::unistd::getpid();
        let process = &self.tcp_process.name();
        log::trace!(target: "TcpWorker.cleanup", "TcpWorker stop[{pid}:{process}]");
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
        let listeners = Rc::new(
            config
                .worker_infos
                .iter()
                .map(|i| {
                    let ip = format!("{}:{}", i.host, i.port);
                    let listener = TcpListener::bind(ip.clone()).unwrap();
                    return (ip, listener);
                })
                .fold(HashMap::new(), |m, pair| {
                    let mut new_m = HashMap::from(m);
                    new_m.insert(pair.0, pair.1);
                    return new_m;
                }),
        );

        let group: Vec<WorkerGroup> = config
            .worker_infos
            .iter()
            .map(|i| {
                return WorkerGroup::new(
                    i.worker,
                    Rc::new(TcpWorker {
                        timeout_ms: config.timeout_ms,
                        listeners: listeners.clone(),
                        host: format!("{}:{}", i.host, i.port),
                        tcp_process: i.process.clone(),
                    }),
                );
            })
            .collect();

        let manager = WorkerManager::new(group);
        let mut group_list = manager.start();

        manager.run(&mut group_list);
    }
}
