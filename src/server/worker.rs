use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::{net::TcpListener, process::exit, rc::Rc, time::Duration};

use nix::libc::close;
use nix::{
    sys::signal::{
        SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal, sigaction, sigprocmask,
    },
    unistd::getpid,
};

use crate::process::Process;
use crate::worker::Worker;

static mut RUNNING: bool = true;

extern "C" fn tcpworker_exit_signal_handler(sig_no: i32) {
    unsafe { RUNNING = false };
    let pid = getpid();
    log::trace!(target:"tcpworker_exit_signal_handler", "{sig_no} received in TcpWorker[{pid}]");
}

pub struct TcpWorker {
    pub timeout_ms: u64,
    pub listeners: Rc<HashMap<String, TcpListener>>,
    pub host: String,
    pub tcp_process: Rc<dyn Process>,
}

impl Worker for TcpWorker {
    fn run(&self) {
        let tcp_listener_opt = self.listeners.get(&self.host);
        if tcp_listener_opt.is_none() {
            return;
        }

        let tcp_listener = tcp_listener_opt.unwrap();

        while unsafe { RUNNING } {
            let stream_result = tcp_listener.accept();

            match stream_result {
                Ok((stream, client)) => {
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(self.timeout_ms)));
                    let _ = self.tcp_process.process(stream, client);
                }
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::WouldBlock => (),
                        _ => {
                            log::error!(target: "TcpWorker::run", "Accept failed: {err}");
                            exit(1);
                        }
                    };
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

        let sigaction = unsafe {
            sigaction(
                Signal::SIGTERM,
                &SigAction::new(
                    SigHandler::Handler(tcpworker_exit_signal_handler),
                    SaFlags::empty(),
                    SigSet::empty(),
                ),
            )
        };

        if let Err(e) = sigaction {
            log::error!(target: "WorkerManager.run", "sigaction failed: {e}");
        }

        let mut sig_set = SigSet::empty();
        sig_set.add(Signal::SIGINT);
        if let Err(e) = sigprocmask(SigmaskHow::SIG_SETMASK, Some(&sig_set), None) {
            log::error!(target: "WorkerManager.run", "sigprocmast failed: {e}");
        }
    }

    fn cleanup(&self) {
        let pid = nix::unistd::getpid();
        let process = &self.tcp_process.name();
        log::trace!(target: "TcpWorker.cleanup", "TcpWorker stop[{pid}:{process}]");
        if let Some(listener) = self.listeners.get(&self.host) {
            unsafe { close(listener.as_raw_fd()) };
        }
    }
}
