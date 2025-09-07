use std::process::exit;

use nix::{
    errno::Errno,
    sys::{
        signal::{Signal, kill},
        wait::{WaitStatus, wait},
    },
    unistd::{ForkResult, Pid, fork},
};

use crate::worker::{
    group::WorkerGroup,
    worker::{WaitError, WorkerCleaner},
};

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
        return match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => Ok(child),
            Ok(ForkResult::Child) => {
                group.worker.run();
                exit(0);
            }
            Err(err) => Err(err),
        };
    }
}

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

    pub fn kill(&self, pid: Pid) -> Result<Pid, Errno> {
        return kill(pid, Signal::SIGINT).map(|_| pid);
    }
}
