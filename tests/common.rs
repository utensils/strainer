use anyhow::Result;
use std::env;
use std::path::PathBuf;

pub struct EnvGuard {
    vars: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    #[must_use]
    pub fn new(vars: Vec<&'static str>) -> Self {
        let vars = vars
            .into_iter()
            .map(|var| (var, env::var(var).ok()))
            .collect();
        Self { vars }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Restore original environment state
        for (var, original_value) in &self.vars {
            match original_value {
                Some(value) => env::set_var(var, value),
                None => env::remove_var(var),
            }
        }
    }
}

pub struct DirGuard {
    original_dir: PathBuf,
}

impl DirGuard {
    /// Creates a new `DirGuard` that will restore the current directory when dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined.
    pub fn new() -> Result<Self> {
        let original_dir = env::current_dir()?;
        Ok(Self { original_dir })
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        // Only try to restore the directory if it still exists
        if self.original_dir.exists() {
            if let Err(e) = env::set_current_dir(&self.original_dir) {
                eprintln!("Error restoring original directory: {e}");
            }
        }
    }
}
