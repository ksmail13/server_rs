use nix::{
    Error,
    errno::Errno,
    sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
    unistd::Pid,
};

use crate::worker::{
    error::WaitError,
    group::WorkerGroup,
    helper::{WorkerCleaner, WorkerGenerator},
};

static mut RUNNING: bool = true;

extern "C" fn sigint_handler(_: i32) {
    unsafe { RUNNING = false };
}

pub struct WorkerManager {
    groups: Vec<WorkerGroup>,
    cleaner: WorkerCleaner,
    generator: WorkerGenerator,
}

impl WorkerManager {
    pub fn new(groups: Vec<WorkerGroup>) -> Self {
        return Self {
            groups: groups,
            cleaner: WorkerCleaner,
            generator: WorkerGenerator,
        };
    }

    pub fn start(&self) -> Vec<(&WorkerGroup, Vec<Pid>)> {
        let mut vec = vec![];
        for g in &self.groups {
            let start_result = self.generator.start_group_workers(&g);
            match start_result {
                Err(err) => {
                    log::error!("start failed: {err}");
                }
                Ok(pids) => vec.push((g, pids)),
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

    pub fn run(&self, vec: &mut Vec<(&WorkerGroup, Vec<Pid>)>) {
        let sigaction = unsafe {
            let mut sigset = SigSet::empty();
            sigset.add(Signal::SIGINT);
            sigaction(
                Signal::SIGINT,
                &SigAction::new(
                    SigHandler::Handler(sigint_handler),
                    SaFlags::SA_ONSTACK,
                    sigset,
                ),
            )
        };

        if let Err(e) = sigaction {
            log::error!(target: "WorkerManager.run", "sigaction failed: {e}");
        }

        while unsafe { RUNNING } {
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
                Err(WaitError::WaitFailed(e)) => match e {
                    Errno::EINTR => log::trace!("process over"),
                    Errno::ECHILD => (),
                    _ => log::error!(target: "WorkerManager.run", "wait failed {e}"),
                },
                Err(e) => {
                    log::error!(target: "WorkerManager.run", "wait error {e}");
                    return;
                }
            }
        }

        log::trace!("loop out");

        for (_, pids) in vec {
            let pid_len = pids.len();
            for pid in pids {
                let result = self.cleaner.kill(*pid);
                if result.is_err() {
                    log::trace!("kill child[{pid}] failed")
                }
            }

            for _ in 0..pid_len {
                match self.cleaner.wait() {
                    Ok(_) => (),
                    Err(e) => log::error!("wait failed {e}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{rc::Rc, time::Duration};

    use nix::{
        sys::{
            signal::{SigEvent, SigevNotify, Signal},
            timer::{Expiration::OneShot, Timer, TimerSetTimeFlags},
        },
        time::ClockId,
        unistd::getpid,
    };

    use crate::worker::Worker;

    use super::*;

    struct SleepWorker {}

    impl Worker for SleepWorker {
        fn run(&self) {
            let i: u32 = (0..10000).sum();
            println!("{}", i);
        }

        fn init(&self) {}

        fn cleanup(&self) {}
    }

    #[test]
    fn test_manager() {
        colog::basic_builder()
            .format_file(true)
            .format_line_number(true)
            .format_target(true)
            .format_timestamp_millis()
            .filter_level(log::LevelFilter::Trace)
            .init();
        let group = WorkerGroup::new(1, Rc::new(SleepWorker {}));
        let manager = WorkerManager::new(vec![group]);
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
            OneShot(Duration::from_millis(500).into()),
            TimerSetTimeFlags::empty(),
        );

        if let Err(err) = res {
            log::error!(target: "test_manager", "set timer failed {err}");
        }
        manager.run(&mut group_vec);
    }
}
