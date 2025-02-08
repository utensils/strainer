use anyhow::Result;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::Command;

#[allow(dead_code)]
pub struct ProcessController {
    pid: Pid,
}

#[allow(dead_code)]
impl ProcessController {
    pub const fn new(pid: i32) -> Self {
        Self {
            pid: Pid::from_raw(pid),
        }
    }

    pub fn from_command(command: &[String]) -> Result<(Self, std::process::Child)> {
        let child = Command::new(&command[0])
            .args(&command[1..])
            .spawn()?;
        
        // SAFETY: Process IDs on Unix systems are always positive and within i32 range
        // If this assumption is violated, we want to panic as it indicates a serious system issue
        #[allow(clippy::cast_possible_wrap)]
        let pid = child.id() as i32;
        
        Ok((Self::new(pid), child))
    }

    pub fn pause(&self) -> Result<()> {
        signal::kill(self.pid, Some(Signal::SIGSTOP))?;
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        signal::kill(self.pid, Some(Signal::SIGCONT))?;
        Ok(())
    }

    // TODO: Implement process monitoring
    pub fn is_running(&self) -> bool {
        signal::kill(self.pid, None).is_ok()
    }
}
