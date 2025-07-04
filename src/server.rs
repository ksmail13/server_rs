use std::{io::{Error, Read, Write}, net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream}, pin::Pin, task::{Context, Poll}};
use clap::Parser;
use hyper::{rt::ReadBufCursor, server::conn::http1};

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct ServerArgs {
    #[arg(short, long, default_value_t = 10080)]
    port: u16,
    #[arg(long, default_value_t = DEFAULT_HOST.to_string())]
    host: String,
    #[arg(short, long, default_value_t = 60)]
    worker: u32,
    #[arg(short, long, default_value_t = 4)]
    reserve: u32,
}

#[derive(Debug)]
pub struct Server {
    config: ServerArgs
}

impl Server {
    pub fn init(arg: ServerArgs) -> Self {
        return Self {
            config: arg
        }
    }

    pub fn run_server(&self) -> Option<bool> {
        let host: [u8; 4] = self.config.host.split_terminator('.')
            .map(|p|  p.parse::<u8>().expect("must ip v4 value"))
            .collect::<Vec<u8>>()
            .as_slice().try_into().unwrap();
        
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(host[0], host[1], host[2], host[3])), self.config.port);

        let listener = TcpListener::bind(listen_addr).expect("open TCP failed");

        loop {
            match listener.accept() {
                Ok(conn) => {
                    let req = Request { tcp_stream : conn.0, client: conn.1 };
                    http1::Builder::new().serve_connection(req, service)
                },
                Err(err) => {
                    return Option::None;
                }
            }
        }
    }
}

struct Request {
    tcp_stream: TcpStream,
    client: SocketAddr,
}

impl hyper::rt::Read for Request {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), Error>> {
        let mut tcp_stream = &self.tcp_stream;

        let mut buffer: [u8; 1024] = [0; 1024];
        let result = tcp_stream.read(&mut buffer);

        match result {
            Ok(readed) => {
                if readed > 0 {
                    buf.put_slice(&buffer);
                    return Poll::Pending;
                } else if readed == 0 {
                    return Poll::Ready(Ok(()))
                }
            }
            Err(error) => {
                return Poll::Ready(Err(error))
            }
        }

        return Poll::Pending;
    }
}


impl hyper::rt::Write for Request {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut tcp_stream = &self.tcp_stream;
        return Poll::Ready(tcp_stream.write(buf));
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let mut tcp_stream = &self.tcp_stream;
        return Poll::Ready(tcp_stream.flush())
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let tcp_stream = &self.tcp_stream;
        return Poll::Ready(tcp_stream.shutdown(std::net::Shutdown::Write));
    }
}

impl hyper::rt::Unpin for Request {

}