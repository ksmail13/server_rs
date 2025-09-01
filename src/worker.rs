use std::rc::Rc;

use nix::{Error, errno::Errno, sys::wait::WaitStatus, unistd::Pid};

pub trait Worker {
    fn run(&self);
}

pub struct WorkerGroup {
    count: u32,
    worker: Option<Rc<dyn Worker>>,
}

impl WorkerGroup {
    pub fn new(count: u32, worker: Option<Rc<dyn Worker>>) -> Self {
        return Self {
            count: count,
            worker: worker,
        };
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

mod helper {
    use std::process::exit;

    use nix::{
        errno::Errno,
        sys::wait::{WaitStatus, wait},
        unistd::{ForkResult, Pid, fork},
    };

    use crate::worker::{self, WorkerGroup};

    pub struct WorkerGenerator;

    impl WorkerGenerator {
        pub fn start_group_workers(&self, group: &WorkerGroup) -> Result<Vec<Pid>, &str> {
            let mut remains = group.count;
            let threshold = 5;
            let mut pids = vec![];
            for _ in 0..threshold {
                for _ in 0..remains {
                    if let Ok(pid) = self.fork_child(group) {
                        pids.push(pid);
                        remains -= 1;
                    }
                }
                if remains == 0 {
                    return Ok(pids);
                }
            }
            return Err("Failed run workers");
        }

        pub fn fork_child(&self, group: &WorkerGroup) -> Result<Pid, Errno> {
            if group.worker.is_none() {
                return Err(Errno::EINVAL);
            }

            return match unsafe { fork() } {
                Ok(ForkResult::Parent { child }) => Ok(child),
                Ok(ForkResult::Child) => {
                    if let Some(w) = &group.worker {
                        w.run();
                    }
                    exit(0);
                }
                Err(err) => Err(err),
            };
        }
    }

    impl worker::WorkerCleaner {
        pub fn wait(&self) -> Result<Pid, worker::WaitError> {
            let wait_result = wait();
            return match wait_result {
                Ok(WaitStatus::Exited(pid, excode)) => {
                    if excode == 0 {
                        Ok(pid)
                    } else {
                        Err(worker::WaitError::ErrorExit(pid, excode))
                    }
                }
                Ok(ws) => Err(worker::WaitError::NotExited(ws)),
                Err(e) => Err(worker::WaitError::WaitFailed(e)),
            };
        }
    }
}

pub struct WorkerManager<'a> {
    groups: Vec<&'a WorkerGroup>,
    cleaner: WorkerCleaner,
    generator: helper::WorkerGenerator,
}

impl<'a> WorkerManager<'a> {
    pub fn new(groups: Vec<&'a WorkerGroup>, cleaner: Option<WorkerCleaner>) -> Self {
        return Self {
            groups: groups,
            cleaner: if let Some(c) = cleaner {
                c
            } else {
                WorkerCleaner {}
            },
            generator: helper::WorkerGenerator,
        };
    }

    pub fn start(&mut self) -> Vec<(&'a WorkerGroup, Vec<Pid>)> {
        let mut vec = vec![];
        for g in &self.groups {
            let start_result = self.generator.start_group_workers(&g);
            match start_result {
                Err(err) => {
                    log::error!("start failed: {err}");
                }
                Ok(pids) => vec.push((*g, pids)),
            }
        }

        return vec;
    }

    fn collect_and_fork(
        &self,
        group: &WorkerGroup,
        pids: &mut Vec<Pid>,
        pid: Pid,
    ) -> Result<Option<Pid>, Error> {
        let idx: Option<usize> = pids.iter().position(|p| p.as_raw() == pid.as_raw());

        if idx.is_none() {
            return Ok(None);
        }

        pids.remove(idx.unwrap());
        return self.generator.fork_child(group).map(|p: Pid| Some(p));
    }

    pub fn run(&self, vec: &mut Vec<(&'a WorkerGroup, Vec<Pid>)>) {
        loop {
            match self.cleaner.wait() {
                Ok(pid) => {
                    for (g, pids) in &mut *vec {
                        match self.collect_and_fork(g, pids, pid) {
                            Ok(Some(pid)) => pids.push(pid),
                            Ok(None) => {
                                log::trace!(target:"WorkerManager.run", "pid[{pid}] is not in group")
                            }
                            Err(err) => {
                                log::error!(target: "WorkerManager.run", "fork failed {err}")
                            }
                        }
                    }
                }
                Err(WaitError::WaitFailed(e)) => {
                    if e != Errno::ECHILD {
                        log::error!(target: "WorkerManager.run", "wait failed {e}");
                    }
                }
                Err(e) => {
                    log::error!(target: "WorkerManager.run", "wait error {e}");
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
            signal::{SigEvent, SigevNotify, Signal},
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
        let mut group_vec = manager.start();
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
        manager.run(&mut group_vec);
    }
}
