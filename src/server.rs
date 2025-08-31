use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::exit,
    rc::Rc,
    time::Duration,
};

use clap::Parser;

use crate::worker::{Worker, WorkerGroup, WorkerManager};

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct ServerArgs {
    #[arg(short, long, default_value_t = 10080)]
    port: u16,
    #[arg(short, long, default_value_t = 10079)]
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
    main_manager: WorkerGroup,
    reserve_manager: WorkerGroup,
}

pub struct TcpWorker {
    timeout_ms: u64,
    tcp_listener: TcpListener,
    tcp_process: Box<dyn Process>,
}

pub trait Process {
    fn process(&self, stream: TcpStream, client_addr: SocketAddr) -> io::Result<(usize, usize)>;
    fn name(&self) -> String {
        return "process".to_string();
    }
}

#[derive(Debug)]
pub struct EchoProcess {
    prefix: Option<String>,
}

impl Process for EchoProcess {
    fn process(&self, mut stream: TcpStream, client: SocketAddr) -> io::Result<(usize, usize)> {
        let pid = nix::unistd::getpid();
        let mut bufs: Vec<u8> = vec![0; 1024];
        let mut all_readed = 0;
        let mut all_writed = 0;
        loop {
            let read_result = stream.read(&mut bufs);

            let echo_result = match read_result {
                Ok(readed) => {
                    if readed == 0 {
                        break;
                    }
                    all_readed += readed;
                    if let Some(prefix) = &self.prefix {
                        let _ = stream.write_fmt(format_args!("{prefix}"));
                    }
                    stream.write(&bufs)
                }
                Err(ref read_err) => {
                    log::error!(target: "MainWorker.process", "Read error: {read_err}");
                    stream.write_fmt(format_args!("{read_err}")).map(|_| 0)
                }
            };

            if echo_result.is_err() {
                break;
            }

            all_writed += echo_result.unwrap();
        }

        log::info!(target:"access log", "{pid} {client} {all_readed} {all_writed}");

        return Ok((all_readed, all_writed));
    }

    fn name(&self) -> String {
        return "EchoProcess".to_string();
    }
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
        let main_worker = config.worker;
        let reserve_worker = config.reserve;

        return Self {
            config,
            main_manager: WorkerGroup::new(main_worker, None),
            reserve_manager: WorkerGroup::new(reserve_worker, None),
        };
    }

    pub fn open_server(&mut self) -> Result<i32, String> {
        loop {
            let config = &self.config;
            let main_connect =
                TcpListener::bind(format!("{}:{}", config.host, config.port)).unwrap();
            let reserve_connect = if self.config.reserve > 0 {
                Some(TcpListener::bind(format!("{}:{}", config.host, config.reserve_port)).unwrap())
            } else {
                None
            };

            let mut group = vec![];
            self.main_manager.set_worker(Some(Rc::new(TcpWorker {
                timeout_ms: config.timeout_ms,
                tcp_listener: main_connect,
                tcp_process: Box::new(EchoProcess { prefix: None }),
            })));
            group.push(&mut self.main_manager);

            if let Some(conn) = reserve_connect {
                self.reserve_manager.set_worker(Some(Rc::new(TcpWorker {
                    timeout_ms: config.timeout_ms,
                    tcp_listener: conn,
                    tcp_process: Box::new(EchoProcess {
                        prefix: Some("reserve: ".to_string()),
                    }),
                })));
                group.push(&mut self.reserve_manager);
            }

            let mut manager = WorkerManager::new(group, None);
            let _ = manager.start();
            loop {
                manager.manage();
            }
        }
    }
}
