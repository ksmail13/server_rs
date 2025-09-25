use std::{
    fmt::Display,
    net::{SocketAddr, TcpStream},
};

pub mod echo;

pub enum Error {
    ParseFail(String),
}

pub trait Process {
    #[allow(dead_code)]
    fn process(&self, stream: TcpStream, client_addr: SocketAddr) -> Result<(usize, usize), Error>;

    fn name(&self) -> String {
        return "process".to_string();
    }
}

impl Display for dyn Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str(&self.name());
    }
}
