use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
};

use crate::process::process::Process;

#[derive(Debug)]
pub struct EchoProcess {
    pub prefix: Option<String>,
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
