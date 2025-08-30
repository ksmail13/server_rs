use std::{process::exit, rc::Rc, vec};

use nix::{
    errno::Errno,
    sys::{
        signal::{
            Signal::{self},
            kill,
        },
        wait::{WaitStatus, waitpid},
    },
    unistd::{ForkResult, Pid, fork},
};

pub trait Worker {
    fn run(&self);
}

pub struct WorkerManager {
    count: u32,
    stopped: bool,
    pids: Vec<Pid>,
    worker: Option<Rc<dyn Worker>>,
}

impl Drop for WorkerManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl WorkerManager {
    pub fn new(count: u32, worker: Option<Rc<dyn Worker>>) -> Self {
        let pids = Vec::new();

        let new_struct = Self {
            count: count,
            stopped: false,
            pids: pids,
            worker: worker,
        };

        return new_struct;
    }

    fn fork_child(&mut self) -> Result<Pid, Errno> {
        if self.worker.is_none() {
            return Err(Errno::EINVAL);
        }

        return match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                self.pids.push(child);
                Ok(child)
            }
            Ok(ForkResult::Child) => {
                if let Some(w) = &self.worker {
                    w.run();
                }
                exit(0);
            }
            Err(err) => {
                log::warn!(target: "WorkerManager.start", "fork failed: {err}");
                Err(err)
            }
        };
    }

    pub fn start(&mut self) -> Result<(), &str> {
        let mut remains = self.count;
        let threshold = 5;
        for _ in 0..threshold {
            for _ in 0..remains {
                if let Ok(pid) = self.fork_child() {
                    self.pids.push(pid);
                    remains -= 1;
                }
            }
            if remains == 0 {
                return Ok(());
            }
        }
        return Err("Failed run workers");
    }

    pub fn stop(&mut self) -> Result<(), &str> {
        if self.stopped {
            return Ok(());
        }

        let pids: Vec<Pid> = self.pids.iter().map(|p| p.clone()).collect();
        for pid in pids {
            let wait_result = self.kill_child(pid);
            if let Err(err) = wait_result {
                if err != Errno::ECHILD && err != Errno::ESRCH {
                    log::error!(target: "WorkerManager.stop", "child kill & wait failed: {err}");
                }
            }
        }
        self.wait_child();
        self.stopped = true;
        return Ok(());
    }

    fn kill_child(&mut self, pid: Pid) -> Result<(), Errno> {
        return kill(pid, Some(Signal::SIGINT));
    }

    fn wait_child(&mut self) -> (usize, Vec<Errno>) {
        let mut waited_process: Vec<Pid> = vec![];
        let mut errors: Vec<(Pid, Errno)> = vec![];

        let childs = self.pids.clone();

        for p in childs {
            let wait_result = waitpid(p, None);

            match wait_result {
                Ok(WaitStatus::Exited(pid, _)) => waited_process.push(pid),
                Ok(ws) => log::trace!("{ws:?}"),
                Err(err) => errors.push((p, err)),
            }
        }

        for wp in &waited_process {
            if let Some(idx) = self.pids.iter().position(|p| p.as_raw() == wp.as_raw()) {
                self.pids.remove(idx);
            }
        }

        let mut ret_err: Vec<Errno> = vec![];

        for (pid, err) in errors {
            if err == Errno::ECHILD {
                if let Some(idx) = self.pids.iter().position(|p| p.as_raw() == pid.as_raw()) {
                    self.pids.remove(idx);
                }
            } else {
                ret_err.push(err);
            }
        }

        return (waited_process.len(), ret_err);
    }

    pub fn manage(&mut self) {
        let (waited, fails) = self.wait_child();
        fails
            .iter()
            .filter(|e| **e != Errno::EINVAL)
            .for_each(|e| log::error!("wait failed {e}"));
        if waited > 0 {
            log::debug!("wait {waited} processes");
        }
        for _ in 0..waited {
            let _ = self.fork_child();
        }
    }

    pub fn set_worker(&mut self, worker: Option<Rc<dyn Worker>>) {
        self.worker = worker;
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use nix::{
        sys::{
            signal::{SigEvent, SigevNotify},
            timer::{Expiration::OneShot, Timer, TimerSetTimeFlags},
        },
        time::ClockId,
        unistd::getpid,
    };

    use super::*;

    struct SleepWorker {}

    impl Worker for SleepWorker {
        fn run(&self) {
            let getpid = getpid();
            let mut sum = 0;
            for i in 0..10000 {
                sum += i;
            }

            log::info!(target: "SleepWorker.run", "process {getpid} over : {sum}");
        }
    }

    #[test]
    fn test_manager() {
        colog::basic_builder()
            .default_format()
            .filter_level(log::LevelFilter::Trace)
            .format_line_number(true)
            .write_style(env_logger::fmt::WriteStyle::Always)
            .init();
        let mut manager = WorkerManager::new(5, Some(Rc::new(SleepWorker {})));
        let pid = getpid();
        log::debug!(target: "test_manager", "start {pid}");
        let _ = manager.start();

        let mut timer = Timer::new(
            ClockId::CLOCK_MONOTONIC,
            SigEvent::new(SigevNotify::SigevSignal {
                signal: Signal::SIGINT,
                si_value: 0,
            }),
        )
        .unwrap();
        let res = timer.set(
            OneShot(Duration::from_millis(5000).into()),
            TimerSetTimeFlags::empty(),
        );

        if let Err(err) = res {
            log::error!(target: "test_manager", "set timer failed {err}");
        }
        loop {
            manager.manage();
        }
    }
}
