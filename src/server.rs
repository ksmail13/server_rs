use std::{net::{SocketAddrV4, TcpListener}};

use clap::Parser;
use nix::libc::fork;
use crate::handler::Worker;

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct ServerArgs {
    #[arg(short, long, default_value_t = 10080)]
    port: u16,
    #[arg(short, long, default_value_t = 10080)]
    reserve_port: u16,
    #[arg(long, default_value_t = DEFAULT_HOST.to_string())]
    host: String,
    #[arg(short, long, default_value_t = 60)]
    worker: u32,
    #[arg(short, long, default_value_t = 4)]
    reserve: u32,
}

pub struct Server {
    config: ServerArgs,
    main_handler: Box<dyn Worker>,
    reserve_handler: Box<dyn Worker>,
}

impl std::ops::Deref for Server {
    type Target = Box<dyn Worker>;

    fn deref(&self) -> &Self::Target {
        &self.main_handler
    }
}

impl Server {
    pub fn open_server(&self) -> Result<i32, String> {
        let main_addr: Result<SocketAddrV4, _> = format!("{}:{}", self.config.host, self.config.port).parse();
        let reserve_addr: Result<SocketAddrV4, _> = format!("{}:{}", self.config.host, self.config.reserve_port).parse();

        if let Err(err) = main_addr {
            return Err(err.to_string());
        }

        if let Err(err) = reserve_addr {
            return Err(err.to_string());
        }

        let main = TcpListener::bind(main_addr.unwrap());
        let reserve = TcpListener::bind(reserve_addr.unwrap());

        let mut main_childs: Vec<i32> = vec![];
        let mut reserve_childs: Vec<i32> = vec![];

        for _ in 0..self.config.worker {
            let pid = unsafe { nix::libc::fork() };

            match pid {
                0 => {self.main_handler.as_ref().run()}
                _ => {main_childs.push(pid)}
            }
        }

        for _ in 0..self.config.reserve {
            let pid = unsafe {nix::libc::fork()};
        }


        return Ok(0)
    }
}