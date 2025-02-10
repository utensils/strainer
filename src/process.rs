use anyhow::Result;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::Child;
use std::process::Command;

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

    pub fn from_command(command: &[String]) -> Result<(Self, Child)> {
        let child = Command::new(&command[0]).args(&command[1..]).spawn()?;

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

    pub fn is_running(&self) -> bool {
        signal::kill(self.pid, None).is_ok()
    }

    pub fn terminate(&self) -> Result<()> {
        signal::kill(self.pid, Some(Signal::SIGKILL))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn spawn_test_process() -> Vec<String> {
        // Use sleep command as a test process that we can safely control
        vec!["sleep".to_string(), "10".to_string()]
    }

    #[test]
    fn test_process_creation() {
        let command = spawn_test_process();
        let (controller, mut child) = ProcessController::from_command(&command).unwrap();
        assert!(controller.is_running());
        controller.terminate().unwrap();
        let _ = child.wait();
        thread::sleep(Duration::from_millis(100)); // Give OS time to clean up
        assert!(!controller.is_running());
    }

    #[test]
    fn test_process_pause_resume() {
        let command = spawn_test_process();
        let (controller, mut child) = ProcessController::from_command(&command).unwrap();

        // Test pause
        assert!(controller.pause().is_ok());
        assert!(controller.is_running()); // Process should still be running, just paused

        // Test resume
        assert!(controller.resume().is_ok());
        assert!(controller.is_running());

        // Cleanup
        controller.terminate().unwrap();
        let _ = child.wait();
        thread::sleep(Duration::from_millis(100));
        assert!(!controller.is_running());
    }

    #[test]
    fn test_process_status() {
        let command = spawn_test_process();
        let (controller, mut child) = ProcessController::from_command(&command).unwrap();

        // Test running status
        assert!(controller.is_running());

        // Kill process and verify status
        controller.terminate().unwrap();
        let _ = child.wait();
        thread::sleep(Duration::from_millis(100));
        assert!(!controller.is_running());
    }

    #[test]
    fn test_invalid_process() {
        // Using a very large PID that's unlikely to exist
        let controller = ProcessController::new(999_999);
        assert!(!controller.is_running());
        assert!(controller.pause().is_err());
        assert!(controller.resume().is_err());
    }

    #[test]
    fn test_invalid_command() {
        let command = vec!["nonexistent_command".to_string()];
        assert!(ProcessController::from_command(&command).is_err());
    }
}
