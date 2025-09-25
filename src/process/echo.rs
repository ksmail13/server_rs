use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream},
};

use crate::process::{Error, Process};

#[derive(Debug)]
pub struct EchoProcess {
    pub prefix: Option<String>,
}

impl Process for EchoProcess {
    fn process(&self, mut stream: TcpStream, client: SocketAddr) -> Result<(usize, usize), Error> {
        let pid = nix::unistd::getpid();
        let mut all_readed = 0;
        let mut all_writed = 0;

        let mut bufs: Vec<u8> = vec![0; 1024];

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
                    stream.write(&bufs[..readed])
                }
                Err(ref read_err) => {
                    let err_kind = read_err.kind();
                    if err_kind == ErrorKind::WouldBlock {
                        Ok(0)
                    } else {
                        log::error!(target: "MainWorker.process", "Read error: {err_kind}");
                        stream.write_fmt(format_args!("{read_err}")).map(|_| 0)
                    }
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
