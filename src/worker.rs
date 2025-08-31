use std::{process::exit, rc::Rc};

use nix::{
    Error,
    errno::Errno,
    sys::{
        signal::{
            Signal::{self},
            kill,
        },
        wait::{WaitStatus, wait},
    },
    unistd::{ForkResult, Pid, fork},
};

pub trait Worker {
    fn run(&self);
}

pub struct WorkerGroup {
    count: u32,
    stopped: bool,
    pids: Vec<Pid>,
    worker: Option<Rc<dyn Worker>>,
}

impl Drop for WorkerGroup {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl WorkerGroup {
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

    fn start(&mut self) -> Result<(), &str> {
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

    fn stop(&mut self) -> Result<(), &str> {
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
        self.stopped = true;
        return Ok(());
    }

    fn kill_child(&mut self, pid: Pid) -> Result<(), Errno> {
        return kill(pid, Some(Signal::SIGINT));
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
            Err(err) => Err(err),
        };
    }

    pub fn is_this_group(&self, pid: Pid) -> bool {
        return self.pids.iter().any(|p| p.as_raw() == pid.as_raw());
    }

    pub fn collect_and_fork(&mut self, pid: Pid) -> Result<Pid, Error> {
        if let Some(idx) = self.pids.iter().position(|p| p.as_raw() == pid.as_raw()) {
            self.pids.remove(idx);
        }

        return self.fork_child();
    }

    pub fn set_worker(&mut self, worker: Option<Rc<dyn Worker>>) {
        self.worker = worker;
    }
}

pub enum WaitError {
    ErrorExit(Pid, i32),
    WaitFailed(Errno),
    NotExited(WaitStatus),
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            WaitError::ErrorExit(pid, exit_code) => {
                f.write_fmt(format_args!("ErrorExit({pid}, {exit_code})"))
            }
            WaitError::WaitFailed(errno) => f.write_fmt(format_args!("WaitFailed({errno})")),
            WaitError::NotExited(wait_status) => {
                f.write_fmt(format_args!("NotExited({wait_status:?})"))
            }
        };
    }
}

pub struct WorkerCleaner;

impl WorkerCleaner {
    pub fn wait(&self) -> Result<Pid, WaitError> {
        let wait_result = wait();
        return match wait_result {
            Ok(WaitStatus::Exited(pid, excode)) => {
                if excode == 0 {
                    Ok(pid)
                } else {
                    Err(WaitError::ErrorExit(pid, excode))
                }
            }
            Ok(ws) => Err(WaitError::NotExited(ws)),
            Err(e) => Err(WaitError::WaitFailed(e)),
        };
    }
}

pub struct WorkerManager<'a> {
    groups: Vec<&'a mut WorkerGroup>,
    cleaner: WorkerCleaner,
}

impl<'a> WorkerManager<'a> {
    pub fn new(groups: Vec<&'a mut WorkerGroup>, cleaner: Option<WorkerCleaner>) -> Self {
        return Self {
            groups: groups,
            cleaner: if let Some(c) = cleaner {
                c
            } else {
                WorkerCleaner {}
            },
        };
    }

    pub fn start(&mut self) -> Result<(), &str> {
        for g in &mut self.groups {
            let start_result = g.start();
            if let Err(err) = start_result {
                log::error!("start failed: {err}");
            }
        }
        return Ok(());
    }

    pub fn manage(&mut self) {
        loop {
            match self.cleaner.wait() {
                Ok(pid) => {
                    for g in &mut self.groups {
                        if !g.is_this_group(pid) {
                            continue;
                        }
                        let _ = g.collect_and_fork(pid);
                    }
                }
                Err(WaitError::WaitFailed(e)) => {
                    if e != Errno::ECHILD {
                        log::error!(target: "WorkerManager.manage", "wait failed {e}");
                    }
                }
                Err(e) => {
                    log::error!(target: "WorkerManager.manage", "wait error {e}");
                }
            }
        }
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
        let mut group = WorkerGroup::new(5, Some(Rc::new(SleepWorker {})));
        let mut manager = WorkerManager::new(vec![&mut group], Some(WorkerCleaner {}));
        let _ = manager.start();
        let pid = getpid();
        log::debug!(target: "test_manager", "start {pid}");

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
