use std::{
    fmt::Display,
    io,
    net::{SocketAddr, TcpStream},
};

pub trait Process {
    fn process(&self, stream: TcpStream, client_addr: SocketAddr) -> io::Result<(usize, usize)>;
    fn name(&self) -> String {
        return "process".to_string();
    }
}

impl Display for dyn Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str("process");
    }
}
